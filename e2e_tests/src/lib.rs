// Shared helpers for e2e_tests. The actual tests live in tests/*.rs
// and are gated on a live sequencer at $LEZ_SEQUENCER_URL.

pub const SEQUENCER_ENV: &str = "LEZ_SEQUENCER_URL";

/// True if the env var is set and points at something usable. Tests
/// gate on this so `cargo test -p attestation_e2e_tests` is a no-op
/// without a sequencer up.
pub fn sequencer_available() -> bool {
    std::env::var(SEQUENCER_ENV).is_ok_and(|v| !v.is_empty())
}
