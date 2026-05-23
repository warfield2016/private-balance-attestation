// Logos Messaging transport for the off-chain attestation flow.
// Four logical messages — offer, challenge, response, result — wrapped
// in a tagged binary envelope and shipped over a MessagingTransport.
// The transport is an interface so a real Logos client and a mock can
// both implement it.

import { sha256 } from "@noble/hashes/sha256";
import * as ed from "@noble/ed25519";

import type { AttestationBundle, Hash32, Signature64 } from "./types";
import { AdmissionDenied, ErrorCode, MessagingError } from "./errors";

// Wire tags.
export const MSG_OFFER = 0x01;
export const MSG_CHALLENGE = 0x02;
export const MSG_RESPONSE = 0x03;
export const MSG_RESULT = 0x04;

export interface OfferMsg {
  tag: typeof MSG_OFFER;
  receipt: Uint8Array;
  journal: Uint8Array;
  presenterPk: Hash32;
}
export interface ChallengeMsg {
  tag: typeof MSG_CHALLENGE;
  challenge: Hash32;
  actionTag: Uint8Array; // 16 bytes
  expiresAt: bigint;
}
export interface ResponseMsg {
  tag: typeof MSG_RESPONSE;
  signature: Signature64;
}
export interface ResultMsg {
  tag: typeof MSG_RESULT;
  ok: boolean;
  errorCode: number;
  presenterPk: Hash32;
}

export type AnyMsg = OfferMsg | ChallengeMsg | ResponseMsg | ResultMsg;

export interface MessagingTransport {
  send(to: Hash32, payload: Uint8Array): Promise<void>;
  recv(): Promise<{ from: Hash32; payload: Uint8Array }>;
  close(): Promise<void>;
}

export interface OpenSessionOpts {
  transport: MessagingTransport;
  verifierPk: Hash32;
}

export interface AttestationSession {
  send(bundle: AttestationBundle): Promise<void>;
  awaitChallenge(opts?: { timeoutMs?: number }): Promise<ChallengeMsg>;
  respond(signature: Signature64): Promise<void>;
  awaitAdmission(opts?: { timeoutMs?: number }): Promise<ResultMsg>;
}

export async function openAttestationSession(
  opts: OpenSessionOpts
): Promise<AttestationSession> {
  const { transport, verifierPk } = opts;

  return {
    async send(bundle) {
      const payload = encode({
        tag: MSG_OFFER,
        receipt: bundle.receipt,
        journal: new Uint8Array(0),
        presenterPk: bundle.presenterPk,
      });
      await transport.send(verifierPk, payload);
    },

    async awaitChallenge(o) {
      return (await recvOfTag(transport, MSG_CHALLENGE, o?.timeoutMs ?? 30_000)) as ChallengeMsg;
    },

    async respond(signature) {
      await transport.send(verifierPk, encode({ tag: MSG_RESPONSE, signature }));
    },

    async awaitAdmission(o) {
      const r = (await recvOfTag(transport, MSG_RESULT, o?.timeoutMs ?? 30_000)) as ResultMsg;
      if (!r.ok) throw new AdmissionDenied(r.errorCode as ErrorCode);
      return r;
    },
  };
}

export function freshChallenge(): Hash32 {
  const out = new Uint8Array(32);
  if (typeof crypto === "undefined" || !crypto.getRandomValues) {
    throw new Error("crypto.getRandomValues unavailable; cannot generate challenge");
  }
  crypto.getRandomValues(out);
  return out;
}

export async function verifyingKeyFromSeed(seed: Uint8Array): Promise<Hash32> {
  return await ed.getPublicKeyAsync(seed);
}

export async function signChallenge(seed: Uint8Array, challenge: Hash32): Promise<Signature64> {
  return await ed.signAsync(challenge, seed);
}

// Off-chain helper that mirrors the on-chain rebuild_challenge rule.
// Lets verifiers issue domain-separated challenges instead of pure
// randomness when they want to.
export function deterministicChallenge(opts: {
  programId: Hash32;
  slotHashOrRandom: Hash32;
  presenterPk: Hash32;
  actionTag: Uint8Array;
}): Hash32 {
  if (opts.actionTag.length !== 16) throw new Error("actionTag must be 16 bytes");
  const domain = new TextEncoder().encode("lp-0005-challenge-v1");
  const parts = [domain, opts.programId, opts.slotHashOrRandom, opts.presenterPk, opts.actionTag];
  const total = parts.reduce((n, p) => n + p.length, 0);
  const buf = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    buf.set(p, off);
    off += p.length;
  }
  return sha256(buf);
}

// 16-byte action tag with zero-padding. Tests and the on-chain code
// space-pad in some places; either pads consistently as long as both
// sides agree on the bytes, but the SDK ships zero-padding so it's
// distinguishable from accidental ASCII content.
export function actionTag(s: string): Uint8Array {
  const enc = new TextEncoder().encode(s);
  if (enc.length > 16) throw new Error(`action tag '${s}' exceeds 16 bytes`);
  const out = new Uint8Array(16);
  out.set(enc.subarray(0, Math.min(enc.length, 16)));
  return out;
}

