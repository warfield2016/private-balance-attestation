# LP-0005 — private balance attestation for LEZ

🌐 **Live Basecamp app:** <https://lp-0005-attestation.vercel.app> ·
short-links: [/repo](https://lp-0005-attestation.vercel.app/repo) ·
[/bounty](https://lp-0005-attestation.vercel.app/bounty)

On a transparent chain, a vote, a fee tier, or a chat-room invite
exposes the holder's wallet to anyone watching — and through that
wallet, every other action they will ever take. This primitive
breaks the linkage at the cryptographic layer, not at the policy
layer.

A holder of a shielded LEZ token account generates a RISC0 proof
that `balance >= N` and presents it through either of two
verification paths against the same `verify_attestation` function:

- **on-chain** via `attestation_program`, a `#[lez_program]`
  annotated SPEL program (governance gates, fee tiers, allowlists)
- **off-chain** over **Logos Messaging**, verified locally by the
  recipient with no transaction (gated chat groups, private
  allowlists, member-only assemblies)

The proof reveals: a Merkle root, a threshold, a context-id, the
presenter's verification key, the token-program owner. It does not
reveal: the balance, the nullifier key, the nonce, the data payload,
or any link to other actions the holder has taken.

LP-0005 bounty submission. MIT or Apache-2.0.

## Layout

LEZ-canonical: snake_case crates at the workspace root, `methods/`
holds the RISC0 guest as a separate sub-workspace.

```
attestation_core/        shared types, hashing, PDA, instruction wire (no_std)
attestation_program/     on-chain LEZ program (#[lez_program] surface)
attestation_host/        RISC0 prover wrapper
attestation_verifier/    off-chain verifier (host-side full check)
attestation_cli/         prove / verify / inspect / sign / context-id
methods/                 RISC0 build glue (build.rs invokes embed_methods)
methods/guest/           the riscv32im guest binary + IDL generator
idl-gen/                 host-side IDL JSON generator
e2e_tests/               sequencer-gated end-to-end tests
basecamp-app/            static HTML/JS bundle, sideloadable in Basecamp
examples/                governance-gate, chat-gate, fee-tier-gate
ts/                      @lp-0005/sdk: TypeScript wrapper + Logos Messaging
docs/                    design notes (architecture, circuit, ...)
solutions/               LP-0005.md submission text (LP-0000 template)
outreach/                external-integrator post templates
scripts/                 demo.sh, benchmark.sh, setup-devnet.sh

Makefile                 setup / build / idl / cli / deploy / demo / precheck / bundle
spel.toml                SPEL CLI config + pinned binary path
scaffold.toml            logos-scaffold config with pinned LEZ + SPEL tags
ADR.md, FURPS.md, SPEC.md, SECURITY.md, CONTRIBUTING.md
```

Pinned dependencies (matching `lez-multisig`'s working build):

```
nssa_core      v0.2.0-rc3   (logos-blockchain/logos-execution-zone)
spel-framework v0.3.0       (logos-co/spel)
risc0-zkvm     =3.0.5       (risc0/risc0)
```

## Build

```
# Rust (stable), Node 18+, the RISC0 toolchain, logos-scaffold, spel:
curl -L https://risczero.com/install | bash
rzup install
cargo install --git https://github.com/logos-co/logos-scaffold
cargo install --git https://github.com/logos-co/spel spel

# One-time project setup: vendor LEZ + create the local wallet
make setup

# Build the guest, generate the IDL, deploy to localnet:
make build idl deploy

# Or the host-side toolchain only:
cargo build --workspace --release
npm install --workspaces --include-workspace-root
npm -w @lp-0005/sdk run build
```

`make precheck` runs the same gauntlet CI does. Run it before any
push.

## Demo

```
export RISC0_DEV_MODE=0   # required: real proofs, not dev-mode
make demo
```

`make demo` chains `build`, `idl`, and `scripts/demo.sh`. The script
mints two test accounts at the localnet sequencer, runs the on-chain
path through the governance example (Alice above threshold admitted,
Bob below rejected with `E_THRESHOLD_TOO_LOW`), then runs the
off-chain chat-gate flow with four messages over the in-process
transport.

CLI:

```
attestation-cli context-id program <program_pk> <gate_seed>
attestation-cli context-id chat    <group_pk>   <epoch>
attestation-cli context-id fee     <tier>       <group_pk>

attestation-cli prove \
    --threshold 1000 \
    --context-id 0x... \
    --presenter-pk 0x... \
    --program-owner 0x... \
    --witness ./witness.json \
    --out ./out/proof.bin

attestation-cli inspect --proof ./out/proof.bin
attestation-cli verify  --proof ./out/proof.bin \
                        --context-id 0x... --program-owner 0x... \
                        --challenge ./challenge.bin \
                        --signature ./sig.bin \
                        --trusted-roots ./roots.bin
```

## Privacy and what it forecloses

What stays private:

- the balance value
- `npk` and any nullifier derivation
- the account's `nonce` and `data` blob
- cross-gate linkability, if the presenter uses a fresh
  `presenter_pk` per gate (the SDK has an ephemeral mode)

What becomes public:

- the Merkle root the proof was generated against
- the threshold the proof clears
- the `program_owner` (which token program the account belongs to)
- the `presenter_pk` the verifier's challenge is signed against

Forwarding resistance: a proof handed to a third party does not
authorise the recipient unless they also hold the spending secret
key embedded in the account's `data`. Cooperative live signing
(Alice signs Bob's challenge in real time) is unsolvable in pure
cryptography and is documented as a policy concern in
[docs/presenter-binding.md](docs/presenter-binding.md).

## Why this matters for parallel institutions

The primitive is Circle-ready. A local Circle running a treasury, a
member roll, or a gated assembly can adopt it without integration
work: derive a `gate_seed` per assembly, point any
`#[lez_program]`-using program at the same `attestation_program`
via CPI, and gate any state-changing action on a verified
attestation. The chat-gate example shows the same pattern for
purely off-chain admission — no transaction, no public record.

## Error codes

Codes overlap between the on-chain program and the off-chain
verifier for the shared range; on-chain-only codes are higher.

```
 1  context_id mismatch
 2  threshold below minimum
 3  merkle_root not in trusted set
 4  circuit version not allowed
 5  receipt invalid
 6  signature invalid
 7  journal decode failed
 8  invalid presenter_pk
 9  program_owner mismatch
10  challenge slot stale       (on-chain)
11  presenter pk did not sign  (on-chain)
12  challenge reused           (on-chain)
13  gate already initialised   (on-chain)
```

## Status

Code, docs, submission text, Basecamp bundle, CI workflow, and
outreach templates are in. Three operational items close the
submission and are laid out as a runbook in [SHIP.md](SHIP.md):

1. **`make setup && make deploy`** — capture the program id (~45 min)
2. **Record the demo video** with `RISC0_DEV_MODE=0` visible (~60 min)
3. **External-party integration** — templates in
   [outreach/](outreach/) (~10 min to post; 24-72 h wall clock)

Then [SHIP.md](SHIP.md) Step 3 files the PR to
`logos-co/lambda-prize`.

## License

MIT or Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
