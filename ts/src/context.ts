// Context-id helpers. Output is byte-identical to the Rust
// attestation-core::context helpers; the trailing colon on every
// domain string prevents prefix collisions across kinds.

import { sha256 } from "@noble/hashes/sha256";
import type { Hash32 } from "./types";

const DOMAIN_PROGRAM = new TextEncoder().encode("lp-0005:onchain:");
const DOMAIN_CHAT = new TextEncoder().encode("lp-0005:chat:");
const DOMAIN_FEE = new TextEncoder().encode("lp-0005:fee:");
const DOMAIN_GENERIC = new TextEncoder().encode("lp-0005:generic:");

function concat(...parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

function u64leBytes(v: bigint): Uint8Array {
  const out = new Uint8Array(8);
  let x = v;
  for (let i = 0; i < 8; i++) {
    out[i] = Number(x & 0xffn);
    x >>= 8n;
  }
  return out;
}

function u32leBytes(v: number): Uint8Array {
  const out = new Uint8Array(4);
  out[0] = v & 0xff;
  out[1] = (v >>> 8) & 0xff;
  out[2] = (v >>> 16) & 0xff;
  out[3] = (v >>> 24) & 0xff;
  return out;
}

export function contextIdForProgram(programPubkey: Hash32, gateSeed: Hash32): Hash32 {
  return sha256(concat(DOMAIN_PROGRAM, programPubkey, gateSeed));
}

export function contextIdForChat(groupPubkey: Hash32, epoch: bigint): Hash32 {
  return sha256(concat(DOMAIN_CHAT, groupPubkey, u64leBytes(epoch)));
}

export function contextIdForFeeTier(tier: number, groupPubkey: Hash32): Hash32 {
  return sha256(concat(DOMAIN_FEE, u32leBytes(tier), groupPubkey));
}

export function contextIdGeneric(integrationId: string, extra: Uint8Array = new Uint8Array()): Hash32 {
  return sha256(concat(DOMAIN_GENERIC, new TextEncoder().encode(integrationId), extra));
}
