// Fee-tier integration core. Pure verification + mapping logic; the
// CLI wraps it in IO. Integrators replace the in-memory tier table
// with whatever their service uses.

import {
  contextIdForFeeTier,
  type Hash32,
  type JournalFields,
} from "@lp-0005/sdk";
import { verify } from "@lp-0005/sdk/verify";

export interface TierThreshold {
  // Higher tier values are better.
  tier: number;
  // Minimum balance threshold for this tier.
  minThreshold: bigint;
}

export interface TierVerifyOpts {
  // Service's group / project pubkey, folded into the context-id.
  groupPk: Hash32;
  // LEZ token program the gate accepts attestations from.
  programOwner: Hash32;
  // Recent trusted Merkle roots (e.g., last 8 anchored roots).
  trustedRoots: Hash32[];
  // Verifier-issued challenge.
  challenge: Hash32;
  // 64-byte presenter signature over the challenge.
  signature: Uint8Array;
}

export interface TierVerifyResult {
  awardedTier: number;
  thresholdSatisfied: bigint;
  presenterPk: Hash32;
  journal: JournalFields;
}

// Verify an attestation against the fee-tier context and award the
// tier the prover targeted. Each tier has its own context-id, so the
// prover commits to a specific tier at proof-gen time; over-awarding
// is not possible. Throws AdmissionDenied on verification failure.
export async function verifyTierAttestation(
  receipt: Uint8Array,
  thresholds: TierThreshold[],
  opts: TierVerifyOpts
): Promise<TierVerifyResult> {
  const sorted = [...thresholds].sort((a, b) => Number(a.minThreshold - b.minThreshold));

  for (let i = sorted.length - 1; i >= 0; i--) {
    const { tier, minThreshold } = sorted[i];
    const ctx = contextIdForFeeTier(tier, opts.groupPk);
    try {
      const journal = await verify({
        receipt,
        expectedContextId: ctx,
        expectedProgramOwner: opts.programOwner,
        trustedRoots: opts.trustedRoots,
        minimumThreshold: minThreshold,
        challenge: opts.challenge,
        signature: opts.signature,
      });
      return {
        awardedTier: tier,
        thresholdSatisfied: journal.threshold,
        presenterPk: journal.presenterPk,
        journal,
      };
    } catch {
      // The proof didn't target this tier; try the next-lower one.
      continue;
    }
  }
  throw new Error("no tier matched — proof failed verification or targeted an unknown tier");
}

// Which tier does a balance qualify for, ignoring proofs?
export function tierForBalance(
  thresholds: TierThreshold[],
  balance: bigint
): number | null {
  let best: number | null = null;
  for (const t of thresholds) {
    if (balance >= t.minThreshold && (best === null || t.tier > best)) {
      best = t.tier;
    }
  }
  return best;
}
