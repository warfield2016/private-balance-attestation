# External-integrator outreach

Three message variants, depending on channel + audience. Pick one,
paste, replace `<your handle>` and a deadline.

## Logos Discord — `#builders` channel

```
Building LP-0005 (private balance attestation for LEZ) and looking
for one external integrator to wire `attestation_verifier` into a
real fee-tier gate. The scaffold is ~200 lines + a clear contract:

  https://github.com/warfield2016/private-balance-attestation/tree/main/examples/fee-tier-gate

What you'd build:
- Replace the in-memory tier table with whatever your service uses
  (DEX, fee-bearing API, allowlist registry, anything)
- Output a 1-page README pointing at your repo and your commit hash
- A 60-second screen recording showing tier admission against a
  proof file produced by `attestation-cli`

I'll cite you in the LP-0005 submission and split the $1,200 prize
80/20 if my submission wins. Need it inside the week.

DM if interested — <your handle>
```

## Forum post — forum.logos.co

Title: `[LP-0005] Looking for one external integrator: fee-tier gate scaffold (~1 day)`

```
The fee-tier scaffold at examples/fee-tier-gate/ verifies a balance
attestation against a per-tier context-id and awards the highest tier
the proven threshold satisfies. It's intentionally minimal so a
third-party can stand it up in an evening — that's the LP-0005
"≥1 integration outside the submitting team" criterion.

Repo: https://github.com/warfield2016/private-balance-attestation
Scaffold: examples/fee-tier-gate/

I cover:
- 80/20 split of the $1,200 prize if my submission wins (your 20%)
- Citation in the submission write-up (your handle + commit hash)
- Pre-merged review of your README before you publish

You bring:
- Original work, MIT or Apache-2.0
- One end-to-end demo against a real or stubbed fee service
- A short note confirming authorship + license acceptance

Pinning to LEZ tag v0.2.0-rc3 and spel-framework v0.3.0. Builds with
logos-scaffold setup; verifies under RISC0_DEV_MODE=0.

Drop a reply or DM me. Closing the slot in ~72 hours either way.

— <your handle>
```

## X / Twitter (≤ 280 chars)

```
LP-0005 (private balance attestation on Logos LEZ) needs one external
integrator to wire the SDK into a real fee-tier gate. ~1 day of work,
$240 + citation in the submission.

Scaffold + contract:
github.com/warfield2016/private-balance-attestation/tree/main/examples/fee-tier-gate
```

## Direct DM template (for known Logos contributors)

```
Hey — quick ask. LP-0005 (the private balance attestation bounty)
requires one integration by a party outside my team. Repo's at:

  https://github.com/warfield2016/private-balance-attestation

The slot is examples/fee-tier-gate/ — small scaffold (~200 lines),
clearly documented hand-off contract. ~1 day of work. Happy to split
$240 of the $1,200 if my submission wins, plus cite you in the
write-up.

If you can't take it on, any pointers on who in the community would
be a good ask? Thanks either way.
```

## What to send back after they bite

Once someone says yes, send them this confirmation message so
authorship is on record:

```
Thanks for taking the slot. Three things to confirm before you
start:

1. You're writing original code; no copy-paste from another repo
   you don't hold the rights to.
2. You agree to license your integration under MIT or Apache-2.0
   (matches the rest of the project).
3. After delivery, you'll send me:
   - A link to your repo + the commit hash
   - A 60-second screen recording of the demo running
   - A one-paragraph note I can paste into docs/integrations.md

The contract is in examples/fee-tier-gate/README.md. Ping me with
any questions.
```
