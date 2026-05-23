// Pure-Rust handler functions called by the `#[lez_program]` macro mod
// in `lib.rs`. Each takes the accounts the macro routed to it, reads/
// writes their `data` payloads via the nssa_core `Account` shape, and
// returns the post-state account list.
//
// Compiled for both the host (so unit tests run on x86_64) and the
// guest (so the macro mod can delegate). nssa_core::account types are
// target-independent — only spel-framework's macro mod itself is
// `target_os = "zkvm"` gated.

pub mod admin;
pub mod gate_action;
pub mod initialize;
