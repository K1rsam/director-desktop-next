# Director official Codex SDK bridge

Private Node ESM sidecar using the real, pinned `@openai/codex-sdk@0.160.1` package. This package is not imported into React. Live execution is disabled; no flag enables it. CLI remains the first native acceptance path.

```sh
npm ci --ignore-scripts --omit=optional --no-audit --no-fund
npm run verify
node src/main.mjs --state-dir /absolute/dedicated/worker-directory
```

The default process permits zero-runtime hello/inspect and local receipt observation. Execute fails with `SDK_LIVE_DISABLED`. `--offline-fixture happy` runs only a bundled synthetic executable through the real SDK, without model/network/account calls. The fixture launcher is POSIX-only.

Protocol and Rust mapping: [SDK_BRIDGE.md](../docs/SDK_BRIDGE.md). Package hashes and complete test log are in `verification/`. Live/native limitations are release blockers, not hidden fallback paths.
