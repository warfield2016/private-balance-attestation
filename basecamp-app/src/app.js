// LP-0005 Basecamp app. Static ES module — no bundler, no build.
// Imports @noble/hashes and @noble/ed25519 from pinned CDNs so the
// deploy is one folder. CSP in vercel.json whitelists jsdelivr.

import { sha256 } from "https://cdn.jsdelivr.net/npm/@noble/hashes@1.4.0/+esm";
import * as ed from "https://cdn.jsdelivr.net/npm/@noble/ed25519@2.1.0/+esm";

// @noble/ed25519 v2 needs a sync sha512 set up explicitly.
import { sha512 } from "https://cdn.jsdelivr.net/npm/@noble/hashes@1.4.0/sha512/+esm";
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));

// --- DOM ----------------------------------------------------------------

const $ = (id) => document.getElementById(id);
const domainSelect    = $("domain");
const contextOut      = $("context-id");
const keyLabel        = $("key-label");
const keyInput        = $("key");
const seedInput       = $("seed");
const epochInput      = $("epoch");
const tierInput       = $("tier");
const integrationIn   = $("integration");
const fileInput       = $("receipt-file");
const journalView     = $("journal-view");
const trustedRootsIn  = $("trusted-roots");
const minThresholdIn  = $("min-threshold");
const verifyBtn       = $("verify-btn");
const verifyOut       = $("verify-result");
const sampleBtn       = $("sample-btn");
const mRoot           = $("m-root");
const mThreshold      = $("m-threshold");
const mVersion        = $("m-version");
const mContext        = $("m-context");
const mPresenter      = $("m-presenter");
const mOwner          = $("m-owner");
const kpPublic        = $("kp-public");
const kpSecret        = $("kp-secret");
const kpChallenge     = $("kp-challenge");
const kpSignature     = $("kp-signature");
const kpGen           = $("kp-gen");
const kpSample        = $("kp-sample");
const kpSign          = $("kp-sign");
const kpUse           = $("kp-use");

// State the verifier reads. Either the manual tab populates it or the
// upload tab parses it out of a receipt.
let currentJournal = null;

// --- hex / byte helpers -------------------------------------------------

function hexToBytes(s) {
  if (!s) return new Uint8Array();
  const clean = (s.startsWith("0x") ? s.slice(2) : s).trim();
  if (clean.length % 2 !== 0) throw new Error("hex string has odd length");
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return out;
}
function bytesToHex(b) {
  return Array.from(b).map((x) => x.toString(16).padStart(2, "0")).join("");
}
function utf8(s) { return new TextEncoder().encode(s); }
function concat(...parts) {
  const total = parts.reduce((n, p) => n + p.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const p of parts) { out.set(p, off); off += p.length; }
  return out;
}
function u64le(v) {
  const out = new Uint8Array(8);
  let x = BigInt(v);
  for (let i = 0; i < 8; i++) { out[i] = Number(x & 0xffn); x >>= 8n; }
  return out;
}
function u32le(v) {
  const out = new Uint8Array(4);
  out[0] =  v        & 0xff;
  out[1] = (v >>> 8) & 0xff;
  out[2] = (v >>> 16) & 0xff;
  out[3] = (v >>> 24) & 0xff;
  return out;
}

// --- context-id (mirrors attestation_core::context, byte-for-byte) ----

const DOMAINS = {
  program: utf8("lp-0005:onchain:"),
  chat:    utf8("lp-0005:chat:"),
  fee:     utf8("lp-0005:fee:"),
  generic: utf8("lp-0005:generic:"),
};

function contextId(kind) {
  switch (kind) {
    case "program": return sha256(concat(DOMAINS.program, hexToBytes(keyInput.value), hexToBytes(seedInput.value)));
    case "chat":    return sha256(concat(DOMAINS.chat,    hexToBytes(keyInput.value), u64le(BigInt(epochInput.value || "1"))));
    case "fee":     return sha256(concat(DOMAINS.fee,     u32le(parseInt(tierInput.value || "1", 10)), hexToBytes(keyInput.value)));
    case "generic": return sha256(concat(DOMAINS.generic, utf8(integrationIn.value || "")));
  }
}

