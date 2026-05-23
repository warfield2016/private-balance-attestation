// Off-chain path e2e: drive the four-message Logos Messaging flow
// (offer -> challenge -> response -> result) end-to-end against
// the SDK's verify_attestation, assert admission.
//
// Skipped unless LEZ_SEQUENCER_URL is set (proof generation still
// needs a live sequencer to pull the Merkle path).

use attestation_e2e_tests::sequencer_available;

#[test]
fn admitted_after_valid_challenge_response() {
    if !sequencer_available() {
        eprintln!("skipped: LEZ_SEQUENCER_URL not set");
        return;
    }
    // 1. Prove against the local sequencer.
    // 2. Stand up a paired InProcessTransport.
    // 3. Prover side sends AttestationOffer.
    // 4. Verifier side issues challenge.
    // 5. Prover signs with spending sk.
    // 6. Verifier runs attestation_verifier::verify_attestation.
    // 7. Assert Admitted with the expected presenter_pk.
    panic!("e2e harness scaffolded; live sequencer wiring lands with `make deploy`");
}

#[test]
fn forwarded_proof_without_secret_key_is_rejected() {
    if !sequencer_available() {
        eprintln!("skipped: LEZ_SEQUENCER_URL not set");
        return;
    }
    // Alice generates a valid proof and hands it to Bob.
    // Bob attempts admission with his own keypair → SignatureInvalid.
    panic!("e2e harness scaffolded; live sequencer wiring lands with `make deploy`");
}
