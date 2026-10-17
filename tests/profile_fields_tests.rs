//! Integration tests for the hospital and worker profile fields: a hospital's
//! type, staff size, operating hours and departments, a clinician's years of
//! experience and languages, and the signed-in password change.
//!
//! Requires a reachable Postgres — set TEST_DATABASE_URL (falls back to a local
//! `nexuscare_test` database). Skips (prints a notice and returns early) rather
//! than failing the suite if no database is reachable, matching
//! `patient_records_tests.rs`.
//!
//! The tenancy tests here are the point of the file. `PATCH
//! /api/v1/hospitals/{id}` previously had no authorization at all — no route
//! guard, no `extract_claims` — so the first two hospital tests are a
//! regression for a hole that was open, not a hypothetical one. Each negative
//! case re-reads the row with direct SQL afterwards: an authorization test that
//! only checks the `Err` has not shown that nothing was written.
//!
//! Services are constructed directly over a real pool; the HTTP layer is not
//! exercised, as elsewhere in this directory.

use std::sync::Arc;

use nexuscare_backend::models::hospital::{
    HospitalType, OperatingHours, UpdateHospitalRequest,
};
use nexuscare_backend::models::user::{Claims, UserRole};
use chrono::{DateTime, Duration, Utc};
use nexuscare_backend::repositories::clinician::ClinicianRepository;
use nexuscare_backend::repositories::hospital::HospitalRepository;
use nexuscare_backend::repositories::notification::NotificationRepository;
use nexuscare_backend::repositories::shift::ShiftRepository;
use nexuscare_backend::repositories::wallet::WalletRepository;
use nexuscare_backend::repositories::EmailOutboxRepository;
use nexuscare_backend::services::fcm::FcmClient;
use nexuscare_backend::services::push_service::PushService;
use nexuscare_backend::services::safehaven::SafeHavenClient;
use nexuscare_backend::services::shift_service::ShiftService;
use nexuscare_backend::services::wallet_service::WalletService;
use nexuscare_backend::services::auth_service::{hash_password, AuthError, AuthService};
use nexuscare_backend::services::email_outbox_service::EmailOutboxService;
use nexuscare_backend::services::notification_service::NotificationService;
use nexuscare_backend::services::hospital_profile_service::{
    HospitalProfileError, HospitalProfileService,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ndii@localhost:5432/nexuscare_test".to_string());

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIPPED: no test database reachable at {url}: {e}");
            return None;
        }
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations against test database");

    Some(pool)
}

// Fixtures

async fn seed_hospital(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO hospitals (name, registration_number, email, address, phone_number)
        VALUES ($1, $2, $3, 'Test Address', '08000000000')
        RETURNING id
        "#,
    )
    .bind(format!("Profile Hospital {}", Uuid::new_v4()))
    .bind(format!("RC-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(format!("{}@example.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("failed to seed hospital")
}

async fn seed_user(pool: &PgPool, role: &str, hospital_id: Option<Uuid>) -> Uuid {
    seed_user_with_password(pool, role, hospital_id, "not-a-real-hash").await
}

async fn seed_user_with_password(
    pool: &PgPool,
    role: &str,
    hospital_id: Option<Uuid>,
    password_hash: &str,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, first_name, last_name, password_hash, role, hospital_id)
        VALUES ($1, 'Test', 'User', $4, $2::user_role, $3)
        RETURNING id
        "#,
    )
    .bind(format!("{}@example.test", Uuid::new_v4()))
    .bind(role)
    .bind(hospital_id)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .expect("failed to seed user")
}

async fn seed_clinician(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO clinicians (
            user_id, first_name, last_name, specialty, role_title,
            license_number, clinician_role
        )
        VALUES ($1, 'Amina', 'Bello', 'emergency_medicine', 'Emergency Doctor',
                $2, 'doctor')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(format!("MDCN-{}", &Uuid::new_v4().to_string()[..8]))
    .fetch_one(pool)
    .await
    .expect("failed to seed clinician")
}

fn admin_claims(user_id: Uuid, hospital_id: Uuid) -> Claims {
    Claims {
        sub: user_id.to_string(),
        email: "admin@example.test".to_string(),
        role: UserRole::HospitalAdmin,
        hospital_id: Some(hospital_id.to_string()),
        exp: 0,
        iat: 0,
    }
}