function refreshDomain() {
  const kind = domainSelect.value;
  document.querySelectorAll("[data-domain]").forEach((el) => {
    el.style.display = el.dataset.domain.includes(kind) ? "" : "none";
  });
  keyLabel.textContent =
    kind === "program" ? "Program pubkey" :
    kind === "chat"    ? "Group pubkey" :
    kind === "fee"     ? "Group pubkey" : "Key";
  refreshContextId();
}

function refreshContextId() {
  try {
    const id = contextId(domainSelect.value);
    contextOut.textContent = "0x" + bytesToHex(id);
  } catch {
    contextOut.textContent = "(fill in the inputs above)";
  }
}

domainSelect.addEventListener("change", refreshDomain);
[keyInput, seedInput, epochInput, tierInput, integrationIn].forEach((el) => {
  el.addEventListener("input", refreshContextId);
});
refreshDomain();

// --- tab handling -------------------------------------------------------

function activateTab(name) {
  document.querySelectorAll(".tab").forEach((t) => {
    if (t.dataset.tab) t.classList.toggle("tab-active", t.dataset.tab === name);
  });
  document.querySelectorAll(".tab-panel").forEach((p) => {
    p.classList.toggle("tab-panel-active", p.id === `tab-${name}`);
  });
}
document.querySelectorAll(".tab[data-tab]").forEach((t) => {
  t.addEventListener("click", () => activateTab(t.dataset.tab));
});

// --- manual mode -> currentJournal --------------------------------------

function readManualJournal() {
  // Each field defaults to 32 zero bytes if empty; threshold/version
  // default to numeric zero. This lets a user partially fill the form
  // and still get a meaningful "merkle_root not in trusted set" or
  // "context_id mismatch" response.
  const pad32 = (s) => {
    const bytes = hexToBytes(s);
    const out = new Uint8Array(32);
    out.set(bytes.subarray(0, 32));
    return out;
  };
  return {
    merkle_root:     pad32(mRoot.value),
    threshold:       BigInt(mThreshold.value || "0"),
    context_id:      pad32(mContext.value),
    presenter_pk:    pad32(mPresenter.value),
    program_owner:   pad32(mOwner.value),
    circuit_version: parseInt(mVersion.value || "2", 10),
    _source:         "manual",
  };
}

function renderJournal(j) {
  journalView.classList.remove("placeholder");
  journalView.textContent =
    `merkle_root     = 0x${bytesToHex(j.merkle_root)}\n` +
    `threshold       = ${j.threshold}\n` +
    `context_id      = 0x${bytesToHex(j.context_id)}\n` +
    `presenter_pk    = 0x${bytesToHex(j.presenter_pk)}\n` +
    `program_owner   = 0x${bytesToHex(j.program_owner)}\n` +
    `circuit_version = ${j.circuit_version}` +
    (j._source ? `\n\n(source: ${j._source})` : "");
}

[mRoot, mThreshold, mContext, mPresenter, mOwner, mVersion].forEach((el) => {
  el.addEventListener("input", () => {
    currentJournal = readManualJournal();
    renderJournal(currentJournal);
  });
});

// --- "Load sample" — populates manual fields with a plausible journal ---

const SAMPLE = {
  merkle_root:   "0x1f2e3d4c5b6a79889788776655443322110011223344556677889900aabbccdd",
  threshold:     "1000",
  context_id:    "",  // computed from the current domain selection
  presenter_pk:  "0x8a8b8c8d8e8f909192939495969798999a9b9c9d9e9f0102030405060708090a",
  program_owner: "0xdeadbeefcafebabe1111222233334444555566667777888899990000aaaabbbb",
  circuit_version: "2",
};

sampleBtn.addEventListener("click", async () => {
  // Auto-fill the gate inputs first so context_id has something to hash.
  if (!keyInput.value) keyInput.value = "0x" + "11".repeat(32);
  if (!seedInput.value) seedInput.value = "0x" + "22".repeat(32);
  refreshDomain();
  // Set up the sample keypair so presenter_pk has a valid value.
  await setKeypair(hexToBytes(SAMPLE_SEED_HEX));
  // Pre-fill a challenge so "Sign challenge" works in one click.
  if (!kpChallenge.value) kpChallenge.value = "0x" + "ab".repeat(32);
  // Then populate the journal manual tab.
  mRoot.value      = SAMPLE.merkle_root;
  mThreshold.value = SAMPLE.threshold;
  mContext.value   = contextOut.textContent.startsWith("0x") ? contextOut.textContent : SAMPLE.merkle_root;
  mPresenter.value = kpPublic.value; // ← presenter_pk = sample-seed's public key
  mOwner.value     = SAMPLE.program_owner;
  mVersion.value   = SAMPLE.circuit_version;
  if (!trustedRootsIn.value) trustedRootsIn.value = SAMPLE.merkle_root;
  if (!minThresholdIn.value || minThresholdIn.value === "0") minThresholdIn.value = "100";
  activateTab("manual");
  currentJournal = readManualJournal();
  renderJournal(currentJournal);
});

