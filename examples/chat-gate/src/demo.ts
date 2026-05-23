// End-to-end chat-gate demo. Spawns the prover side and the host side
// against a paired in-process transport, runs the four-message flow,
// prints the admission result.
//
// Usage:
//   node --import tsx ./src/demo.ts \
//        --witness ./keys/alice/witness.json \
//        --spending-key ./keys/alice/spending.bin \
//        --group-pk <hex>
//
// Real Logos Messaging deployments swap InProcessTransport for a Logos
// client; the rest of the flow is unchanged.

import { spawn } from "node:child_process";
import * as fs from "node:fs/promises";
import { parseArgs } from "node:util";

import * as ed from "@noble/ed25519";
import {
  actionTag,
  contextIdForChat,
  ErrorCode,
  freshChallenge,
  openAttestationSession,
  signChallenge,
} from "@lp-0005/sdk";
import { verify } from "@lp-0005/sdk/verify";
import { InProcessTransport } from "./mock-transport.js";

const { values } = parseArgs({
  options: {
    "group-pk": { type: "string" },
    "program-owner": { type: "string" },
    witness: { type: "string" },
    "spending-key": { type: "string" },
    threshold: { type: "string", default: "100" },
    epoch: { type: "string", default: "1" },
  },
});

function fail(msg: string): never {
  console.error(msg);
  process.exit(2);
}

for (const k of ["group-pk", "program-owner", "witness", "spending-key"] as const) {
  if (!values[k]) fail(`missing --${k}`);
}

function hexToBytes(s: string): Uint8Array {
  const t = s.startsWith("0x") ? s.slice(2) : s;
  const out = new Uint8Array(t.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(t.substr(i * 2, 2), 16);
  return out;
}

const groupPk = hexToBytes(values["group-pk"]!);
const programOwner = hexToBytes(values["program-owner"]!);
const seed = await fs.readFile(values["spending-key"]!);
const presenterPk = await ed.getPublicKeyAsync(seed);
const ctxId = contextIdForChat(groupPk, BigInt(values.epoch!));

console.error("Proving (this will take ~60s with RISC0_DEV_MODE=0)…");
const proofPath = "out/chat-demo.proof";
await fs.mkdir("out", { recursive: true });
await runSync("attestation-cli", [
  "prove",
  "--threshold", values.threshold!,
  "--context-id", `0x${toHex(ctxId)}`,
  "--presenter-pk", `0x${toHex(presenterPk)}`,
  "--program-owner", `0x${toHex(programOwner)}`,
  "--witness", values.witness!,
  "--out", proofPath,
]);
const receipt = new Uint8Array(await fs.readFile(proofPath));

const [proverT, hostT] = InProcessTransport.pair(presenterPk, groupPk);
const trustedRoots: Uint8Array[] = []; // populate with sequencer-anchored roots

const hostTask = (async () => {
  const offer = await hostT.recv();
  const challenge = freshChallenge();
  const tag = actionTag("enter-group");

  // Send the challenge.
  await hostT.send(presenterPk, encodeChallenge(challenge, tag));
  const respMsg = await hostT.recv();
  const sig = respMsg.payload.slice(1, 65);

  try {
    const journal = await verify({
      receipt: parseOfferReceipt(offer.payload),
      expectedContextId: ctxId,
      expectedProgramOwner: programOwner,
      trustedRoots,
      minimumThreshold: BigInt(values.threshold!),
      challenge,
      signature: sig,
    });
    await hostT.send(presenterPk, encodeResult(true, 0, journal.presenterPk));
  } catch (e) {
    const code = (e as { code?: number }).code ?? ErrorCode.JournalDecode;
    await hostT.send(presenterPk, encodeResult(false, code, presenterPk));
  }
})();

const proverTask = (async () => {
  const session = await openAttestationSession({ transport: proverT, verifierPk: groupPk });
  await session.send({
    receipt,
    journal: {
      merkleRoot: new Uint8Array(32),
      threshold: 0n,
      contextId: ctxId,
      presenterPk,
      programOwner,
      circuitVersion: 2,
    },
    presenterPk,
  });
  const challenge = await session.awaitChallenge();
  const sig = await signChallenge(seed, challenge.challenge);
  await session.respond(sig);
  return await session.awaitAdmission();
})();

const [, result] = await Promise.all([hostTask, proverTask]);
if (result.ok) {
  console.log(`Admitted: 0x${toHex(result.presenterPk)}`);
} else {
  console.error(`Admission denied: code=${result.errorCode}`);
  process.exit(3);
}

// --- helpers -------------------------------------------------------------

function toHex(b: Uint8Array): string {
  return Array.from(b).map((x) => x.toString(16).padStart(2, "0")).join("");
}

function encodeChallenge(challenge: Uint8Array, tag: Uint8Array): Uint8Array {
  const out = new Uint8Array(1 + 32 + 16 + 8);
  out[0] = 0x02;
  out.set(challenge, 1);
  out.set(tag, 33);
  // expires_at = 0 for the demo
  return out;
}

function encodeResult(ok: boolean, errorCode: number, presenterPk: Uint8Array): Uint8Array {
  const out = new Uint8Array(1 + 1 + 1 + 32);
  out[0] = 0x04;
  out[1] = ok ? 1 : 0;
  out[2] = errorCode & 0xff;
  out.set(presenterPk, 3);
  return out;
}

function parseOfferReceipt(buf: Uint8Array): Uint8Array {
  if (buf[0] !== 0x01) throw new Error(`bad tag ${buf[0]}`);
  let off = 1;
  const len = ((buf[off] << 24) | (buf[off + 1] << 16) | (buf[off + 2] << 8) | buf[off + 3]) >>> 0;
  return buf.slice(off + 4, off + 4 + len);
}

function runSync(cmd: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: "inherit" });
    p.on("error", reject);
    p.on("close", (code) =>
      code === 0 ? resolve() : reject(new Error(`${cmd} exited ${code}`))
    );
  });
}