fn super_admin_claims() -> Claims {
    Claims {
        sub: Uuid::new_v4().to_string(),
        email: "ops@nexuscare.test".to_string(),
        role: UserRole::SuperAdmin,
        hospital_id: None,
        exp: 0,
        iat: 0,
    }
}

fn worker_claims(user_id: Uuid) -> Claims {
    Claims {
        sub: user_id.to_string(),
        email: "worker@example.test".to_string(),
        role: UserRole::HealthWorker,
        hospital_id: None,
        exp: 0,
        iat: 0,
    }
}

fn profile_service(pool: &PgPool) -> HospitalProfileService {
    HospitalProfileService::new(Arc::new(HospitalRepository::new(pool.clone())))
}

fn auth_service(pool: &PgPool) -> AuthService {
    let outbox = Arc::new(EmailOutboxService::new(
        Arc::new(EmailOutboxRepository::new(pool.clone())),
        Arc::new(NotificationService::new()),
    ));
    AuthService::new(pool.clone(), outbox)
}

fn weekday(day: &str, opens: &str, closes: &str) -> OperatingHours {
    OperatingHours {
        day: day.to_string(),
        opens_at: Some(opens.to_string()),
        closes_at: Some(closes.to_string()),
        closed: false,
    }
}

/// The stored profile columns, read straight from SQL so a negative test can
/// prove the row did not move.
async fn stored_profile(
    pool: &PgPool,
    hospital_id: Uuid,
) -> (Option<String>, Option<i32>, Option<serde_json::Value>) {
    sqlx::query_as("SELECT hospital_type, staff_size, operating_hours FROM hospitals WHERE id = $1")
        .bind(hospital_id)
        .fetch_one(pool)
        .await
        .expect("failed to read hospital profile")
}

// 1 — the hospital profile. The first two are the tenancy regression.

#[tokio::test]
async fn a_hospital_admin_can_save_the_whole_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let admin = seed_user(&pool, "hospital_admin", Some(hospital_id)).await;

    let request = UpdateHospitalRequest {
        hospital_type: Some(HospitalType::Teaching),
        staff_size: Some(420),
        operating_hours: Some(vec![
            weekday("monday", "08:00", "17:00"),
            OperatingHours {
                day: "sunday".to_string(),
                opens_at: None,
                closes_at: None,
                closed: true,
            },
        ]),
        departments: Some(vec![
            "Emergency".to_string(),
            "ICU".to_string(),
            "  ".to_string(),
        ]),
        ..Default::default()
    };

    let response = service
        .update_profile(&admin_claims(admin, hospital_id), hospital_id, request)
        .await
        .expect("the hospital's own admin should be able to save its profile");

    assert_eq!(response.staff_size, Some(420));
    assert!(matches!(response.hospital_type, Some(HospitalType::Teaching)));
    assert_eq!(
        response.departments,
        vec!["Emergency".to_string(), "ICU".to_string()],
        "the blank department should have been dropped, not stored"
    );
    let hours = response.operating_hours.expect("operating hours");
    assert_eq!(hours.len(), 2);
    assert!(hours[1].closed, "sunday should have been stored as closed");

    let (stored_type, stored_size, stored_hours) = stored_profile(&pool, hospital_id).await;
    assert_eq!(stored_type.as_deref(), Some("teaching"));
    assert_eq!(stored_size, Some(420));
    assert!(stored_hours.is_some(), "operating hours should be persisted");
}

#[tokio::test]
async fn a_foreign_hospital_admin_cannot_touch_another_hospitals_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let other_hospital = seed_hospital(&pool).await;
    let intruder = seed_user(&pool, "hospital_admin", Some(other_hospital)).await;

    let result = service
        .update_profile(
            &admin_claims(intruder, other_hospital),
            hospital_id,
            UpdateHospitalRequest {
                staff_size: Some(9_999),
                hospital_type: Some(HospitalType::Clinic),
                ..Default::default()
            },
        )
        .await;

    assert!(
        matches!(result, Err(HospitalProfileError::WrongHospital)),
        "got {result:?}"
    );

    let (stored_type, stored_size, _) = stored_profile(&pool, hospital_id).await;
    assert_eq!(stored_type, None, "nothing should have been written");
    assert_eq!(stored_size, None, "nothing should have been written");
}

