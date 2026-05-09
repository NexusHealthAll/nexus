pub mod hospital;
pub mod location;
pub mod billing;
pub mod audit;

pub use hospital::HospitalRepository;
pub use location::LocationRepository;
pub use billing::BillingRepository;
pub use audit::AuditRepository;
