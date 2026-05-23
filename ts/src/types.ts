// Types mirroring the Rust public surface. Bytes are Uint8Array
// throughout; hex conversion is left to the caller so the SDK never
// loses information on round-trip.

export type Hash32 = Uint8Array;
export type Signature64 = Uint8Array;

// Public inputs committed to the journal. Keep field order in sync
// with JournalFields in attestation-core.
export interface JournalFields {
  merkleRoot: Hash32;
  threshold: bigint;
  contextId: Hash32;
  presenterPk: Hash32;
  programOwner: Hash32;
  circuitVersion: number;
}

// Private witness; never leaves the prover.
export interface Witness {
  npk: Hash32;
  programOwner: Hash32;
  balance: bigint;
  nonce: Hash32;
  data: Uint8Array;
  merkleSiblings: Hash32[];
  merkleIndices: boolean[];
  spendingPk: Hash32;
  spendingPkOffset: bigint;
}

export interface ProveOutput {
  receipt: Uint8Array;
  journalBytes: Uint8Array;
  journal: JournalFields;
  proveMs: number;
}

export interface AttestationBundle {
  receipt: Uint8Array;
  journal: JournalFields;
  presenterPk: Hash32;
}

export const CIRCUIT_VERSION = 2 as const;
