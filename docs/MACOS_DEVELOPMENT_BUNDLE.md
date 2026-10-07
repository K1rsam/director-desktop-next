# M5 macOS development bundle signature

M4 `3c6c7c30d8b6af90ab201bee62e995f485201298` native inspection found that its raw and copied apps had identical files and permissions, and both lacked `Contents/_CodeSignature/CodeResources`. Strict bundle verification failed for both. This supports a missing bundle-signing step, not resource loss during that copy. The separately observed `SIGABRT` at `_RegisterApplication` / `NSApplication` remains unexplained; a valid signature alone must not be reported as fixing it.

## Build and safety contract

From `apps/desktop`, use `npm run desktop:build:mac-dev` for the approved local development build. The existing ordinary build and Windows behavior are unchanged. This command:

1. Requires macOS and native arm64 or x64 Node, rejects extra arguments, and checks installed Tauri CLI version 2.12.1
2. Allocates a fresh target directory under this new checkout's `.local/macos-dev-target-*`, so no pre-existing descendant symlinks can redirect build output; it does not reuse or sign the frozen M4 bundle
3. Runs the pinned Tauri CLI with `desktop,custom-protocol`, the explicit native target, app-only bundle, `--no-sign`, and `--ci`
4. Removes all `APPLE_*` variables from the child build environment, without changing the caller's environment or printing their values
5. Checks the new app's identity, executable inventory and exact seven SDK resources. Only the declared `director-desktop-next` main binary and optional `director-next-core` helper are supported. Unexpected native code, symlinks or frameworks stop the step
6. Compiles the bounded descriptor-based attribute collector, records byte-hash observations for files/directories/root, signs the known helper first when present, then signs the outer app using fixed `/usr/bin/codesign --force --sign - --options runtime` arguments. It does not use deep signing or any certificate
7. Compares deterministic attribute records under the [bounded M6.1 policy](MACOS_XATTR_EVIDENCE.md), persisting sanitized evidence even on failure, verifies the signature/resources, then rechecks attributes and the exact artifact inventory. A failure stops the command; there is no automatic repair or retry

Why `--no-sign`: Tauri's built-in signing branch clears extended attributes before signing. The approved scope excludes removing quarantine. Its pinned `--no-sign` branch skips keychain handling, xattr removal, signing and notarization. The wrapper applies the ad-hoc signature directly without an attribute-clearing command or operating-system security change; structured snapshots determine whether existing attributes actually remained unchanged. Hardened runtime remains required. Apple ad-hoc signing needs no account or timestamp service; this command supplies no timestamp option.

The result is a development artifact, not a Developer ID/notarized distribution. No Gatekeeper assessment or security override is performed. It does not grant provider generation, log into an account, or launch the app.

The build has a 30-minute observation bound and its own POSIX process group. Timeout, interruption or failure stops that owned group, including compiler descendants, with a bounded wait for the leader. An unconfirmed stop is reported as failure and never proceeds to signing or retry. Fixed native commands have a 30-second bound and 128 KiB output limit, and receive SIGKILL if their bound expires. Native system tools receive a fixed system PATH and C locale, without inherited credential or helper overrides.

## Required native acceptance

Use a checkout of the independently reviewed and published M5 commit. Preserve the existing M4 app and its crash/signature reports before rebuilding. Install the already-approved pinned frontend and SDK prerequisites in the new checkout as needed; SDK setup remains `npm ci --ignore-scripts --omit=optional --no-audit --no-fund` in `sdk-bridge`.

```sh
cd "$M5_CHECKOUT/apps/desktop"
npm run desktop:build:mac-dev
```

The command prints the exact built app path and a new `.local/macos-dev-evidence-*/bundle.json`. Keep this report private with native test evidence. Success requires all of:

- `Contents/_CodeSignature/CodeResources` exists and its SHA-256 is recorded
- `codesign --verify --strict --verbose=2` succeeds for the bundle and any known core helper
- Signature metadata says ad-hoc and includes the hardened-runtime flag, with no certificate authority or team identity
- All seven bundled SDK files match the approved source and upstream package hashes
- A complete file/hash/permission inventory is unchanged during read-only verification
- Every existing attribute compares identically; the two exact new signature nodes are attribute-free or satisfy the documented matching-provenance rule. Strict signature/resource and final observation checks must also pass

After an approved copy to a separate app location, verify the copy without signing or launching it:

```sh
node "$M5_CHECKOUT/scripts/verify_macos_bundle.mjs" \
  --app "$M5_COPIED_APP" --report "$M5_EVIDENCE/copied-bundle.json"
```

Both arguments must be absolute. The report directory must exist, the report must be new, and it must be outside the app. Compare the raw and copied reports' complete `files` arrays, including `CodeResources`. The verifier does not re-sign a bad copy.

Only after this signature/resource gate passes should the coordinator authorize the next bounded startup probe and actual settings-button check against the exact new binary. The existing startup runner's direct-versus-LaunchServices restrictions remain unchanged. Do not repeat an earlier failed probe or force its LaunchServices path merely because this patch exists. Record signature verification, startup/GUI behavior, and provider readiness as separate results.

## Cloud verification and source basis

Run `node --test scripts/test_macos_dev_bundle.mjs` after pinned SDK installation. Tests use synthetic bundles and stub native signing/verification responses; they exercise the real genuine-SDK byte checks, file inventories and failure handling. They never invoke native codesign/xattr, launch an app, or establish a valid macOS signature on Linux. The subprocess-output test invokes only Node against fixed synthetic output.

No Rust, UI, shared workflow, provider, SDK runtime or creative-rule implementation changes are included. Native macOS signing and startup acceptance remain pending until actual logs establish them; Windows acceptance is separate.

Official implementation references:

- [Pinned Tauri no-sign branch](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-bundler/src/bundle/macos/app.rs#L115-L150)
- [Pinned Tauri target/output directory handling](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-cli/src/interface/rust.rs#L1271-L1303)
- [Apple ad-hoc and hardened-runtime signing flags](https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app)
- [Apple nested code and resource seals](https://developer.apple.com/library/archive/technotes/tn2206/_index.html)
- [Apple ad-hoc signer returns before timestamp processing](https://github.com/apple-oss-distributions/Security/blob/db15acbe6a7f257a859ad9a3bb86097bfe0679d9/OSX/libsecurity_codesigning/lib/signer.cpp#L892-L894)
