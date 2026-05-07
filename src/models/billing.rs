use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The card network / payment instrument type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "card_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    Visa,
    Mastercard,
    Verve,
    AmericanExpress,
    Unknown,
}

/// Lifecycle state of a billing transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    /// Charge initiated, awaiting Paystack confirmation
    Pending,
    /// Paystack confirmed the charge succeeded
    Success,
    /// Charge failed (insufficient funds, declined, etc.)
    Failed,
    /// Charge was reversed / refunded
    Reversed,
}

/// What the charge was for.
/// Only shift fees are evidenced in the UI ("You'll be billed for shifts").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "billing_event_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BillingEventType {
    /// Per-shift clinician fee — "You'll be billed for shifts"
    ShiftFee,
}

// ---------------------------------------------------------------------------
// Payment method
// ---------------------------------------------------------------------------

/// A tokenized payment method stored for a hospital.
///
/// IMPORTANT: Raw card numbers and CVVs are NEVER stored here.
/// The frontend collects card details via the Paystack SDK, which returns
/// an authorization_code. Only that token + masked display info is persisted.
/// "Secure payment powered by Paystack"
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HospitalPaymentMethod {
    pub id: Uuid,
    pub hospital_id: Uuid,

    // --- Paystack token (the only sensitive reference we store) ---
    /// Paystack authorization code, e.g. "AUTH_xxxxxxxx"
    pub paystack_authorization_code: String,
    /// Paystack customer code tied to this hospital
    pub paystack_customer_code: Option<String>,

    // --- Display-only card metadata (safe to store, returned by Paystack) ---
    /// Name as entered by the user, e.g. "Dr. Adeyemi Michael"
    pub cardholder_name: String,
    /// Last 4 digits for display, e.g. "4242"
    pub card_last_four: String,
    pub card_type: CardType,
    /// MM/YY expiry for display, e.g. "12/26"
    pub card_expiry: String,
    /// Bank name returned by Paystack, e.g. "GTBank"
    pub bank_name: Option<String>,

    /// Whether this is the hospital's active default payment method
    pub is_default: bool,
    /// Whether this authorization is still valid (Paystack can deactivate tokens)
    pub is_active: bool,

    /// The user who added this payment method (NULL during initial registration)
    pub added_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Billing transactions
// ---------------------------------------------------------------------------

/// A single billing charge or credit on a hospital's account.
/// "You'll be billed for shifts."
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingTransaction {
    pub id: Uuid,
    pub hospital_id: Uuid,
    /// The payment method used for this charge
    pub payment_method_id: Option<Uuid>,

    pub event_type: BillingEventType,
    /// Amount in the smallest currency unit (kobo for NGN, e.g. 500000 = ₦5,000)
    pub amount_kobo: i64,
    /// ISO 4217 currency code, e.g. "NGN"
    pub currency: String,

    pub status: TransactionStatus,

    // --- Paystack references ---
    /// Paystack transaction reference, e.g. "TXN_xxxxxxxx"
    pub paystack_reference: Option<String>,
    /// Paystack transaction ID returned after charge
    pub paystack_transaction_id: Option<String>,

    // --- What was billed ---
    /// The shift this charge relates to (NULL for subscription fees)
    pub shift_id: Option<Uuid>,
    /// Human-readable description, e.g. "Shift fee — Dr. Okafor, 12 Jan 2025"
    pub description: Option<String>,

    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Payload sent after Paystack's frontend SDK tokenizes the card.
/// The backend receives only the authorization reference — never raw card data.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AddPaymentMethodRequest {
    /// Cardholder name as entered on the form
    #[validate(length(min = 2, max = 255, message = "Cardholder name is required"))]
    pub cardholder_name: String,

    /// Paystack authorization code returned by the SDK after card tokenization
    #[validate(length(min = 1, message = "Paystack authorization code is required"))]
    pub paystack_authorization_code: String,

    /// Paystack customer code (returned alongside the authorization)
    pub paystack_customer_code: Option<String>,

    /// Last 4 digits (returned by Paystack, safe to store)
    #[validate(length(equal = 4, message = "card_last_four must be exactly 4 digits"))]
    pub card_last_four: String,

    pub card_type: CardType,

    /// MM/YY format
    #[validate(length(equal = 5, message = "card_expiry must be in MM/YY format"))]
    pub card_expiry: String,

    pub bank_name: Option<String>,

    /// Set this card as the default payment method
    pub set_as_default: Option<bool>,
}

/// Safe response — authorization code is excluded from API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodResponse {
    pub id: Uuid,
    pub hospital_id: Uuid,
    pub cardholder_name: String,
    pub card_last_four: String,
    pub card_type: CardType,
    pub card_expiry: String,
    pub bank_name: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<HospitalPaymentMethod> for PaymentMethodResponse {
    fn from(p: HospitalPaymentMethod) -> Self {
        Self {
            id: p.id,
            hospital_id: p.hospital_id,
            cardholder_name: p.cardholder_name,
            card_last_four: p.card_last_four,
            card_type: p.card_type,
            card_expiry: p.card_expiry,
            bank_name: p.bank_name,
            is_default: p.is_default,
            is_active: p.is_active,
            created_at: p.created_at,
        }
    }
}

/// Response for a billing transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingTransactionResponse {
    pub id: Uuid,
    pub hospital_id: Uuid,
    pub event_type: BillingEventType,
    /// Amount formatted for display, e.g. "₦5,000.00"
    pub amount_kobo: i64,
    pub currency: String,
    pub status: TransactionStatus,
    pub description: Option<String>,
    pub shift_id: Option<Uuid>,
    pub paystack_reference: Option<String>,
    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<BillingTransaction> for BillingTransactionResponse {
    fn from(t: BillingTransaction) -> Self {
        Self {
            id: t.id,
            hospital_id: t.hospital_id,
            event_type: t.event_type,
            amount_kobo: t.amount_kobo,
            currency: t.currency,
            status: t.status,
            description: t.description,
            shift_id: t.shift_id,
            paystack_reference: t.paystack_reference,
            initiated_at: t.initiated_at,
            completed_at: t.completed_at,
        }
    }
}
