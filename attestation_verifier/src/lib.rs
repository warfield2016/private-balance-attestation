// Pure attestation verification. Used by the on-chain program and by
// off-chain clients. Keep it small; the security argument depends on
// this being the single point that decides what counts as a valid
// proof.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use risc0_zkvm::Receipt;

use attestation_core::{journal::JournalFields, Hash32, Signature64};

// Cap raw receipt size to avoid bincode allocating arbitrary heap on
// malformed input. Tune as RISC0 receipt sizes evolve.
pub const MAX_RECEIPT_BYTES: usize = 1 << 20; // 1 MiB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    ContextMismatch = 1,
    ThresholdTooLow = 2,
    RootNotTrusted = 3,
    CircuitVersionUnsupported = 4,
    ReceiptInvalid = 5,
    SignatureInvalid = 6,
    JournalDecode = 7,
    InvalidPresenterKey = 8,
    ProgramOwnerMismatch = 9,
}

impl VerifyError {
    pub fn code(self) -> u8 {
        self as u8
    }
}

#[cfg(feature = "std")]
impl core::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            VerifyError::ContextMismatch => "context_id mismatch",
            VerifyError::ThresholdTooLow => "threshold below minimum",
            VerifyError::RootNotTrusted => "merkle_root not trusted",
            VerifyError::CircuitVersionUnsupported => "circuit version not allowed",
            VerifyError::ReceiptInvalid => "RISC0 receipt invalid",
            VerifyError::SignatureInvalid => "ed25519 signature invalid",
            VerifyError::JournalDecode => "journal decode failed",
            VerifyError::InvalidPresenterKey => "invalid presenter_pk",
            VerifyError::ProgramOwnerMismatch => "program_owner mismatch",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VerifyError {}

pub struct VerifyArgs<'a> {
    pub receipt_bytes: &'a [u8],
    pub image_id: &'a [u32; 8],
    pub expected_context_id: &'a Hash32,
    pub expected_program_owner: &'a Hash32,
    pub trusted_roots: &'a [Hash32],
    pub minimum_threshold: u64,
    pub allowed_versions: &'a [u32],
    pub challenge: &'a Hash32,
    pub signature: &'a Signature64,
}

pub fn verify_attestation(args: VerifyArgs<'_>) -> Result<JournalFields, VerifyError> {
    // Size guard before bincode touches the bytes.
    if args.receipt_bytes.len() > MAX_RECEIPT_BYTES {
        return Err(VerifyError::ReceiptInvalid);
    }

    let receipt: Receipt =
        bincode::deserialize(args.receipt_bytes).map_err(|_| VerifyError::ReceiptInvalid)?;

    // Verify the cryptographic proof before reading any field from the
    // receipt. Reading journal fields first leaks which policy check
    // fails for an unauthenticated receipt.
    receipt
        .verify(*args.image_id)
        .map_err(|_| VerifyError::ReceiptInvalid)?;

    let journal: JournalFields = receipt
        .journal
        .decode()
        .map_err(|_| VerifyError::JournalDecode)?;

    if !args.allowed_versions.contains(&journal.circuit_version) {
        return Err(VerifyError::CircuitVersionUnsupported);
    }
    if journal.context_id != *args.expected_context_id {
        return Err(VerifyError::ContextMismatch);
    }
    if journal.program_owner != *args.expected_program_owner {
        return Err(VerifyError::ProgramOwnerMismatch);
    }
    if journal.threshold < args.minimum_threshold {
        return Err(VerifyError::ThresholdTooLow);
    }
    if !args.trusted_roots.iter().any(|r| r == &journal.merkle_root) {
        return Err(VerifyError::RootNotTrusted);
    }

    let vk = VerifyingKey::from_bytes(&journal.presenter_pk)
        .map_err(|_| VerifyError::InvalidPresenterKey)?;

    // ed25519-dalek 2.x: Signature::from_bytes is infallible on
    // &[u8; 64]; the actual validation runs in verify(). If anyone
    // downgrades to 1.x this becomes a Result and must be updated.
    let sig = Signature::from_bytes(args.signature);
    vk.verify(args.challenge, &sig)
        .map_err(|_| VerifyError::SignatureInvalid)?;

    Ok(journal)
}

// Convenience peek for UI flows that want to display "you're about to
// authenticate as <presenter_pk>" before signing. Not a verification.
pub fn peek_journal(receipt_bytes: &[u8]) -> Result<JournalFields, VerifyError> {
    if receipt_bytes.len() > MAX_RECEIPT_BYTES {
        return Err(VerifyError::ReceiptInvalid);
    }
    let receipt: Receipt =
        bincode::deserialize(receipt_bytes).map_err(|_| VerifyError::ReceiptInvalid)?;
    receipt
        .journal
        .decode()
        .map_err(|_| VerifyError::JournalDecode)
}

pub trait RootSource {
    type Error;
    fn recent_roots(&self, k: usize) -> Result<Vec<Hash32>, Self::Error>;
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::*;
    use attestation_core::journal::CIRCUIT_VERSION;

    #[test]
    fn error_codes_are_stable() {
        // Docs pin these numeric values; this test catches accidental
        // reordering of the enum variants.
        assert_eq!(VerifyError::ContextMismatch.code(), 1);
        assert_eq!(VerifyError::ThresholdTooLow.code(), 2);
        assert_eq!(VerifyError::RootNotTrusted.code(), 3);
        assert_eq!(VerifyError::CircuitVersionUnsupported.code(), 4);
        assert_eq!(VerifyError::ReceiptInvalid.code(), 5);
        assert_eq!(VerifyError::SignatureInvalid.code(), 6);
        assert_eq!(VerifyError::JournalDecode.code(), 7);
        assert_eq!(VerifyError::InvalidPresenterKey.code(), 8);
        assert_eq!(VerifyError::ProgramOwnerMismatch.code(), 9);
    }

    #[test]
    fn peek_journal_rejects_garbage_receipt() {
        assert_eq!(
            peek_journal(b"not a receipt"),
            Err(VerifyError::ReceiptInvalid)
        );
    }

    #[test]
    fn peek_journal_rejects_oversize_receipt() {
        let huge = vec![0u8; MAX_RECEIPT_BYTES + 1];
        assert_eq!(peek_journal(&huge), Err(VerifyError::ReceiptInvalid));
    }

    #[test]
    fn circuit_version_constant_is_two() {
        assert_eq!(CIRCUIT_VERSION, 2);
    }
}
