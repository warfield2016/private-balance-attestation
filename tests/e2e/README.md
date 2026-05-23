# End-to-end tests

The e2e suite runs the full prove → verify (off-chain) and prove → submit
→ verify (on-chain) flows against a standalone LEZ sequencer.

It is invoked by the `e2e-standalone-sequencer` CI job.

## Why these are separate from unit tests

Unit tests in each crate cover pure logic and never invoke the RISC0
prover. The e2e tests require:

- a real RISC0 toolchain (`rzup install` succeeded),
- `RISC0_DEV_MODE=0` so the receipt is real,
- a local LEZ standalone sequencer with deterministic root anchoring.

Each of those is a heavyweight dependency we don't want to force on
contributors who only touch the off-chain verifier or the SDK. The
e2e job runs only on the `main` push + on PRs that touch the circuit
or the verifier crates.

## Running locally

```bash
export RISC0_DEV_MODE=0
./run-e2e.sh
```

`run-e2e.sh` will land alongside the live LEZ devnet wiring in sprint
day D11. Until then, this directory holds the structure (and CI's
expectations) so the wiring slot is clearly carved out.
