//! Editing a hospital's own profile — the write side of the settings screen.
//!
//! This service exists for one reason: `PATCH /api/v1/hospitals/{id}` had no
//! authorization of any kind. The route carried no `require_role` layer and the
//! handler never called `extract_claims`, so an anonymous caller who knew a
//! hospital's UUID could rename it, repoint its email and swap its logo. That
//! was survivable only because the body could change little; this change gives
//! the endpoint the hospital's type, size, opening times and departments, so
//! the hole is closed here before the fields are added rather than after.
//!
//! The tenant rule is the one used everywhere else in this codebase —
//! `claims.hospital_id == hospital_id` — and *not* `admin_user_id == claims.sub`,
//! which would 403 a hospital's second admin and is not a tenancy check at all
//! (see `VideoService::authorize_shift_data_access`). Platform staff are allowed
//! through because they already administer hospital records elsewhere
//! (`admin::suspend_hospital`); a clinician never is, whatever their claims say.
//!
//! Validation of `operating_hours` lives here rather than in a `validator`
//! derive because the rules are cross-field (a day may omit its times only when
//! it is closed) and cross-element (no day may appear twice) — neither of which
//! a per-field attribute can express. The DB cannot express them either: the
//! column is JSONB and a CHECK constraint would have to parse it.

use std::sync::Arc;

use uuid::Uuid;

use crate::models::hospital::{HospitalResponse, OperatingHours, UpdateHospitalRequest};
use crate::models::user::{Claims, UserRole};
use crate::repositories::hospital::{HospitalRepository, RepositoryError};

/// One variant per distinct failure. The two authorization failures are
/// deliberately separate: "you are not this hospital" and "your role never
/// edits a hospital" are different mistakes, and a 403 that cannot tell the
/// caller which one happened is the bug that makes support tickets unanswerable.
#[derive(Debug, thiserror::Error)]
pub enum HospitalProfileError {
    #[error("Hospital not found")]
    NotFound,
    #[error("Not your hospital")]
    WrongHospital,
    #[error("This role cannot edit a hospital profile")]
    RoleNotPermitted,
    #[error("Invalid operating hours: {0}")]
    InvalidOperatingHours(String),
    #[error("Database error: {0}")]
    Database(#[from] RepositoryError),
}

pub struct HospitalProfileService {
    hospital_repo: Arc<HospitalRepository>,
}

/// The seven day names accepted in `operating_hours`, lowercase. Fixed rather
/// than parsed with a date library because they are a wire contract with the
/// settings screen, not a locale-aware rendering.
const DAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

impl HospitalProfileService {
    pub fn new(hospital_repo: Arc<HospitalRepository>) -> Self {
        Self { hospital_repo }
    }

    /// Read a hospital's editable profile, including the departments that live
    /// in their own table.
    ///
    /// Behind the same `authorize` check as the write, and for the same reason:
    /// `HospitalResponse` carries back-office state that the curated public
    /// drill-down (`HospitalPublicDetail`) deliberately leaves out —
    /// `rejection_reason`, `approved_by`, `admin_user_id`,
    /// `admin_registration_status`, `legal_submitted_at`. Served ungated, a
    /// hospital UUID would be enough to read why a competitor was rejected and
    /// which user account administers it. A caller who only wants the public
    /// view already has `GET /api/v1/hospitals/{id}`.
    pub async fn get_profile(
        &self,
        claims: &Claims,
        hospital_id: Uuid,
    ) -> Result<HospitalResponse, HospitalProfileError> {
        self.authorize(claims, hospital_id)?;

        let hospital = self
            .hospital_repo
            .find_by_id(hospital_id)
            .await?
            .ok_or(HospitalProfileError::NotFound)?;

        let departments = self.hospital_repo.list_departments(hospital_id).await?;
        let mut response = HospitalResponse::from(hospital);
        response.departments = departments;
        Ok(response)
    }

