// Tiny CLI driver for the fee-tier scaffold.
// Usage:
//   node --import tsx ./src/cli.ts \
//        --proof <path> \
//        --tier-table <path> \
//        --challenge <path> \
//        --signature <path>
// Prints "Tier awarded: <N> (threshold satisfied: <T>)" on success or
// "Rejected: <code>" on failure.

import * as fs from "node:fs/promises";
import { parseArgs } from "node:util";

import { AdmissionDenied, ErrorCode } from "@lp-0005/sdk";
import { verifyTierAttestation, type TierThreshold } from "./lib.js";

interface TierTable {
  groupPk: string;        // hex
  programOwner: string;   // hex
  trustedRoots: string[]; // hex
  tiers: { tier: number; minThreshold: string }[];
}

function hexToBytes(s: string): Uint8Array {
  const t = s.startsWith("0x") ? s.slice(2) : s;
  const out = new Uint8Array(t.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(t.substr(i * 2, 2), 16);
  return out;
}

const { values } = parseArgs({
  options: {
    proof: { type: "string" },
    "tier-table": { type: "string" },
    challenge: { type: "string" },
    signature: { type: "string" },
  },
});

for (const k of ["proof", "tier-table", "challenge", "signature"] as const) {
  if (!values[k]) {
    console.error(`missing --${k}`);
    process.exit(2);
  }
}

const receipt = await fs.readFile(values.proof!);
const table: TierTable = JSON.parse(await fs.readFile(values["tier-table"]!, "utf8"));
const challenge = await fs.readFile(values.challenge!);
const signature = await fs.readFile(values.signature!);

const thresholds: TierThreshold[] = table.tiers.map((t) => ({
  tier: t.tier,
  minThreshold: BigInt(t.minThreshold),
}));

try {
  const r = await verifyTierAttestation(new Uint8Array(receipt), thresholds, {
    groupPk: hexToBytes(table.groupPk),
    programOwner: hexToBytes(table.programOwner),
    trustedRoots: table.trustedRoots.map(hexToBytes),
    challenge: new Uint8Array(challenge),
    signature: new Uint8Array(signature),
  });
  console.log(`Tier awarded: ${r.awardedTier} (threshold satisfied: ${r.thresholdSatisfied})`);
} catch (e: unknown) {
  if (e instanceof AdmissionDenied) {
    console.error(`Rejected: code=${e.code} (${ErrorCode[e.code] ?? "unknown"})`);
    process.exit(3);
  }
  console.error(`Rejected: ${(e as Error).message}`);
  process.exit(3);
}
