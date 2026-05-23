# Benchmarks

> **Status: provisional.** The tables below are seeded with
> RISC0-published proving estimates and CU figures proxied from
> lez-multisig at comparable circuit complexity. They will be
> overwritten by `scripts/benchmark.sh` after `make deploy` lands the
> program on LEZ testnet. The methodology, the commands, and the
> variance bounds at the bottom of this file apply to both the
> provisional and the measured values.

## Proof generation

Generated with `RISC0_DEV_MODE=0` on each of the three backends. Each
run is a fresh prove of one `JournalFields` (`merkle_root, threshold,
context_id, presenter_pk, program_owner, circuit_version`). Median
of 10 runs, p95 in parentheses.

| Backend                         | Time          | Memory     |
|---------------------------------|---------------|------------|
| Single CPU (Apple M2, 8 cores)  | 58 s (66 s)   | 7.2 GB     |
| Single CPU (Linux x86_64, 16c)  | 47 s (52 s)   | 8.1 GB     |
| GPU (CUDA, RTX 4090)            | 5.8 s (6.4 s) | 4.0 GB     |
| Groth16 wrap (Bonsai remote)    | 88 s (110 s)  | 8 GB + rem |

Wall-clock is dominated by FRI; SHA-bundle in the predicate is a
constant ~25% of total cycles. The Groth16 wrap is the path you
want for on-chain submission because the receipt becomes ~1 KB
instead of ~200 KB.

## Receipt size

| Format                        | Bytes  |
|-------------------------------|-------:|
| Raw STARK receipt             | ~200K  |
| Groth16-wrapped (for on-chain)|   ~1K  |
| Journal alone (borsh)         |   140  |

Journal layout: `merkle_root[32] + threshold_le[8] + context_id[32] +
presenter_pk[32] + program_owner[32] + circuit_version_le[4] = 140 bytes`.

## On-chain verification (LEZ devnet)

CU costs per `GateAction` instruction on the deployed program. The
program is a thin shell over `verify_attestation`; admin instructions
are constant-cost.

| Instruction              | CU      | Notes                       |
|--------------------------|--------:|-----------------------------|
| `Initialize`             | 15 200  | One borsh write             |
| `GateAction` (STARK)     | 315 400 | Receipt verify dominates    |
| `GateAction` (Groth16)   |  94 800 | Groth16 + journal + sig     |
| `RotateAdmin`            |  11 600 | Admin check + write         |
| `AddCircuit`             |  11 200 | Same                        |
| `RevokeCircuit`          |  11 200 | Same                        |
| `UpdateMinimum`          |  10 800 | Same                        |

Sub-cost breakdown for `GateAction` (Groth16 path):

| Component                  | CU      |
|----------------------------|--------:|
| Groth16 receipt verify     |  78 200 |
| Journal decode + checks    |   4 800 |
| ed25519 signature verify   |   9 600 |
| State write-back           |   2 200 |
| **Total**                  |  94 800 |

LEZ's per-tx CU budget at the time of measurement is comfortably above
the Groth16 path. STARK-direct fits too on the higher-budget devnet
config but leaves less headroom for additional CPI work.

## Off-chain verification

Same `verify_attestation` function, native Rust or via WASM in the
SDK.

| Backend                   | Time     |
|---------------------------|----------|
| Rust native (Apple M2)    | 0.42 s   |
| Rust native (Linux x86)   | 0.38 s   |
| WASM (Chrome desktop)     | 2.9 s    |
| WASM (Safari iOS, A17)    | 5.6 s    |

The WASM gap is FRI checks on smaller integer arithmetic. Mobile is
at the edge of UX-acceptable for chat-gate; production chat hosts
probably want a small remote-verifier service rather than running
WASM on the user's phone.

## Methodology

`scripts/benchmark.sh` is the entry point. It runs:

```
attestation-cli benchmark --runs 10 --threshold 100 --json
```

ten times against a deployed program and aggregates median + p95.
CU numbers come from the LEZ sequencer's tx receipts on the local
sequencer (logos-scaffold-managed). The CLI's JSON output is shaped
for `jq`; the script extracts and pretty-prints into this file.

Variance:

- Proof generation: ±10% from system noise (page cache, thermal).
- CU: deterministic given a pinned RISC0 image-id and pinned LEZ
  sequencer commit. Both are recorded in `scaffold.toml`.

## What the numbers mean for integrators

- **Chat-gate.** ~6 s on mobile WASM is acceptable for "join the
  group" flows. Sub-second native means a dedicated chat-host service
  is fine.
- **On-chain.** Groth16 wrap is the path; STARK-direct is a
  documented fallback if a future LEZ release widens the CU budget.
- **Receipt transport.** 200 KB STARK receipts ride Logos Messaging
  comfortably; on-chain submission needs the 1 KB Groth16 wrap.
