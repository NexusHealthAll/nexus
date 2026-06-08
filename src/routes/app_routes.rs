use axum::{
    middleware::from_fn,
    routing::{delete, get, patch, post},
    Router,
};

use crate::middlewares::require_role;
use crate::models::user::UserRole;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    admin, auth, feedback, health, hospitals, ml_health, patients, pipeline, pipeline_gateway,
    registration, clinician_registration, shifts,
};
use crate::repositories::{
    audit::AuditRepository,
    billing::BillingRepository,
    clinician::ClinicianRepository,
    hospital::HospitalRepository,
    location::LocationRepository,
    shift::ShiftRepository,
    patient::PatientRepository,
    feedback::FeedbackRepository,
};
use crate::services::{
    audit_service::AuditService,
    auth_service::AuthService,
    encryption::EncryptionService,
    geocoding::GeocodingClient,
    location_service::LocationService,
    notification_service::NotificationService,
    email_outbox_service::EmailOutboxService,
    payment_service::PaymentService,
    paystack::PaystackClient,
    registration_service::RegistrationService,
    clinician_registration_service::ClinicianRegistrationService,
    shift_service::ShiftService,
    bronze_service::BronzeService,
    silver_service::SilverService,
    gold_service::GoldService,
    ml_service::MlService,
    pipeline_service::PipelineService,
    pipeline_event_service::PipelineEventService,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub registration_service: Arc<RegistrationService>,
    pub clinician_registration_service: Arc<ClinicianRegistrationService>,
    pub auth_service: Arc<AuthService>,
    pub shift_service: Arc<ShiftService>,
    pub clinician_repo: Arc<ClinicianRepository>,
    pub pipeline_service: Arc<PipelineService>,
    pub patient_repo: Arc<PatientRepository>,
    pub feedback_repo: Arc<FeedbackRepository>,
    pub ml_service: Arc<MlService>,
    pub event_service: Arc<PipelineEventService>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health_check,
        crate::handlers::health::db_health_check,
        crate::handlers::auth::register,
        crate::handlers::auth::login,
        crate::handlers::auth::email_otp_send,
        crate::handlers::auth::email_otp_verify,
        crate::handlers::auth::forgot_password,
        crate::handlers::auth::reset_password,
        crate::handlers::auth::refresh_token,
        crate::handlers::auth::logout,
        crate::handlers::registration::register_hospital,
        crate::handlers::registration::list_hospitals,
        crate::handlers::registration::get_registration_status,
        crate::handlers::registration::approve_hospital,
        crate::handlers::registration::reject_hospital,
        crate::handlers::clinician_registration::send_otp,
        crate::handlers::clinician_registration::verify_otp,
        crate::handlers::clinician_registration::complete_profile,
        crate::handlers::clinician_registration::add_bank_account,
        crate::handlers::shifts::create_shift,
        crate::handlers::shifts::list_shifts,
        crate::handlers::shifts::preview_shift,
        crate::handlers::shifts::get_shift,
        crate::handlers::shifts::express_interest,
        crate::handlers::shifts::apply_for_shift,
        crate::handlers::shifts::list_shift_applications,
        crate::handlers::shifts::list_interested_for_shift,
        crate::handlers::shifts::offer_shift,
        crate::handlers::shifts::accept_shift,
        crate::handlers::shifts::decline_shift,
        crate::handlers::shifts::clock_in,
        crate::handlers::shifts::submit_handover,
        crate::handlers::shifts::clock_out,
        crate::handlers::shifts::request_handover_revision,
        crate::handlers::shifts::rate_worker,
        crate::handlers::shifts::rate_hospital,
        crate::handlers::shifts::edit_rating,
        crate::handlers::shifts::list_nearby_shifts,
        crate::handlers::shifts::list_my_applications,
        crate::handlers::shifts::withdraw_interest,
        crate::handlers::shifts::bookmark_shift,
        crate::handlers::shifts::unbookmark_shift,
        crate::handlers::shifts::dismiss_shift,
        crate::handlers::shifts::request_clockin_approval,
        crate::handlers::shifts::approve_clockin_request,
        crate::handlers::shifts::deny_clockin_request,
        crate::handlers::shifts::assign_shift,
        crate::handlers::shifts::cancel_shift,
        crate::handlers::shifts::reschedule_shift,
        crate::handlers::admin::list_hospitals_admin,
        crate::handlers::admin::list_clinicians_admin,
    ),
    components(
        schemas(
            // Registration
            crate::handlers::registration::HospitalRegistrationResponse,
            crate::handlers::registration::StatusChangeResponse,
            crate::handlers::registration::ApprovalRequest,
            crate::handlers::registration::RejectionRequest,
            crate::handlers::registration::ErrorResponse,
            crate::handlers::registration::ListHospitalsQuery,
            // Shifts
            crate::handlers::shifts::ShiftPreviewResponse,
            crate::handlers::shifts::ErrorResponse,
            crate::handlers::shifts::ShiftListResponse,
            crate::handlers::shifts::ShiftApplicationsResponse,
            crate::handlers::shifts::PaginationMetadata,
            crate::models::shift::RankedInterestedClinician,
            crate::models::shift::ShiftOfferRequest,
            crate::models::shift::ShiftOfferResponse,
            crate::models::shift::NdprConsent,
            crate::models::shift::AcceptShiftRequest,
            crate::models::shift::DeclineShiftRequest,
            crate::models::shift::ClockinRequest,
            crate::models::shift::ClockinResponse,
            crate::models::shift::ClockinMethod,
            crate::models::shift::SubmitHandoverRequest,
            crate::models::shift::HandoverResponse,
            crate::models::shift::ClockoutResponse,
            crate::models::shift::HandoverRevisionRequest,
            crate::models::shift::HospitalRatingDimensions,
            crate::models::shift::RateWorkerRequest,
            crate::models::shift::RateHospitalRequest,
            crate::models::shift::EditRatingRequest,
            crate::models::shift::RatingResponse,
            crate::models::shift::NearbyShiftCard,
            crate::models::shift::MyApplicationEntry,
            crate::models::shift::ClockinApprovalRequest,
            crate::models::shift::ClockinApprovalDecisionRequest,
            crate::models::shift::ClockinApprovalRecord,
            // Admin
            crate::handlers::admin::ClinicianListResponse,
            crate::handlers::admin::PaginationMetadata,
            crate::handlers::admin::ListCliniciansQuery,
            // Models
            crate::models::admin_registration::HospitalRegistrationRequest,
            crate::models::admin_registration::Address,
            crate::models::admin_registration::PaymentDetails,
            crate::models::admin_registration::PaymentMethodType,
            crate::models::shift::Shift,
            crate::models::shift::CreateShiftRequest,
            crate::models::shift::ShiftStatus,
            crate::models::shift::ShiftPriority,
            crate::models::shift::ShiftType,
            crate::models::shift::RoleCategory,
            crate::models::shift::PayType,
            crate::models::shift::ShiftApplication,
            crate::models::shift::ShiftApplicationRequest,
            crate::models::shift::ShiftApplicationStatus,
            crate::models::shift::ShiftApplicationsQuery,
            crate::models::shift::ShiftListQuery,
            crate::models::shift::ShiftInterestRequest,
            crate::models::shift::ShiftAssignRequest,
            crate::models::shift::ShiftCancelRequest,
            crate::models::shift::ShiftRescheduleRequest,
            crate::models::user::UserResponse,
            crate::models::user::CreateUserRequest,
            crate::models::user::LoginRequest,
            crate::models::user::LoginResponse,
            crate::models::user::EmailLoginRequest,
            crate::models::user::EmailOtpVerifyRequest,
            crate::models::user::ForgotPasswordRequest,
            crate::models::user::ResetPasswordRequest,
            crate::models::user::RefreshTokenRequest,
            crate::models::user::LogoutRequest,
            crate::models::clinician_registration::SendOtpRequest,
            crate::models::clinician_registration::SendOtpResponse,
            crate::models::clinician_registration::VerifyOtpRequest,
            crate::models::clinician_registration::VerifyOtpResponse,
            crate::models::clinician_registration::CompleteProfileRequest,
            crate::models::clinician_registration::ProfileResponse,
            crate::models::clinician_registration::AddBankAccountRequest,
            crate::models::clinician_registration::BankAccountResponse,
            crate::models::clinician::ClinicianAdminSummary,
            // Services
            crate::services::registration_service::RegistrationStatusResponse,
            crate::services::registration_service::HospitalListResponse,
            crate::services::registration_service::HospitalSummary,
            crate::services::registration_service::PaginationMetadata,
        )
    ),
    info(
        title = "NexusCare Hospital Management API",
        version = "1.0.0",
        description = "Hospital management, ML pipeline, real-time SSE events",
        contact(name = "NexusCare Support", email = "support@nexuscare.com")
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development"),
        (url = "https://api.nexuscare.com", description = "Production")
    ),
    tags(
        (name = "health"),
        (name = "auth"),
        (name = "hospitals"),
        (name = "clinicians"),
        (name = "shifts"),
        (name = "admin"),
        (name = "pipeline", description = "ML data pipeline — ingest, events, patient dashboards"),
        (name = "feedback", description = "Doctor corrections and outcome feedback loop"),
    )
)]
struct ApiDoc;

