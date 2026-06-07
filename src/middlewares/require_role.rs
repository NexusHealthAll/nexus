//! Role-based authorization middleware.
//!
//! Wrap an Axum route with `axum::middleware::from_fn(require_role(&[...]))` to
//! restrict it to a set of `UserRole` values derived from the caller's JWT.
//! The middleware reads the `Authorization: Bearer <token>` header via
//! [`extract_claims`](crate::utils::extract_claims), then either lets the
//! request through or returns 401 (missing/invalid token) or 403 (wrong role).

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::models::user::UserRole;
use crate::utils::extract_claims;

/// Build an Axum middleware that admits only callers whose JWT `role` claim is
/// in `allowed`. Returns 401 if the token is missing/invalid, 403 if the role
/// doesn't match.
///
/// Usage in `app_routes.rs`:
/// ```ignore
/// use axum::middleware::from_fn;
/// use crate::middlewares::require_role;
/// use crate::models::user::UserRole;
///
/// .route("/api/v1/shifts", post(shifts::create_shift))
///     .route_layer(from_fn(require_role(&[UserRole::HospitalAdmin])));
/// ```
pub fn require_role(
    allowed: &'static [UserRole],
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send
       + Sync
       + 'static {
    move |req: Request, next: Next| {
        Box::pin(async move {
            let claims = match extract_claims(req.headers()) {
                Ok(c) => c,
                Err(_) => return reject(StatusCode::UNAUTHORIZED, "Missing or invalid token"),
            };
            if !allowed.iter().any(|r| r == &claims.role) {
                return reject(StatusCode::FORBIDDEN, "Insufficient role for this endpoint");
            }
            next.run(req).await
        })
    }
}

fn reject(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}
