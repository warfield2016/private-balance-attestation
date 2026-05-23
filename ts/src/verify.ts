// Off-chain verification. Shells out to attestation-cli, which uses
// the Rust attestation-verifier crate. A WASM build of the same crate
// will replace this in a later version so verification runs in-process.

import { spawn } from "node:child_process";
import { writeFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import type { Hash32, JournalFields, Signature64 } from "./types";
import { AdmissionDenied, ErrorCode } from "./errors";

export interface VerifyArgs {
  receipt: Uint8Array;
  expectedContextId: Hash32;
  expectedProgramOwner: Hash32;
  trustedRoots: Hash32[];
  minimumThreshold: bigint;
  challenge: Hash32;
  signature: Signature64;
  cliPath?: string;
}

// Decode the journal without verifying the receipt. Useful for "you're
// about to authenticate as X" UI flows; never a substitute for verify().
export async function peekJournal(receipt: Uint8Array, cliPath = "attestation-cli"): Promise<JournalFields> {
  const dir = await mkdtemp(join(tmpdir(), "lp0005-peek-"));
  try {
    const proofPath = join(dir, "proof.bin");
    await writeFile(proofPath, receipt);
    const result = await spawnCapture(cliPath, ["inspect", "--proof", proofPath, "--json"]);
    if (result.code !== 0) throw new Error(`peekJournal failed: ${result.stderr}`);
    return parseJournal(JSON.parse(result.stdout));
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

// Full verification. Throws AdmissionDenied on failure.
export async function verify(args: VerifyArgs): Promise<JournalFields> {
  const cli = args.cliPath ?? "attestation-cli";
  const dir = await mkdtemp(join(tmpdir(), "lp0005-verify-"));
  try {
    const proofPath = join(dir, "proof.bin");
    const challengePath = join(dir, "challenge.bin");
    const sigPath = join(dir, "sig.bin");
    const rootsPath = join(dir, "roots.bin");

    await Promise.all([
      writeFile(proofPath, args.receipt),
      writeFile(challengePath, args.challenge),
      writeFile(sigPath, args.signature),
      writeFile(rootsPath, concat(args.trustedRoots)),
    ]);

    const result = await spawnCapture(cli, [
      "verify",
      "--proof", proofPath,
      "--context-id", `0x${bytesToHex(args.expectedContextId)}`,
      "--program-owner", `0x${bytesToHex(args.expectedProgramOwner)}`,
      "--challenge", challengePath,
      "--signature", sigPath,
      "--trusted-roots", rootsPath,
      "--min-threshold", String(args.minimumThreshold),
      "--json",
    ]);

    if (result.code === 0) {
      return parseJournal(JSON.parse(result.stdout));
    }
    // Non-zero exit: parse the JSON error envelope from stdout. If the
    // CLI didn't emit JSON (it crashed or exited from a different
    // path), fall back to a generic error from stderr.
    try {
      const parsed = JSON.parse(result.stdout) as { ok: false; error_code: number };
      throw new AdmissionDenied(parsed.error_code as ErrorCode, `code=${parsed.error_code}`);
    } catch (e) {
      if (e instanceof AdmissionDenied) throw e;
      throw new Error(`verify failed: ${result.stderr || result.stdout}`);
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

// --- helpers -------------------------------------------------------------

function parseJournal(raw: {
  merkle_root: number[] | string;
  threshold: number | string;
  context_id: number[] | string;
  presenter_pk: number[] | string;
  program_owner: number[] | string;
  circuit_version: number;
}): JournalFields {
  return {
    merkleRoot: toBytes(raw.merkle_root),
    threshold: BigInt(raw.threshold),
    contextId: toBytes(raw.context_id),
    presenterPk: toBytes(raw.presenter_pk),
    programOwner: toBytes(raw.program_owner),
    circuitVersion: raw.circuit_version,
  };
}

function bytesToHex(b: Uint8Array): string {
  return Array.from(b).map((x) => x.toString(16).padStart(2, "0")).join("");
}

function toBytes(v: number[] | string): Uint8Array {
  if (Array.isArray(v)) return Uint8Array.from(v);
  const s = v.startsWith("0x") ? v.slice(2) : v;
  const out = new Uint8Array(s.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(s.substr(i * 2, 2), 16);
  }
  return out;
}

function concat(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

function spawnCapture(
  cmd: string,
  args: string[]
): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args);
    let stdout = "";
    let stderr = "";
    p.stdout.on("data", (d: Buffer) => {
      stdout += d.toString();
    });
    p.stderr.on("data", (d: Buffer) => {
      stderr += d.toString();
    });
    p.on("error", reject);
    p.on("close", (code: number | null) => resolve({ code: code ?? -1, stdout, stderr }));
  });
}