pub fn create_router(
    pool: PgPool,
    notification_service: Arc<NotificationService>,
    email_outbox_service: Arc<EmailOutboxService>,
) -> (Router, AppState) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let hospital_repo = Arc::new(HospitalRepository::new(pool.clone()));
    let location_repo = Arc::new(LocationRepository::new(pool.clone()));
    let billing_repo = Arc::new(BillingRepository::new(pool.clone()));
    let audit_repo = Arc::new(AuditRepository::new(pool.clone()));
    let clinician_repo = Arc::new(ClinicianRepository::new(pool.clone()));
    let shift_repo = Arc::new(ShiftRepository::new(pool.clone()));
    let patient_repo = Arc::new(PatientRepository::new(pool.clone()));
    let feedback_repo = Arc::new(FeedbackRepository::new(pool.clone()));

    let geocoding_client = Arc::new(GeocodingClient::new(std::env::var("GEOCODING_API_URL").ok()));

    let paystack_client = Arc::new(PaystackClient::new(
        std::env::var("PAYSTACK_SECRET_KEY").unwrap_or_else(|_| "sk_test_dummy".to_string()),
        std::env::var("PAYSTACK_API_URL").ok(),
    ));

    let encryption_service = Arc::new({
        let key_hex = std::env::var("ENCRYPTION_KEY").unwrap_or_else(|_| "0".repeat(64));
        let key_bytes = hex::decode(&key_hex).unwrap_or_else(|_| vec![0u8; 32]);
        EncryptionService::new(key_bytes).expect("Failed to create encryption service")
    });

    let location_service = Arc::new(LocationService::new(geocoding_client, location_repo.clone()));
    let payment_service = Arc::new(PaymentService::new(paystack_client.clone(), billing_repo.clone(), encryption_service.clone()));
    let audit_service = Arc::new(AuditService::new(audit_repo));

    let registration_service = Arc::new(RegistrationService::new(
        hospital_repo, location_service, payment_service, audit_service,
        email_outbox_service.clone(), pool.clone(),
    ));

    let clinician_registration_service = Arc::new(ClinicianRegistrationService::new(
        clinician_repo.clone(), email_outbox_service.clone(),
        paystack_client.clone(), encryption_service.clone(), pool.clone(),
    ));

    let auth_service = Arc::new(AuthService::new(pool.clone(), email_outbox_service.clone()));

    let shift_service = Arc::new(ShiftService::new(
        shift_repo, pool.clone(), notification_service.clone(), email_outbox_service.clone(),
    ));

    // Pipeline services
    let event_service = Arc::new(PipelineEventService::new());
    let bronze = Arc::new(BronzeService::new(patient_repo.clone()));
    let silver = Arc::new(SilverService::new(patient_repo.clone()));
    let gold = Arc::new(GoldService::new(patient_repo.clone()));
    let ml_service = Arc::new(MlService::new());

    let pipeline_service = Arc::new(PipelineService::new(
        bronze, silver, gold,
        ml_service.clone(),
        event_service.clone(),
        patient_repo.clone(),
    ));

    let state = AppState {
        pool: pool.clone(),
        registration_service,
        clinician_registration_service,
        auth_service,
        shift_service,
        clinician_repo: clinician_repo.clone(),
        pipeline_service,
        patient_repo,
        feedback_repo,
        ml_service,
        event_service,
    };

    let api_router = Router::new()
        .route("/health", get(health::health_check))
        .route("/health/db", get(health::db_health_check))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/otp/send", post(auth::email_otp_send))
        .route("/api/v1/auth/otp/verify", post(auth::email_otp_verify))
        .route("/api/v1/auth/forgot-password", post(auth::forgot_password))
        .route("/api/v1/auth/reset-password", post(auth::reset_password))
        .route("/api/v1/auth/refresh", post(auth::refresh_token))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/hospitals/register", post(registration::register_hospital))
        .route("/api/v1/hospitals", get(registration::list_hospitals))
        .route("/api/v1/hospitals/{hospital_id}/status", get(registration::get_registration_status))
        .route("/api/v1/admin/hospitals/{hospital_id}/approve", post(registration::approve_hospital))
        .route("/api/v1/admin/hospitals/{hospital_id}/reject", post(registration::reject_hospital))
        .route("/api/v1/admin/hospitals", get(admin::list_hospitals_admin))
        .route("/api/v1/admin/clinicians", get(admin::list_clinicians_admin))
        .route("/api/v1/hospitals/create", post(hospitals::create_hospital))
        .route("/api/v1/hospitals/{id}", get(hospitals::get_hospital))
        .route("/api/v1/hospitals/{id}", patch(hospitals::update_hospital))
        .route(
            "/api/v1/hospitals/{id}/advance-step",
            patch(hospitals::advance_registration_step),
        )
        // Clinician registration
        .route(
            "/api/v1/clinicians/otp/send",
            post(clinician_registration::send_otp),
        )
        .route(
            "/api/v1/clinicians/otp/verify",
            post(clinician_registration::verify_otp),
        )
        .route(
            "/api/v1/clinicians/{clinician_id}/profile",
            axum::routing::put(clinician_registration::complete_profile),
        )
        .route(
            "/api/v1/clinicians/{clinician_id}/bank-account",
            post(clinician_registration::add_bank_account),
        )
        // Shifts — gated per FRS v2.0 §2.2 permission matrix.
        //
        // HospitalAdmin/SuperAdmin: create, list-all, preview, view applications,
        //                           assign, cancel, reschedule.
        // HealthWorker: express interest, apply.
        // Anyone authenticated: view a single shift's details (worker discovery).
        .route(
            "/api/v1/shifts",
            post(shifts::create_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts",
            get(shifts::list_shifts)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/preview",
            post(shifts::preview_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route("/api/v1/shifts/{shift_id}", get(shifts::get_shift))
        .route(
            "/api/v1/shifts/{shift_id}/interest",
            post(shifts::express_interest)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/apply",
            post(shifts::apply_for_shift)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/applications",
            get(shifts::list_shift_applications)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/interested",
            get(shifts::list_interested_for_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/offer",
            post(shifts::offer_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/accept",
            post(shifts::accept_shift)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/decline",
            post(shifts::decline_shift)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/clockin",
            post(shifts::clock_in)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/handover",
            post(shifts::submit_handover)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/clockout",
            post(shifts::clock_out)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/handover/revision",
            post(shifts::request_handover_revision)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/ratings/worker",
            post(shifts::rate_worker)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/ratings/hospital",
            post(shifts::rate_hospital)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/ratings/{rating_id}",
            patch(shifts::edit_rating),
        )
        .route(
            "/api/v1/worker/shifts/nearby",
            get(shifts::list_nearby_shifts)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/worker/shifts/my-applications",
            get(shifts::list_my_applications)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/interest",
            delete(shifts::withdraw_interest)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/bookmark",
            post(shifts::bookmark_shift)
                .delete(shifts::unbookmark_shift)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/dismiss",
            post(shifts::dismiss_shift)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/clockin/approval-request",
            post(shifts::request_clockin_approval)
                .route_layer(from_fn(require_role(&[UserRole::HealthWorker]))),
        )
        .route(
            "/api/v1/clockin-approvals/{request_id}/approve",
            post(shifts::approve_clockin_request)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/clockin-approvals/{request_id}/deny",
            post(shifts::deny_clockin_request)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/assign",
            post(shifts::assign_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/cancel",
            post(shifts::cancel_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .route(
            "/api/v1/shifts/{shift_id}/reschedule",
            post(shifts::reschedule_shift)
                .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin, UserRole::SuperAdmin]))),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    // Merge with Swagger UI
    let router = Router::new()
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
        .merge(api_router);

    (router, state)
}