#[tokio::test]
async fn a_health_worker_cannot_edit_a_hospital_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let worker_user = seed_user(&pool, "health_worker", None).await;

    let result = service
        .update_profile(
            &worker_claims(worker_user),
            hospital_id,
            UpdateHospitalRequest {
                staff_size: Some(1),
                ..Default::default()
            },
        )
        .await;

    assert!(
        matches!(result, Err(HospitalProfileError::RoleNotPermitted)),
        "got {result:?}"
    );

    let (_, stored_size, _) = stored_profile(&pool, hospital_id).await;
    assert_eq!(stored_size, None, "nothing should have been written");
}

#[tokio::test]
async fn an_omitted_field_leaves_the_stored_value_alone() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let admin = seed_user(&pool, "hospital_admin", Some(hospital_id)).await;
    let claims = admin_claims(admin, hospital_id);

    service
        .update_profile(
            &claims,
            hospital_id,
            UpdateHospitalRequest {
                hospital_type: Some(HospitalType::Specialist),
                staff_size: Some(80),
                departments: Some(vec!["Radiology".to_string()]),
                ..Default::default()
            },
        )
        .await
        .expect("first save");

    // A second PATCH that mentions only the phone number must not clear the
    // departments or the type — the bug a naive full-replacement write has.
    let response = service
        .update_profile(
            &claims,
            hospital_id,
            UpdateHospitalRequest {
                phone_number: Some("08099999999".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("second save");

    assert_eq!(response.phone_number, "08099999999");
    assert_eq!(response.staff_size, Some(80));
    assert!(matches!(
        response.hospital_type,
        Some(HospitalType::Specialist)
    ));
    assert_eq!(response.departments, vec!["Radiology".to_string()]);
}

#[tokio::test]
async fn an_empty_department_list_clears_them() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let admin = seed_user(&pool, "hospital_admin", Some(hospital_id)).await;
    let claims = admin_claims(admin, hospital_id);

    service
        .update_profile(
            &claims,
            hospital_id,
            UpdateHospitalRequest {
                departments: Some(vec!["Theatre".to_string()]),
                ..Default::default()
            },
        )
        .await
        .expect("first save");

    let response = service
        .update_profile(
            &claims,
            hospital_id,
            UpdateHospitalRequest {
                departments: Some(Vec::new()),
                ..Default::default()
            },
        )
        .await
        .expect("clearing save");

    assert!(response.departments.is_empty());

    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM hospital_departments WHERE hospital_id = $1")
            .bind(hospital_id)
            .fetch_one(&pool)
            .await
            .expect("count departments");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn malformed_operating_hours_are_rejected_before_anything_is_written() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let admin = seed_user(&pool, "hospital_admin", Some(hospital_id)).await;

    let result = service
        .update_profile(
            &admin_claims(admin, hospital_id),
            hospital_id,
            UpdateHospitalRequest {
                staff_size: Some(50),
                operating_hours: Some(vec![weekday("moonday", "08:00", "17:00")]),
                ..Default::default()
            },
        )
        .await;

    assert!(
        matches!(result, Err(HospitalProfileError::InvalidOperatingHours(_))),
        "got {result:?}"
    );

    let (_, stored_size, stored_hours) = stored_profile(&pool, hospital_id).await;
    assert_eq!(
        stored_size, None,
        "the staff size in the same rejected request must not have been written"
    );
    assert!(stored_hours.is_none());
}

#[tokio::test]
async fn the_profile_read_returns_what_was_saved() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let admin = seed_user(&pool, "hospital_admin", Some(hospital_id)).await;

    service
        .update_profile(
            &admin_claims(admin, hospital_id),
            hospital_id,
            UpdateHospitalRequest {
                hospital_type: Some(HospitalType::PrimaryHealthCenter),
                staff_size: Some(12),
                departments: Some(vec!["Antenatal".to_string(), "Pharmacy".to_string()]),
                operating_hours: Some(vec![weekday("tuesday", "09:00", "15:30")]),
                ..Default::default()
            },
        )
        .await
        .expect("save");

    let profile = service
        .get_profile(&admin_claims(admin, hospital_id), hospital_id)
        .await
        .expect("read back");

    assert!(matches!(
        profile.hospital_type,
        Some(HospitalType::PrimaryHealthCenter)
    ));
    assert_eq!(profile.staff_size, Some(12));
    assert_eq!(
        profile.departments,
        vec!["Antenatal".to_string(), "Pharmacy".to_string()],
        "departments must come back in insertion order"
    );
    let hours = profile.operating_hours.expect("operating hours");
    assert_eq!(hours[0].day, "tuesday");
    assert_eq!(hours[0].opens_at.as_deref(), Some("09:00"));
}

