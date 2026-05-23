# LP-0005 Basecamp app

A self-contained static UI that:

- derives a gate's `context_id` from a domain + inputs (mirrors
  `attestation_core::context` byte-for-byte)
- loads a serialized RISC0 receipt and shows the public journal
  (merkle_root, threshold, context_id, presenter_pk, program_owner,
  circuit_version)
- runs the journal-field-equality checks the on-chain program runs
  (context match, threshold floor, trusted-root membership)

No build step. Drop the folder into Basecamp; everything resolves
client-side. Cryptographic verification (RISC0 receipt + ed25519
signature) is left to `attestation-cli verify` or the SDK's
`verify_attestation`, which the app's output gives you the exact
command for.

## Files

```
basecamp-app/
├── manifest.json        # app metadata for Basecamp loader
├── index.html
├── README.md            # this file
├── src/
│   ├── app.js           # ES module; uses @noble/hashes from CDN
│   └── style.css
└── assets/
    ├── icon-32.png
    └── icon-128.png
```

## Local preview

```
cd basecamp-app
python3 -m http.server 5050     # or any static server
open http://localhost:5050
```

The page works in any modern browser; the CDN imports use ES modules.

## Loading into Basecamp

Per Basecamp's sideload instructions:

```
Logos Basecamp → Apps → Sideload → choose this folder
```

The app shows up as **LP-0005 Attestation**. The `permissions` in
`manifest.json` request the wallet, Logos Messaging, and the LEZ RPC
scopes — granted at first launch.

## Downloadable bundle

`make bundle` (or `zip -r basecamp-app.zip basecamp-app`) packages
the folder for distribution. The bundle is the artefact evaluators
load.
