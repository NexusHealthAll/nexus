use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;
use validator::Validate;

use crate::{
    routes::AppState,
    utils::errors::{AppError, AppResult},
    models::user::{Claims, CreateUserRequest, LoginRequest, User, UserResponse},
};

/// POST /api/v1/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<UserResponse>)> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    // Check for duplicate email
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1")
            .bind(&payload.email)
            .fetch_optional(&state.pool)
            .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email is already registered".to_string()));
    }

    // Hash password with Argon2
    let password_hash = hash_password(&payload.password)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let user: User = sqlx::query_as(
        r#"
        INSERT INTO users (id, hospital_id, first_name, last_name, email, password_hash, role)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, hospital_id, first_name, last_name, email, password_hash,
                  role, is_active, last_login_at, created_at, updated_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.hospital_id)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.email)
    .bind(&password_hash)
    .bind(&payload.role)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<serde_json::Value>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user: Option<User> = sqlx::query_as(
        r#"
        SELECT id, hospital_id, first_name, last_name, email, password_hash,
               role, is_active, last_login_at, created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await?;

    let user = user.ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    if !user.is_active {
        return Err(AppError::Forbidden("Account is deactivated".to_string()));
    }

    // Verify password
    let valid = verify_password(&payload.password, &user.password_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password verification failed: {}", e)))?;

    if !valid {
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    // Update last_login_at
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    // Issue JWT
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    let expiry_hours: u64 = std::env::var("JWT_EXPIRY_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);

    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.clone(),
        hospital_id: user.hospital_id.map(|id: Uuid| id.to_string()),
        exp: now + (expiry_hours as usize * 3600),
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encoding failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": expiry_hours * 3600,
        "user": UserResponse::from(user)
    })))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