#[tokio::test]
async fn a_missing_hospital_is_not_found_rather_than_forbidden() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    // Platform staff, so authorization passes and the lookup is what fails —
    // a hospital admin would (correctly) get WrongHospital first.
    let result = service
        .get_profile(&super_admin_claims(), Uuid::new_v4())
        .await;
    assert!(
        matches!(result, Err(HospitalProfileError::NotFound)),
        "got {result:?}"
    );
}

/// The profile read is gated by the same rule as the write. `HospitalResponse`
/// carries `rejection_reason`, `approved_by` and `admin_user_id`, none of which
/// the public drill-down exposes, so an ungated read here would hand that over
/// to anyone holding a hospital UUID.
#[tokio::test]
async fn a_foreign_hospital_admin_cannot_read_another_hospitals_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let other_hospital = seed_hospital(&pool).await;
    let intruder = seed_user(&pool, "hospital_admin", Some(other_hospital)).await;

    let result = service
        .get_profile(&admin_claims(intruder, other_hospital), hospital_id)
        .await;
    assert!(
        matches!(result, Err(HospitalProfileError::WrongHospital)),
        "got {result:?}"
    );
}

#[tokio::test]
async fn a_health_worker_cannot_read_a_hospital_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;
    let worker_user = seed_user(&pool, "health_worker", None).await;

    let result = service
        .get_profile(&worker_claims(worker_user), hospital_id)
        .await;
    assert!(
        matches!(result, Err(HospitalProfileError::RoleNotPermitted)),
        "got {result:?}"
    );
}

#[tokio::test]
async fn platform_staff_can_read_any_hospital_profile() {
    let Some(pool) = test_pool().await else { return };
    let service = profile_service(&pool);

    let hospital_id = seed_hospital(&pool).await;

    let profile = service
        .get_profile(&super_admin_claims(), hospital_id)
        .await
        .expect("platform staff administer hospital records elsewhere too");
    assert_eq!(profile.id, hospital_id);
}

// 2 — the worker profile.

#[tokio::test]
async fn a_worker_can_store_experience_and_languages() {
    let Some(pool) = test_pool().await else { return };
    let repo = ClinicianRepository::new(pool.clone());

    let user_id = seed_user(&pool, "health_worker", None).await;
    let clinician_id = seed_clinician(&pool, user_id).await;

    let languages = repo
        .set_languages(
            clinician_id,
            &[
                "English".to_string(),
                "Yoruba".to_string(),
                "   ".to_string(),
            ],
        )
        .await
        .expect("set languages");
    assert_eq!(
        languages,
        vec!["English".to_string(), "Yoruba".to_string()],
        "blank entries should be dropped"
    );

    let years = repo
        .set_years_experience(clinician_id, 7)
        .await
        .expect("set years");
    assert_eq!(years, Some(7));

    let read_back = repo
        .get_years_experience(clinician_id)
        .await
        .expect("read years");
    assert_eq!(read_back, Some(7));
}

#[tokio::test]
async fn replacing_languages_removes_the_ones_left_out() {
    let Some(pool) = test_pool().await else { return };
    let repo = ClinicianRepository::new(pool.clone());

    let user_id = seed_user(&pool, "health_worker", None).await;
    let clinician_id = seed_clinician(&pool, user_id).await;

    repo.set_languages(
        clinician_id,
        &["English".to_string(), "Hausa".to_string(), "Igbo".to_string()],
    )
    .await
    .expect("first set");

    let languages = repo
        .set_languages(clinician_id, &["English".to_string()])
        .await
        .expect("second set");

    assert_eq!(languages, vec!["English".to_string()]);

    let stored: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM clinician_languages WHERE clinician_id = $1")
            .bind(clinician_id)
            .fetch_one(&pool)
            .await
            .expect("count languages");
    assert_eq!(stored, 1, "the dropped languages should be gone from the table");
}

