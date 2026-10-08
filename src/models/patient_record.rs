//! Per-patient handover notes, the consult waiting room, and companion-device
//! handoffs.
//!
//! The doctor's clinical note is **not** here — it lives in
//! `crate::models::consultation_note`, which `20240063` extended with
//! `shift_id` / `session_id` and the locking columns. A second table for the
//! same notes about the same patient would be a data-integrity problem.
//!
//! Status columns on these tables are `TEXT` + `CHECK` rather than Postgres
//! enums (see `migrations/20240063_patient_records_and_consult_devices.sql`),
//! so the enums below map to `TEXT` and every new value stays a plain
//! `ALTER TABLE`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::models::consultation_note::ConsultationNote;
use crate::models::patient::NewPatientRequest;

// Enums

/// Where a queued patient is in the consultation. `Removed` is a withdrawal by
/// the hospital, not a clinical outcome — kept rather than deleted so the queue
/// stays auditable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum QueueEntryState {
    /// In the waiting room, not yet called.
    Waiting,
    /// With the doctor right now. At most one per session, enforced by
    /// `uq_consult_queue_one_in_consult`.
    InConsult,
    /// Seen and released.
    Seen,
    /// Pulled from the queue by the hospital without being seen.
    Removed,
}

impl QueueEntryState {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueEntryState::Waiting => "waiting",
            QueueEntryState::InConsult => "in_consult",
            QueueEntryState::Seen => "seen",
            QueueEntryState::Removed => "removed",
        }
    }
}

// Rows

/// One `(session, patient)` queue entry.
#[derive(Debug, Clone, FromRow)]
pub struct ConsultQueueEntry {
    pub id: Uuid,
    pub session_id: Uuid,
    pub shift_id: Option<Uuid>,
    pub patient_id: Uuid,
    pub hospital_id: Uuid,
    pub state: QueueEntryState,
    pub position: i32,
    pub added_by: Uuid,
    pub called_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
    pub removed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A queue entry joined to the patient's triage data and to whether each note
/// has been filed, so the waiting-room screen needs no per-patient follow-up
/// request.
#[derive(Debug, Clone, FromRow)]
pub struct ConsultQueueEntryWithPatient {
    pub id: Uuid,
    pub session_id: Uuid,
    pub patient_id: Uuid,
    pub state: QueueEntryState,
    pub position: i32,
    pub called_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub full_name: String,
    pub age: f32,
    pub gender: String,
    pub severity_level: String,
    pub predictive_risk_score: Option<f32>,
    /// `true` once this patient has at least one consultation note on this shift.
    pub has_clinical_note: bool,
    /// `true` once the on-site party has filed this patient's handover note.
    pub has_handover_note: bool,
}

/// The on-site hospital party's note. Exactly one per `(shift, patient)`.
#[derive(Debug, Clone, FromRow)]
pub struct PatientHandoverNote {
    pub id: Uuid,
    pub shift_id: Uuid,
    pub patient_id: Uuid,
    pub session_id: Option<Uuid>,
    pub hospital_id: Uuid,
    pub author_user_id: Uuid,

    pub summary: String,
    pub outstanding_tasks: serde_json::Value,
    pub medications_given: serde_json::Value,
    pub escalation_required: bool,
    pub escalation_reason: Option<String>,

    pub submitted_at: DateTime<Utc>,
    pub editable_until: DateTime<Utc>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single-use grant for a companion device. The plaintext code is never
/// stored — only `code_hash` — so this row cannot be replayed into a token.
#[derive(Debug, Clone, FromRow)]
pub struct ConsultDeviceHandoff {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub device_ordinal: i32,
    pub participant_role: crate::models::video_session::ParticipantRole,
    pub mode: crate::models::video_session::JoinMode,
    pub device_label: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub redeemed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Request DTOs

/// `POST /api/v1/shifts/{shift_id}/consult/queue`.
///
/// Exactly one of `patient_id` (queue someone already on file) or `patient`
/// (intake a new one and queue them in the same transaction) must be set. That
/// is a conditional rule, so it is enforced in the service rather than by
/// `validator`, matching how `ShiftService::validate_request` handles the
/// `pay_type`-dependent compensation fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct AddPatientToQueueRequest {
    #[serde(default)]
    pub patient_id: Option<Uuid>,
    #[serde(default)]
    #[validate(nested)]
    pub patient: Option<NewPatientRequest>,
}

/// `PUT /api/v1/shifts/{shift_id}/patients/{patient_id}/handover-note`.
/// Idempotent: a second submit inside the edit window updates the single row
/// for this `(shift, patient)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct SubmitPatientHandoverNoteRequest {
    #[validate(length(min = 1, max = 10000, message = "A handover summary is required"))]
    pub summary: String,

