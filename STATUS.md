# Submission status

Snapshot of where this repo stands against the LP-0005 success
criteria. Treat as an internal scorecard; the canonical submission
text is in [solutions/LP-0005.md](solutions/LP-0005.md).

Symbols: ✓ done · ◐ partial · ☐ pending Step 1/2/4 in
[SHIP.md](SHIP.md).

## Functionality

| Criterion | State | Where |
|---|---|---|
| Client-side `balance >= N` proof | ✓ | `attestation_circuit` + `attestation_host` |
| Hides npk, balance, account identity | ✓ | only `JournalFields` committed |
| Context-bound (no cross-gate replay) | ✓ | `attestation_core::context`, trailing-colon domains |
| Presenter-bound (forwarding-resistant) | ✓ | equality-bind in circuit + ed25519 challenge |
| Targets LEZ commitment format byte-for-byte | ✓ | `attestation_core::commitment::compute_commitment` |
| On-chain LEZ verifier program | ✓ | `attestation_program` with `#[lez_program]` |
| Off-chain over Logos Messaging | ✓ | SDK + `examples/chat-gate/` |
| 3 integrations, ≥1 external | ◐ | scaffolds ready; external slot is [SHIP.md Step 4](SHIP.md) |
| Full docs + clean public repo | ✓ | `docs/`, ADR/FURPS/SPEC/SECURITY at root |

## Usability

| Criterion | State | Where |
|---|---|---|
| Module/SDK | ✓ | Rust crates + `@lp-0005/sdk` TypeScript |
| Basecamp app GUI | ✓ | `basecamp-app/`, `make bundle` for the zip |
| SPEL IDL | ✓ | `make idl` → `attestation-idl.json` |

## Reliability

| Criterion | State | Where |
|---|---|---|
| Proof-gen failures surface cleanly | ✓ | `ProveError` enum, 5 variants |
| Off-chain failure doesn't leak data | ✓ | `MessagingError` kinds only |
| Documented deterministic error codes | ✓ | `VerifyError` 1–9, `GateError` 1–13, pinned tests |

## Performance

| Criterion | State | Where |
|---|---|---|
| CU cost documented per op | ◐ | `docs/benchmarks.md` provisional pending [SHIP.md Step 1](SHIP.md) |

## Supportability

| Criterion | State | Where |
|---|---|---|
| Deployed on LEZ testnet | ☐ | [SHIP.md Step 1](SHIP.md) |
| E2E tests in CI | ✓ | `.github/workflows/ci.yml` e2e job |
| CI green on default branch | depends | run latest = TBD; see badge |
| README documents end-to-end usage | ✓ | top-level `README.md` + this file |
| Reproducible demo script | ✓ | `scripts/demo.sh` + `make demo` |
| Recorded demo video | ☐ | [SHIP.md Step 2](SHIP.md) |

## Submission requirements

| Item | State |
|---|---|
| Public repo, MIT or Apache-2.0 | ✓ |
| Verifier program deployed on testnet | ☐ Step 1 |
| Narrated demo video | ☐ Step 2 |
| Write-up | ✓ `solutions/LP-0005.md` |
| Proof-gen + on-chain gas benchmarks | ◐ provisional |

## Open items, total

- **3 hard blockers** for filing: deploy, video, external integrator
- **0 design blockers** — every cryptographic + architectural decision
  is in place and documented (ADR.md ADR-001 through ADR-008)
- **0 code-level blockers** in the host-side crates;
  `attestation_program` builds cleanly without the verifier-crate
  dep that was pulling host crypto into the guest path

## Next action

Run [SHIP.md](SHIP.md). The three steps are mechanical and the
runbook lays them out in order.