// --- upload mode ---------------------------------------------------------

fileInput.addEventListener("change", async () => {
  if (!fileInput.files || fileInput.files.length === 0) return;
  const buf = new Uint8Array(await fileInput.files[0].arrayBuffer());
  try {
    const j = parseJournal(buf);
    j._source = `file: ${fileInput.files[0].name}`;
    currentJournal = j;
    renderJournal(j);
  } catch (e) {
    journalView.classList.add("placeholder");
    journalView.textContent = `Could not parse: ${e.message}`;
    currentJournal = null;
  }
});

// Scan for a 140-byte JournalFields section. Heuristic: a sane
// circuit_version sits in 1..=256 at offset off+136.
function parseJournal(buf) {
  for (let off = 0; off <= buf.length - 140; off++) {
    const j = tryParse(buf, off);
    if (j) return j;
  }
  throw new Error("no JournalFields-shaped section found");
}
function tryParse(buf, off) {
  const merkle_root   = buf.slice(off,        off + 32);
  const thresholdLE   = buf.slice(off + 32,   off + 40);
  const context_id    = buf.slice(off + 40,   off + 72);
  const presenter_pk  = buf.slice(off + 72,   off + 104);
  const program_owner = buf.slice(off + 104,  off + 136);
  const ver           = buf.slice(off + 136,  off + 140);
  const verNum = ver[0] | (ver[1] << 8) | (ver[2] << 16) | (ver[3] << 24);
  if (verNum < 1 || verNum > 256) return null;
  let threshold = 0n;
  for (let i = 7; i >= 0; i--) threshold = (threshold << 8n) | BigInt(thresholdLE[i]);
  return { merkle_root, threshold, context_id, presenter_pk, program_owner, circuit_version: verNum };
}

// --- presenter keypair + signing (ed25519) -------------------------------
//
// ed25519 here matches the on-chain handler's signature check exactly:
// the on-chain dispatcher calls `vk.verify(challenge, sig)` via
// `ed25519-dalek`; we call `ed.verify(sig, challenge, vk)` via
// `@noble/ed25519`. Same algorithm, same outputs.

// 32-byte randomness from the browser CSPRNG.
function randomSeed() {
  const seed = new Uint8Array(32);
  crypto.getRandomValues(seed);
  return seed;
}

// A clearly-fake but stable seed for "Use sample seed". Useful for the
// demo because the resulting pubkey is deterministic — paste it into
// the manual presenter_pk and the verifier accepts it every time.
const SAMPLE_SEED_HEX =
  "0101010101010101010101010101010101010101010101010101010101010101";

async function setKeypair(seed) {
  const pub = await ed.getPublicKeyAsync(seed);
  kpSecret.value = "0x" + bytesToHex(seed);
  kpPublic.value = "0x" + bytesToHex(pub);
  // Wipe any stale signature so the user re-signs after a key change.
  kpSignature.value = "";
}

kpGen.addEventListener("click", async () => {
  await setKeypair(randomSeed());
});

kpSample.addEventListener("click", async () => {
  await setKeypair(hexToBytes(SAMPLE_SEED_HEX));
});

kpSign.addEventListener("click", async () => {
  try {
    if (!kpSecret.value) throw new Error("generate or paste a secret seed first");
    if (!kpChallenge.value) throw new Error("provide a 32-byte challenge");
    const seed = hexToBytes(kpSecret.value);
    const challenge = hexToBytes(kpChallenge.value);
    if (challenge.length !== 32) throw new Error("challenge must be exactly 32 bytes");
    const sig = await ed.signAsync(challenge, seed);
    kpSignature.value = "0x" + bytesToHex(sig);
  } catch (e) {
    kpSignature.value = `error: ${e.message}`;
  }
});