#[tokio::test]
async fn a_duplicated_language_is_stored_once() {
    let Some(pool) = test_pool().await else { return };
    let repo = ClinicianRepository::new(pool.clone());

    let user_id = seed_user(&pool, "health_worker", None).await;
    let clinician_id = seed_clinician(&pool, user_id).await;

    let languages = repo
        .set_languages(
            clinician_id,
            &["English".to_string(), "English".to_string()],
        )
        .await
        .expect("set languages");

    assert_eq!(languages, vec!["English".to_string()]);
}

#[tokio::test]
async fn one_workers_languages_do_not_leak_into_anothers() {
    let Some(pool) = test_pool().await else { return };
    let repo = ClinicianRepository::new(pool.clone());

    let first = seed_clinician(&pool, seed_user(&pool, "health_worker", None).await).await;
    let second = seed_clinician(&pool, seed_user(&pool, "health_worker", None).await).await;

    repo.set_languages(first, &["Yoruba".to_string()])
        .await
        .expect("first worker");
    repo.set_years_experience(first, 11).await.expect("years");

    assert!(repo.list_languages(second).await.expect("second").is_empty());
    assert_eq!(repo.get_years_experience(second).await.expect("years"), None);
}

#[tokio::test]
async fn an_impossible_experience_is_refused_by_the_database() {
    let Some(pool) = test_pool().await else { return };
    let repo = ClinicianRepository::new(pool.clone());

    let user_id = seed_user(&pool, "health_worker", None).await;
    let clinician_id = seed_clinician(&pool, user_id).await;

    // The handler validates 0–70 before this point; the CHECK constraint from
    // `20240070` is the backstop for anything that reaches the repository by
    // another path.
    let result = repo.set_years_experience(clinician_id, 900).await;
    assert!(result.is_err(), "got {result:?}");
}

// 3 — the signed-in password change.

#[tokio::test]
async fn a_user_can_change_their_own_password() {
    let Some(pool) = test_pool().await else { return };
    let service = auth_service(&pool);

    let hash = hash_password("CurrentPass123!").expect("hash");
    let user_id = seed_user_with_password(&pool, "hospital_admin", None, &hash).await;

    service
        .change_password(user_id, "CurrentPass123!", "BrandNewPass456!")
        .await
        .expect("the current password was correct");

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("read hash");
    assert_ne!(stored, hash, "the stored hash should have been replaced");

    assert!(
        nexuscare_backend::services::auth_service::verify_password(
            "BrandNewPass456!",
            &stored
        )
        .expect("verify"),
        "the new password should verify against the stored hash"
    );
}

#[tokio::test]
async fn the_wrong_current_password_changes_nothing() {
    let Some(pool) = test_pool().await else { return };
    let service = auth_service(&pool);

    let hash = hash_password("CurrentPass123!").expect("hash");
    let user_id = seed_user_with_password(&pool, "health_worker", None, &hash).await;

    let result = service
        .change_password(user_id, "NotTheCurrentPassword", "BrandNewPass456!")
        .await;

    assert!(
        matches!(result, Err(AuthError::WrongCurrentPassword)),
        "got {result:?}"
    );

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("read hash");
    assert_eq!(stored, hash, "the password must not have changed");
}

#[tokio::test]
async fn reusing_the_current_password_is_refused() {
    let Some(pool) = test_pool().await else { return };
    let service = auth_service(&pool);

    let hash = hash_password("CurrentPass123!").expect("hash");
    let user_id = seed_user_with_password(&pool, "hospital_admin", None, &hash).await;

    let result = service
        .change_password(user_id, "CurrentPass123!", "CurrentPass123!")
        .await;

    assert!(
        matches!(result, Err(AuthError::PasswordUnchanged)),
        "got {result:?}"
    );
}

#[tokio::test]
async fn changing_the_password_revokes_every_refresh_token() {
    let Some(pool) = test_pool().await else { return };
    let service = auth_service(&pool);

    let hash = hash_password("CurrentPass123!").expect("hash");
    let user_id = seed_user_with_password(&pool, "hospital_admin", None, &hash).await;

    for _ in 0..2 {
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
             VALUES ($1, $2, NOW() + INTERVAL '7 days')",
        )
        .bind(user_id)
        .bind(Uuid::new_v4().to_string())
        .execute(&pool)
        .await
        .expect("seed refresh token");
    }

    service
        .change_password(user_id, "CurrentPass123!", "BrandNewPass456!")
        .await
        .expect("change");

    let live: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 AND revoked = FALSE",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count tokens");
    assert_eq!(live, 0, "a password change must end the other sessions");
}

