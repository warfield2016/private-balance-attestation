// On-chain error codes. Values match the docs and the off-chain
// `attestation_verifier::VerifyError` codes for the overlapping range
// (1-9); on-chain-only codes are 10-13.

use thiserror::Error;

#[repr(u32)]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GateError {
    #[error("context_id mismatch")]
    ContextMismatch = 1,

    #[error("threshold below minimum")]
    ThresholdTooLow = 2,

    #[error("merkle_root not in recent anchored set")]
    RootStale = 3,

    #[error("circuit version rejected")]
    CircuitVersionRejected = 4,

    #[error("RISC0 receipt invalid")]
    ReceiptInvalid = 5,

    #[error("ed25519 signature invalid")]
    SignatureInvalid = 6,

    #[error("challenge slot_hash older than recent K-window")]
    ChallengeStale = 7,

    #[error("admin-only instruction invoked by non-admin")]
    AdminOnly = 8,

    #[error("challenge has been used already (one-shot gate)")]
    ChallengeReused = 9,

    #[error("journal decode failed")]
    JournalDecode = 10,

    #[error("presenter_pk is not a valid ed25519 verification key")]
    InvalidPresenterKey = 11,

    #[error("program_owner does not match the gate's configured value")]
    ProgramOwnerMismatch = 12,

    #[error("gate state has already been initialised")]
    AlreadyInitialized = 13,
}

impl GateError {
    pub fn code(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_documentation_table() {
        assert_eq!(GateError::ContextMismatch.code(), 1);
        assert_eq!(GateError::ThresholdTooLow.code(), 2);
        assert_eq!(GateError::RootStale.code(), 3);
        assert_eq!(GateError::CircuitVersionRejected.code(), 4);
        assert_eq!(GateError::ReceiptInvalid.code(), 5);
        assert_eq!(GateError::SignatureInvalid.code(), 6);
        assert_eq!(GateError::ChallengeStale.code(), 7);
        assert_eq!(GateError::AdminOnly.code(), 8);
        assert_eq!(GateError::ChallengeReused.code(), 9);
        assert_eq!(GateError::JournalDecode.code(), 10);
        assert_eq!(GateError::InvalidPresenterKey.code(), 11);
        assert_eq!(GateError::ProgramOwnerMismatch.code(), 12);
        assert_eq!(GateError::AlreadyInitialized.code(), 13);
    }
}
