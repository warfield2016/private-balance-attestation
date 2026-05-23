// Error codes aligned with VerifyError/GateError on the Rust side.
// Numeric values are stable across SDK versions.

export enum ErrorCode {
  ContextMismatch = 1,
  ThresholdTooLow = 2,
  RootNotTrusted = 3,
  CircuitVersionUnsupported = 4,
  ReceiptInvalid = 5,
  SignatureInvalid = 6,
  JournalDecode = 7,
  InvalidPresenterKey = 8,
  ProgramOwnerMismatch = 9,
  // On-chain-only:
  ChallengeStale = 10,
  ChallengeReused = 11,
  AlreadyInitialized = 12,
}

export class AdmissionDenied extends Error {
  public readonly code: ErrorCode;
  constructor(code: ErrorCode, message?: string) {
    super(message ?? `Admission denied (code=${code})`);
    this.code = code;
    this.name = "AdmissionDenied";
  }
}

export class MessagingError extends Error {
  public readonly kind: "Timeout" | "SessionDropped" | "Malformed" | "TransportFailure";
  constructor(kind: MessagingError["kind"], message?: string) {
    super(message ?? `Messaging error: ${kind}`);
    this.kind = kind;
    this.name = "MessagingError";
  }
}