#[tokio::test]
async fn a_deactivated_account_cannot_change_its_password() {
    let Some(pool) = test_pool().await else { return };
    let service = auth_service(&pool);

    let hash = hash_password("CurrentPass123!").expect("hash");
    let user_id = seed_user_with_password(&pool, "health_worker", None, &hash).await;
    sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("deactivate");

    let result = service
        .change_password(user_id, "CurrentPass123!", "BrandNewPass456!")
        .await;

    assert!(matches!(result, Err(AuthError::Deactivated)), "got {result:?}");
}

// 4 — the profile fields feeding hospital-side matching.
//
// `GET /api/v1/shifts/{shift_id}/interested` is where a hospital picks a worker
// for a shift, and the only place qualifications already drive that choice
// (`quals_match`). These cover the same treatment for experience and languages:
// both are returned on every candidate, and both filter the list.

fn shift_service(pool: &PgPool) -> ShiftService {
    let notification_service = Arc::new(NotificationService::new());
    let email_outbox = Arc::new(EmailOutboxService::new(
        Arc::new(EmailOutboxRepository::new(pool.clone())),
        notification_service.clone(),
    ));
    let wallet_service = Arc::new(WalletService::new(
        Arc::new(WalletRepository::new(pool.clone())),
        Arc::new(SafeHavenClient::from_env()),
        pool.clone(),
    ));
    let push = Arc::new(PushService::new(
        Arc::new(NotificationRepository::new(pool.clone())),
        Arc::new(FcmClient::from_env()),
    ));

    ShiftService::new(
        Arc::new(ShiftRepository::new(pool.clone())),
        pool.clone(),
        notification_service,
        email_outbox,
        wallet_service,
        push,
    )
}

async fn seed_shift(pool: &PgPool, hospital_id: Uuid, created_by: Uuid) -> Uuid {
    let start: DateTime<Utc> = Utc::now() + Duration::days(1);
    sqlx::query_scalar(
        r#"
        INSERT INTO shifts (
            hospital_id, role_category, role_title, shift_type, status,
            scheduled_start, duration_hours, scheduled_end,
            pay_type, rate_kobo_per_hour, grand_total_kobo, created_by
        )
        VALUES ($1, 'doctor', 'Emergency Doctor', 'in_person', 'open',
                $2, 4, $2 + INTERVAL '4 hours',
                'hourly_rate', 800000, 3200000, $3)
        RETURNING id
        "#,
    )
    .bind(hospital_id)
    .bind(start)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("failed to seed shift")
}

async fn express_interest(pool: &PgPool, shift_id: Uuid, clinician_id: Uuid) {
    sqlx::query("INSERT INTO shift_interests (shift_id, clinician_id) VALUES ($1, $2)")
        .bind(shift_id)
        .bind(clinician_id)
        .execute(pool)
        .await
        .expect("failed to express interest");
}

/// A shift with two interested workers: a 12-year Yoruba/English speaker and a
/// 2-year Hausa speaker who has stated no experience figure at all.
struct Candidates {
    shift_id: Uuid,
    admin_user_id: Uuid,
    senior: Uuid,
    junior: Uuid,
    unstated: Uuid,
}

async fn seed_candidates(pool: &PgPool) -> Candidates {
    let repo = ClinicianRepository::new(pool.clone());
    let hospital_id = seed_hospital(pool).await;
    let admin_user_id = seed_user(pool, "hospital_admin", Some(hospital_id)).await;
    let shift_id = seed_shift(pool, hospital_id, admin_user_id).await;

    let senior = seed_clinician(pool, seed_user(pool, "health_worker", None).await).await;
    repo.set_years_experience(senior, 12).await.expect("senior years");
    repo.set_languages(senior, &["Yoruba".to_string(), "English".to_string()])
        .await
        .expect("senior languages");

    let junior = seed_clinician(pool, seed_user(pool, "health_worker", None).await).await;
    repo.set_years_experience(junior, 2).await.expect("junior years");
    repo.set_languages(junior, &["Hausa".to_string()])
        .await
        .expect("junior languages");

    // States neither, to pin down how the filters treat an unknown.
    let unstated = seed_clinician(pool, seed_user(pool, "health_worker", None).await).await;

    for c in [senior, junior, unstated] {
        express_interest(pool, shift_id, c).await;
    }

    Candidates { shift_id, admin_user_id, senior, junior, unstated }
}

