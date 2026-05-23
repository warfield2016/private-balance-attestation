# SHIP — the final mile

Everything I can build autonomously is in. This file is what you do
to actually file the submission. Total wall-clock: ~3 hours,
broken into 4 steps you can run in any order.

If you only have 30 minutes, skip Step 4 (external integrator) and
note in the submission that the third integration is delivered as a
working scaffold pending an external contributor.

---

## Step 1 — Deploy on LEZ testnet (45 min)

```bash
# Install the tooling if you don't have it yet:
curl -L https://risczero.com/install | bash
rzup install
cargo install --git https://github.com/logos-co/logos-scaffold
cargo install --git https://github.com/logos-co/spel spel

# One-time setup for this project:
cd ~/path/to/private-balance-attestation
make setup        # creates .scaffold/wallet, vendors the LEZ source
make build        # compiles the guest under RISC0_DEV_MODE=0
make idl          # generates attestation-idl.json
make deploy       # publishes the program; saves id to .attestation-state

# Verify and capture the program id:
cat .attestation-state | grep PROGRAM_ID
```

Copy the `PROGRAM_ID` value. Paste it into two places:

1. `solutions/LP-0005.md` — replace `[FILLED_AFTER_DEPLOY]`
2. `README.md` — replace `[PROGRAM_ID_HERE]`

Commit:

```bash
git add solutions/LP-0005.md README.md
git -c user.email="warfield2016@gmail.com" -c user.name="warfield2016" \
    commit -m "Record deployed program id"
git push
```

---

## Step 2 — Record the demo video (60 min)

Required by SR3 and S6. The shot-list is in `docs/architecture.md`
section "Demo script" (and mirrored below for convenience).

### Setup

1. Open a clean terminal, dark theme, font ≥ 16 pt.
2. Run `clear; export RISC0_DEV_MODE=0; echo "RISC0_DEV_MODE = $RISC0_DEV_MODE"`.
   Hold the line on screen for **3 seconds** before moving on.
3. Start screen recording (QuickTime on macOS, OBS elsewhere). Have
   audio narration recorded simultaneously — no silent screencasts.

### Run order (~8 minutes total)

**0:00–0:30 Intro.** Title card or first-person voice: "I'm
warfield2016. This is my submission for LP-0005, private balance
attestation on the Logos Execution Zone."

**0:30–2:00 Architecture walkthrough.** Open
`docs/architecture.md` (or the rendered diagram). Point to:
- the six crates and their roles
- the single `verify_attestation` function the on-chain and
  off-chain paths both call
- the equality-bind presenter scheme in
  `docs/presenter-binding.md`

**2:00–4:00 On-chain path.** In the terminal:

```bash
# Generate two test accounts at the local sequencer
./scripts/devnet/mint-test-account.sh --voter alice --balance 5000
./scripts/devnet/mint-test-account.sh --voter bob   --balance 100

# Alice (above threshold) proves and submits
examples/governance-gate/scripts/cast-vote.sh \
    --voter alice --proposal 42 --vote yes

# Bob (below threshold) attempts — rejected with code 2
examples/governance-gate/scripts/cast-vote.sh \
    --voter bob --proposal 42 --vote yes
```

Narrate: "Alice has 5000 tokens. The proof generation takes about a
minute with RISC0_DEV_MODE=0 — you can see the spinner in the
terminal. Once it's done, the receipt goes to the on-chain program
and the gate admits her vote. Bob's balance is below threshold; the
host catches it client-side before paying the proving cost, but if
we forced him through, the program would reject with code 2,
`E_THRESHOLD_TOO_LOW`."

**4:00–6:30 Off-chain path.** Same terminal:

```bash
node --import tsx examples/chat-gate/src/demo.ts \
     --group-pk      $(cat .scaffold/wallet/group_pk.hex) \
     --program-owner $LEZ_TOKEN_PROGRAM \
     --witness       keys/alice/witness.json \
     --spending-key  keys/alice/spending.bin \
     --threshold     100
```

Output: `Admitted: 0x<presenter_pk>`. Narrate: "No transaction. The
host learned Alice's balance clears the threshold and which ed25519
key she'll sign with going forward. Nothing else."

