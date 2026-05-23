// On-chain path e2e: prove a balance against a deployed gate, submit
// to the LEZ verifier program, verify the gate admits the action.
//
// Skipped unless LEZ_SEQUENCER_URL is set. CI sets RISC0_SKIP_BUILD=1
// for the host-only jobs and never reaches these.

use attestation_e2e_tests::sequencer_available;

#[test]
fn alice_above_threshold_is_admitted() {
    if !sequencer_available() {
        eprintln!("skipped: LEZ_SEQUENCER_URL not set");
        return;
    }

    // Setup steps the real implementation will fill in once the
    // sequencer + deployed program are reachable from this harness:
    //
    // 1. Mint a private account at the sequencer with balance > threshold.
    // 2. Pull the Merkle path via get_proof_for_commitment.
    // 3. Build the witness; call attestation_host::prove.
    // 4. Submit the receipt + ed25519-signed challenge to the gate.
    // 5. Read the program's emitted event; assert presenter_pk matches.
    //
    // The corresponding host helper functions land in attestation_host
    // alongside the live sequencer wiring (SHIP.md Step 1).
    panic!("e2e harness scaffolded; live sequencer wiring lands with `make deploy`");
}

#[test]
fn bob_below_threshold_is_rejected() {
    if !sequencer_available() {
        eprintln!("skipped: LEZ_SEQUENCER_URL not set");
        return;
    }
    // Mirror: mint with balance < threshold, prove fails client-side
    // OR submit and assert ThresholdTooLow on the gate's reject.
    panic!("e2e harness scaffolded; live sequencer wiring lands with `make deploy`");
}