#[tokio::test]
async fn candidates_carry_their_experience_and_languages() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    let ranked = service
        .list_ranked_interested(c.shift_id, c.admin_user_id, None, None)
        .await
        .expect("the shift creator may list candidates");

    assert_eq!(ranked.len(), 3, "no filter was asked for, so nobody is dropped");

    let senior = ranked
        .iter()
        .find(|r| r.clinician_id == c.senior)
        .expect("the senior candidate");
    assert_eq!(senior.years_experience, Some(12));
    assert_eq!(
        senior.languages,
        vec!["Yoruba".to_string(), "English".to_string()],
        "languages come back in insertion order"
    );

    let unstated = ranked
        .iter()
        .find(|r| r.clinician_id == c.unstated)
        .expect("the candidate who stated nothing");
    assert_eq!(unstated.years_experience, None);
    assert!(unstated.languages.is_empty());
}

#[tokio::test]
async fn a_minimum_experience_filters_out_the_junior_and_the_unstated() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    let ranked = service
        .list_ranked_interested(c.shift_id, c.admin_user_id, Some(5), None)
        .await
        .expect("list");

    let ids: Vec<Uuid> = ranked.iter().map(|r| r.clinician_id).collect();
    assert_eq!(
        ids,
        vec![c.senior],
        "only the 12-year worker clears a minimum of 5; the 2-year worker and \
         the one who stated nothing are both out"
    );
}

#[tokio::test]
async fn a_language_filter_is_case_insensitive() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    let ranked = service
        .list_ranked_interested(
            c.shift_id,
            c.admin_user_id,
            None,
            Some(&["hausa".to_string()]),
        )
        .await
        .expect("list");

    let ids: Vec<Uuid> = ranked.iter().map(|r| r.clinician_id).collect();
    assert_eq!(ids, vec![c.junior], "'hausa' should match a stored 'Hausa'");
}

#[tokio::test]
async fn every_requested_language_must_be_spoken() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    // The senior speaks both, so asking for both keeps them.
    let both = service
        .list_ranked_interested(
            c.shift_id,
            c.admin_user_id,
            None,
            Some(&["yoruba".to_string(), "english".to_string()]),
        )
        .await
        .expect("list");
    assert_eq!(
        both.iter().map(|r| r.clinician_id).collect::<Vec<_>>(),
        vec![c.senior]
    );

    // Nobody speaks Yoruba *and* Hausa, so the result is empty rather than the
    // union it would be under ANY semantics.
    let neither = service
        .list_ranked_interested(
            c.shift_id,
            c.admin_user_id,
            None,
            Some(&["yoruba".to_string(), "hausa".to_string()]),
        )
        .await
        .expect("list");
    assert!(
        neither.is_empty(),
        "all requested languages must be spoken, got {:?}",
        neither.iter().map(|r| r.clinician_id).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn the_two_filters_combine() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    // The junior speaks Hausa but has only 2 years, so the pair excludes them.
    let ranked = service
        .list_ranked_interested(
            c.shift_id,
            c.admin_user_id,
            Some(5),
            Some(&["hausa".to_string()]),
        )
        .await
        .expect("list");
    assert!(ranked.is_empty(), "the filters must apply together, not either/or");
}

#[tokio::test]
async fn filtering_candidates_is_still_restricted_to_the_shift_creator() {
    let Some(pool) = test_pool().await else { return };
    let service = shift_service(&pool);
    let c = seed_candidates(&pool).await;

    // A filter must not become a way around the creator check.
    let intruder = seed_user(&pool, "hospital_admin", None).await;
    let result = service
        .list_ranked_interested(c.shift_id, intruder, Some(1), Some(&["yoruba".to_string()]))
        .await;
    assert!(result.is_err(), "only the shift creator may list candidates");
}
