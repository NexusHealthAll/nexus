//! Background schedulers — Tokio loops driving cadence and sweep work.
//!
//! Each scheduler is a struct with a `run(self)` async method, spawned from
//! `main.rs` via `tokio::spawn`. The pattern mirrors the existing
//! [`EmailOutboxWorker`](crate::services::EmailOutboxWorker).

pub mod broadcast;
pub mod expiry;
pub mod handover;

pub use broadcast::BroadcastScheduler;
pub use expiry::OfferExpiryScheduler;
pub use handover::HandoverAutoApprovalScheduler;