kpUse.addEventListener("click", () => {
  if (!kpPublic.value) return;
  mPresenter.value = kpPublic.value;
  activateTab("manual");
  currentJournal = readManualJournal();
  renderJournal(currentJournal);
});

// --- verify --------------------------------------------------------------

verifyBtn.addEventListener("click", async () => {
  verifyOut.classList.remove("placeholder", "row-ok", "row-err");
  verifyOut.textContent = "Running…";
  try {
    if (!currentJournal) currentJournal = readManualJournal();
    const j = currentJournal;

    // --- 1. context_id equality -------------------------------------
    const expectedCtx = contextId(domainSelect.value);
    if (bytesToHex(j.context_id) !== bytesToHex(expectedCtx)) {
      throw new Error(`code 1 (ContextMismatch): journal.context_id ≠ gate context_id`);
    }

    // --- 2. threshold floor -----------------------------------------
    const floor = BigInt(minThresholdIn.value || "0");
    if (j.threshold < floor) {
      throw new Error(`code 2 (ThresholdTooLow): journal threshold ${j.threshold} below floor ${floor}`);
    }

    // --- 3. trusted-roots membership --------------------------------
    const roots = trustedRootsIn.value.split(/\s+/).filter(Boolean).map(hexToBytes);
    if (roots.length > 0) {
      const hex = bytesToHex(j.merkle_root);
      const found = roots.some((r) => bytesToHex(r) === hex);
      if (!found) {
        throw new Error(`code 3 (RootNotTrusted): journal.merkle_root not in trusted set`);
      }
    }

    // --- 4. circuit_version allow-list ------------------------------
    if (j.circuit_version !== 2) {
      throw new Error(`code 4 (CircuitVersionUnsupported): journal version ${j.circuit_version}, expected 2`);
    }

    // --- 5. ed25519 signature check (when sig + challenge provided) -
    let sigStatus;
    const sigHex = kpSignature.value.trim();
    const chHex  = kpChallenge.value.trim();
    if (sigHex && chHex && !sigHex.startsWith("error")) {
      try {
        const sig = hexToBytes(sigHex);
        const ch  = hexToBytes(chHex);
        if (sig.length !== 64) throw new Error("signature must be 64 bytes");
        if (ch.length !== 32)  throw new Error("challenge must be 32 bytes");
        const ok = await ed.verifyAsync(sig, ch, j.presenter_pk);
        if (!ok) {
          throw new Error(`code 6 (SignatureInvalid): ed25519 signature does not verify against journal.presenter_pk`);
        }
        sigStatus =
          `signature     ✓ ed25519 verify OK against presenter_pk\n` +
          `challenge     = 0x${bytesToHex(ch)}`;
      } catch (e) {
        if (e.message.startsWith("code ")) throw e;
        throw new Error(`signature check error: ${e.message}`);
      }
    } else {
      sigStatus =
        `signature     ⚠ skipped — no challenge+signature provided\n` +
        `              (use Step 3 above to sign a challenge as the presenter)`;
    }

    // --- 6. RISC0 receipt-crypto verify (CLI only) -----------------
    verifyOut.classList.add("row-ok");
    verifyOut.textContent =
      `OK — public-side verifier passes.\n\n` +
      `presenter_pk  = 0x${bytesToHex(j.presenter_pk)}\n` +
      `program_owner = 0x${bytesToHex(j.program_owner)}\n` +
      `threshold     = ${j.threshold}\n` +
      `${sigStatus}\n\n` +
      `RISC0 receipt cryptographic verify needs the CLI:\n\n` +
      `  attestation-cli verify \\\n` +
      `      --proof <file> \\\n` +
      `      --context-id 0x${bytesToHex(j.context_id)} \\\n` +
      `      --program-owner 0x${bytesToHex(j.program_owner)} \\\n` +
      `      --challenge ./challenge.bin \\\n` +
      `      --signature ./sig.bin \\\n` +
      `      --trusted-roots ./roots.bin`;
  } catch (e) {
    verifyOut.classList.add("row-err");
    verifyOut.textContent = `Rejected: ${e.message}`;
  }
});