    /// Apply a partial profile update on behalf of `claims`.
    ///
    /// Order matters: authorize, then validate, then write. Validating first
    /// would let an unauthorized caller use the 422/403 difference to probe
    /// which hospital ids exist.
    pub async fn update_profile(
        &self,
        claims: &Claims,
        hospital_id: Uuid,
        request: UpdateHospitalRequest,
    ) -> Result<HospitalResponse, HospitalProfileError> {
        self.authorize(claims, hospital_id)?;

        let operating_hours = match request.operating_hours.as_deref() {
            Some(hours) => {
                Self::validate_operating_hours(hours)?;
                Some(serde_json::to_value(hours).map_err(|e| {
                    HospitalProfileError::InvalidOperatingHours(e.to_string())
                })?)
            }
            None => None,
        };

        let hospital = self
            .hospital_repo
            .update_profile(
                hospital_id,
                request.name.as_deref(),
                request.email.as_deref(),
                request.address.as_deref(),
                request.phone_number.as_deref(),
                request.logo_url.as_deref(),
                request.hospital_type.map(hospital_type_as_str),
                request.staff_size,
                operating_hours.as_ref(),
            )
            .await?
            .ok_or(HospitalProfileError::NotFound)?;

        // Departments are a separate table, so an omitted list must mean "leave
        // them alone" — not "delete them all", which is what a naive
        // `set_departments(&[])` on every PATCH would do to a hospital that
        // only wanted to change its phone number.
        let departments = match request.departments {
            Some(list) => self.hospital_repo.set_departments(hospital_id, &list).await?,
            None => self.hospital_repo.list_departments(hospital_id).await?,
        };

        let mut response = HospitalResponse::from(hospital);
        response.departments = departments;
        Ok(response)
    }

    /// The tenant boundary for a hospital's own record. There is no RLS; this
    /// check is it.
    fn authorize(&self, claims: &Claims, hospital_id: Uuid) -> Result<(), HospitalProfileError> {
        match claims.role {
            UserRole::SuperAdmin | UserRole::OperationsAdmin => Ok(()),
            UserRole::HospitalAdmin => {
                let own = claims
                    .hospital_id
                    .as_deref()
                    .and_then(|id| Uuid::parse_str(id).ok());
                if own == Some(hospital_id) {
                    Ok(())
                } else {
                    Err(HospitalProfileError::WrongHospital)
                }
            }
            _ => Err(HospitalProfileError::RoleNotPermitted),
        }
    }

    /// Reject a week that cannot be displayed or honoured: an unknown day, the
    /// same day twice, a non-`HH:MM` time, missing times on an open day, or a
    /// close that is not after the open.
    ///
    /// An overnight shift (opens 20:00, closes 02:00) is rejected rather than
    /// silently stored, because nothing downstream knows to read it as
    /// crossing midnight. A 24-hour A&E is expressed as 00:00–23:59.
    fn validate_operating_hours(hours: &[OperatingHours]) -> Result<(), HospitalProfileError> {
        let invalid = HospitalProfileError::InvalidOperatingHours;
        let mut seen: Vec<String> = Vec::with_capacity(hours.len());

        for entry in hours {
            let day = entry.day.trim().to_lowercase();
            if !DAYS.contains(&day.as_str()) {
                return Err(invalid(format!(
                    "'{}' is not a day name; expected one of {}",
                    entry.day,
                    DAYS.join(", ")
                )));
            }
            if seen.contains(&day) {
                return Err(invalid(format!("{day} appears more than once")));
            }
            seen.push(day.clone());

            if entry.closed {
                continue;
            }

            let (Some(opens), Some(closes)) = (entry.opens_at.as_deref(), entry.closes_at.as_deref())
            else {
                return Err(invalid(format!(
                    "{day} is open, so it needs both opens_at and closes_at"
                )));
            };

            let opens = parse_hhmm(opens)
                .ok_or_else(|| invalid(format!("{day}: '{opens}' is not an HH:MM time")))?;
            let closes = parse_hhmm(closes)
                .ok_or_else(|| invalid(format!("{day}: '{closes}' is not an HH:MM time")))?;

            if closes <= opens {
                return Err(invalid(format!(
                    "{day}: closes_at must be later in the day than opens_at"
                )));
            }
        }

        Ok(())
    }
}

/// The wire value for a `HospitalType`, matching the CHECK constraint in
/// `20240069`. Written out rather than derived from serde so that a rename of
/// the Rust variant cannot silently start writing a value the constraint
/// rejects at runtime.
fn hospital_type_as_str(t: crate::models::hospital::HospitalType) -> &'static str {
    use crate::models::hospital::HospitalType as T;
    match t {
        T::General => "general",
        T::Teaching => "teaching",
        T::Specialist => "specialist",
        T::Clinic => "clinic",
        T::PrimaryHealthCenter => "primary_health_center",
        T::Maternity => "maternity",
        T::DiagnosticCenter => "diagnostic_center",
        T::Other => "other",
    }
}

