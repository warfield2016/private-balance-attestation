// @lp-0005/sdk — public surface.

export * from "./types";
export * from "./errors";
export * from "./context";
export { prove } from "./prove";
export { verify, peekJournal } from "./verify";
export {
  openAttestationSession,
  type AttestationSession,
  type OpenSessionOpts,
  type MessagingTransport,
  type AnyMsg,
  type OfferMsg,
  type ChallengeMsg,
  type ResponseMsg,
  type ResultMsg,
  freshChallenge,
  signChallenge,
  verifyingKeyFromSeed,
  actionTag,
  deterministicChallenge,
} from "./messaging";
