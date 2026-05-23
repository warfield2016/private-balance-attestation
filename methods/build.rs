// Build script: compile the guest binaries in `guest/` for the
// riscv32im-risc0-zkvm-elf target and emit ATTESTATION_*_ELF / _ID
// constants into OUT_DIR/methods.rs for the host crate to include.
fn main() {
    risc0_build::embed_methods();
}
