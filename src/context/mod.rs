//! Context capture and fingerprinting for suggestions.

pub mod capture;
pub mod fingerprint;

pub use capture::{CaptureError, ContextCapture};
pub use fingerprint::{ChangeSignificance, ContextFingerprint};
