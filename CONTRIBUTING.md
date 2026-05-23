# Contributing

## Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | stable (≥ 1.90) | `rustup default stable` |
| Node | ≥ 18.18 | nvm or system package |
| RISC0 toolchain | `rzup`-managed | `curl -L https://risczero.com/install \| bash && rzup install` |
| logos-scaffold | latest | `cargo install --git https://github.com/logos-co/logos-scaffold` |
| spel CLI | v0.3.0 | `cargo install --git https://github.com/logos-co/spel spel` |
| Docker or Podman | any recent | system package |

The RISC0 toolchain is only needed to build the guest binary (`make
build`) and run the live e2e tests. Host-side iteration — Rust unit
tests, clippy, fmt, TypeScript typecheck, IDL generation — works
with just `cargo` and `node`.

## Make targets

| Target | Purpose |
|---|---|
| `make setup` | One-time: vendor LEZ, build the standalone sequencer, create a wallet under `.scaffold/wallet`. |
| `make precheck` | Run the same checks CI runs (fmt, clippy, tests, tsc). Run before every push. |
| `make build` | Compile the guest binary (needs RISC0 toolchain). |
| `make idl` | Generate `attestation-idl.json` from the `#[lez_program]` macros. |
| `make deploy` | Build + IDL + deploy to localnet, save program id to `.attestation-state`. |
| `make demo` | End-to-end demo run; refuses to start unless `RISC0_DEV_MODE=0`. |
| `make bundle` | Zip `basecamp-app/` into `basecamp-app.zip` for sideload. |
| `make test` | Host-side unit tests only. |
| `make clean` | Remove `.attestation-state` and `attestation-idl.json`. |

## CI

GitHub Actions runs three jobs on every push to `main`:

1. **Rust — fmt + clippy + test** with `cargo fmt --check`,
   `cargo clippy -- -D warnings`, `cargo test` over
   `attestation_core`, `attestation_verifier`, and
   `attestation_program`.
2. **TypeScript — build + typecheck** for `@lp-0005/sdk` and both
   example packages.
3. **IDL — generate from `#[lez_program]` macros** with
   `RISC0_SKIP_BUILD=1` (the riscv32 guest is not compiled in CI;
   it's developer-local-only, matching `lez-multisig`'s pattern).

`make precheck` runs the equivalent locally. A green precheck
means a green push.

## Workflow

1. Branch from `main`.
2. Edit code or docs.
3. Run `make precheck`. Fix anything it surfaces.
4. Commit (sign-off optional).
5. Open a PR, or push directly to `main` if you have access.

## Project conventions

- **snake_case crates at the workspace root.** No `crates/`
  directory.
- **`Instruction` and `ChallengeComponents` wire types live in
  `attestation_core::instruction`**, re-exported by
  `attestation_program::state` for back-compat. Off-chain callers
  depend on `attestation_core` only.
- **`#[lez_program]` macro mod is gated on `target_os = "zkvm"`.**
  The pure-Rust dispatch in `dispatch.rs` is the host-testable
  surface and shares all rule-checks with the macro mod.
- **Error codes are stable across versions.** A
  `codes_match_documentation_table` test pins the values.
- **British spelling in docs.** Match the upstream `logos-co`
  convention (`organised`, `decentralised`, `prioritised`).
- **No emoji in source or doc files.**

## Adding a new on-chain instruction

1. Add a variant to `attestation_core::Instruction` (in
   `attestation_core/src/instruction.rs`).
2. Add a `#[instruction] pub fn ...` in the `attestation_gate` mod
   of `attestation_program/src/lib.rs` whose argument names match
   the variant's fields.
3. Add a handler module under
   `attestation_program/src/handlers/` that takes the accounts and
   returns `Result<Vec<Account>, SpelError>`.
4. Add a `GateError` variant if the new instruction has its own
   failure modes.
5. Add per-handler unit tests in `#[cfg(test)] mod tests`.
6. Add the instruction to the SPEL IDL by running `make idl`.

## Reporting security issues

See [SECURITY.md](SECURITY.md). Open a private GitHub Security
Advisory; do not file a public issue for vulnerabilities.