    #[serde(default)]
    pub outstanding_tasks: Vec<serde_json::Value>,

    #[serde(default)]
    pub medications_given: Vec<serde_json::Value>,

    #[serde(default)]
    pub escalation_required: bool,

    #[validate(length(max = 2000))]
    #[serde(default)]
    pub escalation_reason: Option<String>,
}

/// `POST /api/v1/shifts/{shift_id}/consult/handoff` — every field optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CreateHandoffRequest {
    /// Free text shown beside the device in the participant list, e.g. "iPhone".
    #[serde(default)]
    pub device_label: Option<String>,
}

/// `POST /api/v1/consult/handoff/redeem` — unauthenticated; the code is the
/// credential.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct RedeemHandoffRequest {
    #[validate(length(min = 1, max = 128, message = "A handoff code is required"))]
    pub code: String,
}

// Response DTOs

/// One patient in the waiting room. Carries no clinical detail beyond triage —
/// the notes themselves are fetched per patient.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsultQueuePatientView {
    pub entry_id: Uuid,
    pub patient_id: Uuid,
    pub full_name: String,
    pub age: f32,
    pub gender: String,
    pub severity_level: String,
    /// From the ML triage pipeline; `null` until the prediction completes.
    pub predictive_risk_score: Option<f32>,
    pub state: QueueEntryState,
    pub position: i32,
    pub called_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
    pub queued_at: DateTime<Utc>,
    pub has_clinical_note: bool,
    pub has_handover_note: bool,
}

/// The waiting-room block on `GET /consult` and `GET /consult/queue`.
///
/// `patients` is empty for platform admins, who get the counts but never
/// patient identities — the same NDPR line `authorize_shift_access` already
/// draws for the video stream.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ConsultWaitingRoomView {
    pub waiting: i64,
    pub in_consult: i64,
    pub seen: i64,
    /// Everything ever queued for this session, `removed` included.
    pub total: i64,
    pub patients: Vec<ConsultQueuePatientView>,
}

/// A per-patient handover note as returned to an entitled reader.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatientHandoverNoteView {
    pub id: Uuid,
    pub shift_id: Uuid,
    pub patient_id: Uuid,
    pub author_user_id: Uuid,
    pub author_name: Option<String>,
    pub summary: String,
    pub outstanding_tasks: serde_json::Value,
    pub medications_given: serde_json::Value,
    pub escalation_required: bool,
    pub escalation_reason: Option<String>,
    pub submitted_at: DateTime<Utc>,
    /// `false` once `editable_until` has passed.
    pub editable: bool,
    pub editable_until: DateTime<Utc>,
}

/// One patient's complete record for a shift.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PatientRecordView {
    pub patient_id: Uuid,
    pub full_name: String,
    pub age: f32,
    pub gender: String,
    pub severity_level: String,
    pub queue_state: Option<QueueEntryState>,
    pub clinical_notes: Vec<ConsultationNote>,
    pub handover_note: Option<PatientHandoverNoteView>,
}

/// `GET /api/v1/shifts/{shift_id}/patient-records` — the hospital's read of
/// everything a shift produced. Remains available after the shift completes.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ShiftPatientRecordsView {
    pub shift_id: Uuid,
    /// Lets the UI tell a live shift from a final record.
    pub shift_status: String,
    pub shift_ended_at: Option<DateTime<Utc>>,
    pub patients_total: i64,
    pub patients_seen: i64,
    /// Empty for platform admins, who receive counts only.
    pub patients: Vec<PatientRecordView>,
    /// `true` when the caller was served counts without clinical content.
    pub metadata_only: bool,
}

/// `200` from `POST /consult/handoff`.
///
/// `handoff_url` carries the code in the URL **fragment**, which browsers never
/// send to the server — so the credential stays out of access logs, out of any
/// `Referer`, and out of the proxy chain. Show it as a QR code or hand it to
/// the user's own device; it is single-use and short-lived.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateHandoffResponse {
    pub session_id: Uuid,
    pub handoff_url: String,
    /// The plaintext code, returned exactly once. Only its hash is stored.
    pub code: String,
    pub device_ordinal: i32,
    pub expires_at: DateTime<Utc>,
}