/// Minutes since midnight for an `HH:MM` string, or `None` if it is not one.
fn parse_hhmm(value: &str) -> Option<u32> {
    let (h, m) = value.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(day: &str, opens: Option<&str>, closes: Option<&str>, closed: bool) -> OperatingHours {
        OperatingHours {
            day: day.to_string(),
            opens_at: opens.map(str::to_string),
            closes_at: closes.map(str::to_string),
            closed,
        }
    }

    #[test]
    fn a_normal_week_is_accepted() {
        let week: Vec<OperatingHours> = DAYS
            .iter()
            .map(|d| entry(d, Some("08:00"), Some("17:30"), false))
            .collect();
        assert!(HospitalProfileService::validate_operating_hours(&week).is_ok());
    }

    #[test]
    fn a_closed_day_needs_no_times() {
        let week = vec![entry("sunday", None, None, true)];
        assert!(HospitalProfileService::validate_operating_hours(&week).is_ok());
    }

    #[test]
    fn an_open_day_without_times_is_rejected() {
        let week = vec![entry("monday", None, None, false)];
        let r = HospitalProfileService::validate_operating_hours(&week);
        assert!(
            matches!(r, Err(HospitalProfileError::InvalidOperatingHours(_))),
            "got {r:?}"
        );
    }

    #[test]
    fn a_duplicated_day_is_rejected() {
        let week = vec![
            entry("monday", Some("08:00"), Some("12:00"), false),
            entry("Monday", Some("13:00"), Some("17:00"), false),
        ];
        let r = HospitalProfileService::validate_operating_hours(&week);
        assert!(
            matches!(r, Err(HospitalProfileError::InvalidOperatingHours(_))),
            "got {r:?}"
        );
    }

    #[test]
    fn a_close_before_the_open_is_rejected() {
        let week = vec![entry("tuesday", Some("18:00"), Some("09:00"), false)];
        let r = HospitalProfileService::validate_operating_hours(&week);
        assert!(
            matches!(r, Err(HospitalProfileError::InvalidOperatingHours(_))),
            "got {r:?}"
        );
    }

    #[test]
    fn malformed_times_are_rejected() {
        for bad in ["8:00", "0800", "24:00", "12:60", "noon", ""] {
            let week = vec![entry("wednesday", Some(bad), Some("23:00"), false)];
            let r = HospitalProfileService::validate_operating_hours(&week);
            assert!(
                matches!(r, Err(HospitalProfileError::InvalidOperatingHours(_))),
                "'{bad}' should have been rejected, got {r:?}"
            );
        }
    }

    #[test]
    fn hhmm_parses_to_minutes() {
        assert_eq!(parse_hhmm("00:00"), Some(0));
        assert_eq!(parse_hhmm("23:59"), Some(1439));
        assert_eq!(parse_hhmm("09:30"), Some(570));
    }
}
