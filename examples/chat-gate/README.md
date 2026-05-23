# Example: off-chain chat-group admission

The headline use case for LP-0005's off-chain path: a chat group host
admits members based on a private balance attestation. No transaction.

## Architecture

```
   Prover                              Host (chat group)
   ──────                              ──────────────────
   prove + send offer  ─────►          receive offer
                                       issue challenge
                       ◄─────          challenge
   sign with sk_spending
   send response       ─────►          verify receipt + signature
                                       admit on success
                       ◄─────          AdmissionResult
   open chat
```

## Files

```
src/demo.ts          end-to-end demo (prover + host in one process)
src/mock-transport.ts paired in-process transport for the demo
```

The demo runs both sides in one process against
`InProcessTransport.pair()`. A real Logos Messaging deployment swaps
that one type for a Logos transport; the rest of the flow is
unchanged.

## Run

```
node --import tsx ./src/demo.ts \
     --group-pk      $(cat ./keys/group_pk.hex) \
     --program-owner $(cat ./keys/program_owner.hex) \
     --witness       ./keys/alice/witness.json \
     --spending-key  ./keys/alice/spending.bin \
     --threshold     100
```

Expected output: `Admitted: 0x<presenter_pk>`.

## Privacy

The host learns:

- the presenter's verification key (an ed25519 pubkey chosen for this
  flow — not the LEZ account)
- that the presenter's balance is at least the configured threshold
- the time of admission

The host does not learn the balance, `npk`, `nonce`, or any other
account field. See [docs/presenter-binding.md](../../docs/presenter-binding.md)
for what does and does not stay private.