**6:30–7:30 Basecamp app.** Open the Basecamp loadable, click
through: pick a gate, drop the receipt, see the journal. Narrate:
"The Basecamp app is the consumer-facing surface. Same journal, same
context-id, no command line."

**7:30–8:00 Closing.** Show the GitHub repo's green CI badge. Show
`ls docs/` so the design notes are visible. One sentence:
"Source under MIT or Apache-2.0 at
github.com/warfield2016/private-balance-attestation. Deployed on
LEZ testnet at `<PROGRAM_ID>`."

### After recording

Upload to YouTube (unlisted) or Loom. Paste the URL into:

1. `solutions/LP-0005.md` — replace `[FILLED_AFTER_RECORDING]`
2. `README.md` — top section

Commit + push.

---

## Step 3 — Drop a draft submission PR to logos-co/lambda-prize (10 min)

```bash
# Fork the prize repo on GitHub (one click in the UI), then:
git clone https://github.com/warfield2016/lambda-prize ~/tmp/lambda-prize
cd ~/tmp/lambda-prize
cp /path/to/private-balance-attestation/solutions/LP-0005.md \
   solutions/LP-0005-warfield2016.md
git add solutions/LP-0005-warfield2016.md
git commit -m "Solution: LP-0005 — Private Balance Attestation (warfield2016)"
git push origin main

# Then open a PR via the web UI or:
gh pr create --repo logos-co/lambda-prize \
    --title "Solution: LP-0005 — Private Balance Attestation (warfield2016)" \
    --body "Submitting LP-0005. Repo: https://github.com/warfield2016/private-balance-attestation. Demo video: <URL>. Terms accepted."
```

---

## Step 4 — Open an external-integrator call (parallel, 20 min to post)

The bounty requires ≥1 integration built by a party outside the
submitting team. `examples/fee-tier-gate/` is the slot.

Post on the Logos community channel (Discord `#builders` or the
forum):

```
Building LP-0005 (private balance attestation for LEZ) and looking
for one external integrator to wire the `attestation_verifier` SDK
into a real fee-tier gate. Scaffold is at:
  https://github.com/warfield2016/private-balance-attestation/tree/main/examples/fee-tier-gate

What you'd do:
- Replace the in-memory tier table with whatever your service uses
- Hand me a 1-page README pointing at your repo and your commit hash
- A 60-second screen recording showing tier admission against a
  proof file produced by `attestation-cli`

I'll cite you in the submission. Happy to split the bounty 80/20.

DM if interested.
```

When someone takes it: confirm authorship by DM, get a written
license-acceptance (MIT or Apache-2.0) and a commit-hash link, then
add them to `docs/integrations.md`'s external-integrator slot before
the submission PR merges.

If nobody bites in 24 hours: commission a known Logos contributor on
the same terms. The "outside the submitting team" criterion is met
either way.

---

## Pre-flight before clicking submit

Run this checklist after Steps 1–4 are done:

```bash
make precheck        # green
make build idl      # both succeed under RISC0_DEV_MODE=0
make demo           # end-to-end run against the sequencer
make bundle         # produces basecamp-app.zip
```

Check:

- [ ] `solutions/LP-0005.md` has the program id and video URL filled in
- [ ] `README.md` has the program id and video URL filled in
- [ ] `docs/integrations.md` cites the external integrator
- [ ] CI green on `main` (the badge in the README)
- [ ] `attestation-idl.json` is committed
- [ ] `basecamp-app.zip` attached as a release asset
- [ ] No witness files (`keys/*.json`) accidentally committed

Then post on social per the bounty's social-media policy with a
link back to the ns.com LP-0005 task page.

---

## What to do if Tranquil-Flow's submission #60 wins first

The prize is FCFS. If their submission passes evaluator review
before you file, the $1,200 is theirs. Two contingency paths:

1. **Pivot to LP-0003** (Private Allowlist / Airdrop Distributor) —
   reuses our exact primitives. Open submission by Timidan
   (issue #44) is still pending; we'd be first or second
   depending on timing. ~3 days to adapt.
2. **Land this as portfolio.** The repo is a credible Logos LEZ
   project regardless of the prize outcome. Post the architecture
   write-up on the forum; useful for any future bounty.

Either way, ship Step 3 (file the submission) so you're in line if
Tranquil-Flow's entry fails evaluation. The bounty allows 3
submissions per builder; filing costs nothing.
