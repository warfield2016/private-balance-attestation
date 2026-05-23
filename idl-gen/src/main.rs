// Prints the SPEL IDL for attestation_program as JSON to stdout.
// `make idl` runs this and redirects to attestation-idl.json.
spel_framework::generate_idl!("../attestation_program/src/lib.rs");
