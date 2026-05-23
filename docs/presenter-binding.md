# Presenter binding

A ZK proof is just bytes. If Alice generates a valid proof, sends it
to Bob, and Bob presents it to a verifier that accepts it on its
face — Bob can impersonate Alice. The bounty calls this out as a
required problem to solve.

## Scheme

Each LEZ account embeds an ed25519 verification key (`spending_pk`)
inside its `data` payload at a known offset. The circuit asserts:

- `data[offset .. offset + 32] == witness.spending_pk`
- `witness.spending_pk == public.presenter_pk`

To use the proof, the verifier issues a fresh 32-byte challenge. The
presenter signs it with the secret key for `presenter_pk`. The
verifier checks both the ZK receipt and the signature; either fails,
both fail.

A forwarded proof is useless without the spending secret key.

## On-chain flow

The on-chain program rebuilds the challenge deterministically from a
recent slot hash, the program id, `presenter_pk`, and the action tag.
The presenter signs this rebuilt challenge before submitting. The
program rebuilds it independently — a presenter can't smuggle in an
attacker-chosen value.

## Off-chain flow

The off-chain verifier sends an explicit challenge over Logos
Messaging, waits for the signed response, and then runs the same
`verify_attestation` call. Same logic, different transport.

## Why `spending_pk` lives in `data`, not as `npk`

LEZ's `npk` is the nullifier public key — it anchors privacy across
many distinct LEZ operations. Conflating it with a signing key would
make a signing-side compromise into a privacy break. Keeping
`spending_pk` separate in `data` is cleaner; the cost is that account
templates reserve a known byte range for it and the circuit takes the
offset as a witness field.

## What this guarantees

- A forwarded proof does not authorise the recipient.
- A presenter cannot smuggle a chosen challenge past the verifier.
- The proof remains balance-private — `presenter_pk` is a signing
  identity, not the LEZ account.

## What this does not guarantee

- **Voluntary key sharing.** If Alice gives Bob her spending key, Bob
  can sign anything Alice could. Pure crypto cannot prevent this.
- **Co-operative live signing.** Alice can sign on Bob's behalf in
  real time. Mitigation is policy, not crypto — rate-limit per
  `presenter_pk` if the gate cares.
- **Linkability across gates.** A single long-lived `presenter_pk`
  used across gates lets those gates correlate the presenter even
  though none of them sees the LEZ account. The SDK supports an
  ephemeral mode that generates a fresh `presenter_pk` per session,
  provided the LEZ token program allows the user to mint or rotate
  `data` for that purpose.
- **Pre-signed challenges.** If the verifier reuses challenges, a
  captured signature can be replayed. Reference verifiers issue fresh
  challenges per session; integrators rolling their own must do the
  same. `freshChallenge()` in the SDK uses `crypto.getRandomValues`.

## Why not signature-of-knowledge inside the circuit

A SoK construction that puts the ed25519 verification inside the
guest is cleaner cryptographically — the signature *is* the
proof — but it adds ~100k cycles of curve work per proof and forces
the verifier to know the challenge ahead of time. Equality-bind plus
an out-of-circuit signature is much cheaper and gives the same
forwarding-resistance. If reviewers decide the equality-bind is not
sufficient, switching to SoK is additive: the off-chain path keeps
working unchanged.