// --- wire codec ----------------------------------------------------------

function encode(m: AnyMsg): Uint8Array {
  const enc = new Encoder();
  enc.u8(m.tag);
  switch (m.tag) {
    case MSG_OFFER:
      enc.bytesLen(m.receipt);
      enc.bytesLen(m.journal);
      enc.fixed(m.presenterPk, 32);
      break;
    case MSG_CHALLENGE:
      enc.fixed(m.challenge, 32);
      enc.fixed(m.actionTag, 16);
      enc.u64be(m.expiresAt);
      break;
    case MSG_RESPONSE:
      enc.fixed(m.signature, 64);
      break;
    case MSG_RESULT:
      enc.u8(m.ok ? 1 : 0);
      enc.u8(m.errorCode & 0xff);
      enc.fixed(m.presenterPk, 32);
      break;
  }
  return enc.finish();
}

function decode(buf: Uint8Array): AnyMsg {
  const dec = new Decoder(buf);
  const tag = dec.u8();
  switch (tag) {
    case MSG_OFFER:
      return {
        tag: MSG_OFFER,
        receipt: dec.bytesLen(),
        journal: dec.bytesLen(),
        presenterPk: dec.fixed(32),
      };
    case MSG_CHALLENGE:
      return {
        tag: MSG_CHALLENGE,
        challenge: dec.fixed(32),
        actionTag: dec.fixed(16),
        expiresAt: dec.u64be(),
      };
    case MSG_RESPONSE:
      return { tag: MSG_RESPONSE, signature: dec.fixed(64) };
    case MSG_RESULT:
      return {
        tag: MSG_RESULT,
        ok: dec.u8() === 1,
        errorCode: dec.u8(),
        presenterPk: dec.fixed(32),
      };
    default:
      throw new MessagingError("Malformed", `unknown tag ${tag}`);
  }
}

async function recvOfTag(
  transport: MessagingTransport,
  tag: number,
  timeoutMs: number
): Promise<AnyMsg> {
  const timer = new Promise<never>((_, reject) =>
    setTimeout(() => reject(new MessagingError("Timeout")), timeoutMs)
  );
  const recv = (async () => {
    while (true) {
      const m = await transport.recv();
      const decoded = decode(m.payload);
      if (decoded.tag === tag) return decoded;
      // Unexpected message tag — drop and keep waiting; Logos Messaging
      // is a shared transport, peers can send unrelated traffic.
    }
  })();
  return await Promise.race([recv, timer]);
}

class Encoder {
  private chunks: Uint8Array[] = [];

  u8(v: number) {
    this.chunks.push(Uint8Array.of(v & 0xff));
  }
  fixed(b: Uint8Array, len: number) {
    if (b.length !== len) throw new Error(`fixed: expected ${len} bytes, got ${b.length}`);
    this.chunks.push(b);
  }
  bytesLen(b: Uint8Array) {
    const len = b.length >>> 0;
    this.chunks.push(
      Uint8Array.of(
        (len >>> 24) & 0xff,
        (len >>> 16) & 0xff,
        (len >>> 8) & 0xff,
        len & 0xff,
      ),
    );
    this.chunks.push(b);
  }
  u64be(v: bigint) {
    const out = new Uint8Array(8);
    let x = v;
    for (let i = 7; i >= 0; i--) {
      out[i] = Number(x & 0xffn);
      x >>= 8n;
    }
    this.chunks.push(out);
  }
  finish(): Uint8Array {
    const total = this.chunks.reduce((n, c) => n + c.length, 0);
    const out = new Uint8Array(total);
    let off = 0;
    for (const c of this.chunks) {
      out.set(c, off);
      off += c.length;
    }
    return out;
  }
}

class Decoder {
  private buf: Uint8Array;
  private off = 0;
  constructor(b: Uint8Array) {
    this.buf = b;
  }
  u8(): number {
    if (this.off >= this.buf.length) throw new MessagingError("Malformed", "u8 EOF");
    return this.buf[this.off++];
  }
  fixed(n: number): Uint8Array {
    if (this.off + n > this.buf.length) throw new MessagingError("Malformed", `fixed(${n}) EOF`);
    const out = this.buf.slice(this.off, this.off + n);
    this.off += n;
    return out;
  }
  bytesLen(): Uint8Array {
    if (this.off + 4 > this.buf.length) throw new MessagingError("Malformed", "bytesLen EOF");
    // `>>> 0` keeps the length unsigned; without it a high-bit set
    // would sign-extend and produce a negative length, which slice()
    // silently turns into an empty array.
    const len =
      (((this.buf[this.off] << 24) |
        (this.buf[this.off + 1] << 16) |
        (this.buf[this.off + 2] << 8) |
        this.buf[this.off + 3]) >>>
        0);
    this.off += 4;
    return this.fixed(len);
  }
  u64be(): bigint {
    if (this.off + 8 > this.buf.length) throw new MessagingError("Malformed", "u64 EOF");
    let v = 0n;
    for (let i = 0; i < 8; i++) {
      v = (v << 8n) | BigInt(this.buf[this.off + i]);
    }
    this.off += 8;
    return v;
  }
}
