use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::admin_registration::NewHospital;
use crate::models::hospital::Hospital;
use crate::models::registration::RegistrationStatus;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Hospital not found: {0}")]
    NotFound(Uuid),

    #[error("Duplicate email: {0}")]
    DuplicateEmail(String),
}

/// Repository for hospital data persistence operations
pub struct HospitalRepository {
    pool: PgPool,
}

impl HospitalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new hospital record within a transaction
    pub async fn create(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hospital: NewHospital,
    ) -> Result<Hospital, RepositoryError> {
        let result = sqlx::query_as::<_, Hospital>(
            r#"
            INSERT INTO hospitals (
                name,
                registration_number,
                email,
                address,
                phone_number,
                admin_user_id,
                admin_first_name,
                admin_last_name,
                admin_registration_status,
                verification_status,
                registration_step
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', 'pending', 'profile_setup')
            RETURNING
                id, name, registration_number, email, address, phone_number,
                verification_status, registration_step,
                admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                legal_submitted_at, setup_progress_percent, logo_url, created_at, updated_at
            "#,
        )
        .bind(&hospital.name)
        .bind(&hospital.registration_number)
        .bind(&hospital.email)
        .bind(&hospital.address)
        .bind(&hospital.phone)
        .bind(hospital.admin_user_id)
        .bind(&hospital.admin_first_name)
        .bind(&hospital.admin_last_name)
        .fetch_one(&mut **tx)
        .await;

        match result {
            Ok(hospital) => Ok(hospital),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(RepositoryError::DuplicateEmail(hospital.email))
            }
            Err(e) => Err(RepositoryError::DatabaseError(e)),
        }
    }

    /// Find hospital by ID
    pub async fn find_by_id(&self, hospital_id: Uuid) -> Result<Option<Hospital>, RepositoryError> {
        let hospital = sqlx::query_as::<_, Hospital>(
            r#"
            SELECT 
                id, name, registration_number, email, address, phone_number,
                verification_status, registration_step,
                admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                legal_submitted_at, setup_progress_percent, logo_url,
                hospital_type, staff_size, operating_hours, created_at, updated_at
            FROM hospitals
            WHERE id = $1
            "#,
        )
        .bind(hospital_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(hospital)
    }

    /// Find hospital by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<Hospital>, RepositoryError> {
        let hospital = sqlx::query_as::<_, Hospital>(
            r#"
            SELECT 
                id, name, registration_number, email, address, phone_number,
                verification_status, registration_step,
                admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                legal_submitted_at, setup_progress_percent, logo_url, created_at, updated_at
            FROM hospitals
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(hospital)
    }

    /// Update hospital registration status
    pub async fn update_status(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        hospital_id: Uuid,
        status: RegistrationStatus,
        admin_id: Option<Uuid>,
        rejection_reason: Option<String>,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE hospitals
            SET 
                admin_registration_status = $2,
                approved_at = CASE WHEN $2 = 'approved' THEN NOW() ELSE approved_at END,
                approved_by = $3,
                rejection_reason = $4,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(hospital_id)
        .bind(status)
        .bind(admin_id)
        .bind(rejection_reason)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(hospital_id));
        }

        Ok(())
    }

    /// List all pending hospital registrations
    pub async fn list_pending(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Hospital>, RepositoryError> {
        let hospitals = sqlx::query_as::<_, Hospital>(
            r#"
            SELECT 
                id, name, registration_number, email, address, phone_number,
                verification_status, registration_step,
                admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                legal_submitted_at, setup_progress_percent, logo_url, created_at, updated_at
            FROM hospitals
            WHERE admin_registration_status = 'pending'
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(hospitals)
    }

    /// List all hospitals with optional status filter
    pub async fn list_all(
        &self,
        status_filter: Option<RegistrationStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Hospital>, RepositoryError> {
        let hospitals = if let Some(status) = status_filter {
            sqlx::query_as::<_, Hospital>(
                r#"
                SELECT 
                    id, name, registration_number, email, address, phone_number,
                    verification_status, registration_step,
                    admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                    legal_submitted_at, setup_progress_percent, logo_url, created_at, updated_at
                FROM hospitals
                WHERE admin_registration_status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Hospital>(
                r#"
                SELECT 
                    id, name, registration_number, email, address, phone_number,
                    verification_status, registration_step,
                    admin_registration_status, approved_by, approved_at, rejection_reason, admin_user_id,
                    legal_submitted_at, setup_progress_percent, logo_url, created_at, updated_at
                FROM hospitals
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(hospitals)
    }

    /// Count total hospitals with optional status filter
    pub async fn count_all(
        &self,
        status_filter: Option<RegistrationStatus>,
    ) -> Result<i64, RepositoryError> {
        let count = if let Some(status) = status_filter {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM hospitals
                WHERE admin_registration_status = $1
                "#,
            )
            .bind(status)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM hospitals
                "#,
            )
            .fetch_one(&self.pool)
            .await?
        };

        Ok(count)
    }

    /// Columns every profile read returns. Hoisted so the UPDATE's RETURNING
    /// and `find_by_id` cannot drift apart — a column listed in one and not
    /// the other decodes as `None` and looks like a lost write.
    const PROFILE_COLUMNS: &'static str = "id, name, registration_number, email, address, \
         phone_number, verification_status, registration_step, admin_registration_status, \
         approved_by, approved_at, rejection_reason, admin_user_id, legal_submitted_at, \
         setup_progress_percent, logo_url, hospital_type, staff_size, operating_hours, \
         created_at, updated_at";

    /// Apply a partial profile update. Every parameter is `Option`, and
    /// `COALESCE($n, column)` leaves an omitted field untouched — so the
    /// settings screen can PATCH one field without having to send the rest
    /// back, which is what makes a concurrent edit of a *different* field
    /// safe. Returns `None` when no such hospital exists, so the caller can
    /// 404 without a separate existence check.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_profile(
        &self,
        hospital_id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
        address: Option<&str>,
        phone_number: Option<&str>,
        logo_url: Option<&str>,
        hospital_type: Option<&str>,
        staff_size: Option<i32>,
        operating_hours: Option<&serde_json::Value>,
    ) -> Result<Option<Hospital>, RepositoryError> {
        let sql = format!(
            r#"
            UPDATE hospitals
               SET name            = COALESCE($2, name),
                   email           = COALESCE($3, email),
                   address         = COALESCE($4, address),
                   phone_number    = COALESCE($5, phone_number),
                   logo_url        = COALESCE($6, logo_url),
                   hospital_type   = COALESCE($7, hospital_type),
                   staff_size      = COALESCE($8, staff_size),
                   operating_hours = COALESCE($9, operating_hours),
                   updated_at      = NOW()
             WHERE id = $1
            RETURNING {}
            "#,
            Self::PROFILE_COLUMNS
        );

        let hospital = sqlx::query_as::<_, Hospital>(&sql)
            .bind(hospital_id)
            .bind(name)
            .bind(email)
            .bind(address)
            .bind(phone_number)
            .bind(logo_url)
            .bind(hospital_type)
            .bind(staff_size)
            .bind(operating_hours)
            .fetch_optional(&self.pool)
            .await?;

        Ok(hospital)
    }

    /// A hospital's departments, in the order they were added.
    pub async fn list_departments(
        &self,
        hospital_id: Uuid,
    ) -> Result<Vec<String>, RepositoryError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT department FROM hospital_departments \
             WHERE hospital_id = $1 ORDER BY created_at ASC",
        )
        .bind(hospital_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Replace the department set wholesale, in one transaction. Modelled on
    /// `ClinicianRepository::set_qualifications` — same shape of data, same
    /// full-replacement contract, so the two should not behave differently.
    /// Blank entries are dropped rather than rejected: a trailing empty row in
    /// the settings form is a UI artefact, not a request to fail the save.
    pub async fn set_departments(
        &self,
        hospital_id: Uuid,
        departments: &[String],
    ) -> Result<Vec<String>, RepositoryError> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        sqlx::query("DELETE FROM hospital_departments WHERE hospital_id = $1")
            .bind(hospital_id)
            .execute(&mut *tx)
            .await?;
        for d in departments {
            let d = d.trim();
            if d.is_empty() {
                continue;
            }
            sqlx::query(
                "INSERT INTO hospital_departments (hospital_id, department) \
                 VALUES ($1, $2) ON CONFLICT (hospital_id, department) DO NOTHING",
            )
            .bind(hospital_id)
            .bind(d)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.list_departments(hospital_id).await
    }
}
