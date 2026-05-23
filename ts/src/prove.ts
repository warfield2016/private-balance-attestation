// Proof generation entrypoint. Two executor backends:
//   - cli (Node): shells out to attestation-cli.
//   - wasm (browser/Node): stubbed; lands when risc0-zkvm's wasm
//     feature is wired through.
// Defaults: cli on Node, wasm in browsers. Override with `executor`.

import { spawn } from "node:child_process";
import { writeFile, readFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import type { ProveOutput, Witness } from "./types";
import { peekJournal } from "./verify";

export interface ProveInput {
  cliPath?: string;
  executor?: "cli" | "wasm";
  threshold: bigint;
  contextId: Uint8Array;
  presenterPk: Uint8Array;
  programOwner: Uint8Array;
  witness: Witness;
  merkleRoot: Uint8Array;
  sequencerUrl?: string;
}

function isBrowser(): boolean {
  return typeof window !== "undefined" && typeof process === "undefined";
}

function toHex(b: Uint8Array): string {
  return Array.from(b).map((x) => x.toString(16).padStart(2, "0")).join("");
}

export async function prove(input: ProveInput): Promise<ProveOutput> {
  const executor = input.executor ?? (isBrowser() ? "wasm" : "cli");
  if (executor === "wasm") {
    throw new Error(
      "WASM prover is not yet available. Use the `cli` executor on Node, or shell out to the attestation-cli binary directly.",
    );
  }
  return proveViaCli(input);
}

async function proveViaCli(input: ProveInput): Promise<ProveOutput> {
  const cli = input.cliPath ?? "attestation-cli";
  const dir = await mkdtemp(join(tmpdir(), "lp0005-"));
  try {
    const witnessPath = join(dir, "witness.json");
    const proofPath = join(dir, "proof.bin");

    const witnessFile = {
      npk: toHex(input.witness.npk),
      program_owner: toHex(input.witness.programOwner),
      balance: Number(input.witness.balance),
      nonce: toHex(input.witness.nonce),
      data: toHex(input.witness.data),
      merkle_root: toHex(input.merkleRoot),
      merkle_siblings: input.witness.merkleSiblings.map(toHex),
      merkle_indices: input.witness.merkleIndices,
      spending_pk: toHex(input.witness.spendingPk),
      spending_pk_offset: Number(input.witness.spendingPkOffset),
    };
    await writeFile(witnessPath, JSON.stringify(witnessFile));

    const args = [
      "prove",
      "--threshold", String(input.threshold),
      "--context-id", `0x${toHex(input.contextId)}`,
      "--presenter-pk", `0x${toHex(input.presenterPk)}`,
      "--program-owner", `0x${toHex(input.programOwner)}`,
      "--witness", witnessPath,
      "--out", proofPath,
      "--json",
    ];
    if (input.sequencerUrl) args.push("--sequencer-url", input.sequencerUrl);

    const result = await spawnCapture(cli, args);
    if (result.code !== 0) {
      throw new Error(`prove failed (exit ${result.code}): ${result.stderr}`);
    }

    const receipt = await readFile(proofPath);
    const summary = JSON.parse(result.stdout) as {
      ok: boolean;
      prove_ms: number;
      receipt_bytes: number;
    };
    if (!summary.ok) {
      // Defensive: the CLI exits non-zero on failure, but a future
      // version emitting {"ok": false} with exit 0 would slip past.
      throw new Error(`prove reported ok=false: ${result.stdout}`);
    }

    const journal = await peekJournal(new Uint8Array(receipt), cli);
    return {
      receipt: new Uint8Array(receipt),
      journalBytes: new Uint8Array(0),
      journal,
      proveMs: summary.prove_ms,
    };
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
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
