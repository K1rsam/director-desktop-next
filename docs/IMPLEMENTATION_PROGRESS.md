# Cloud implementation checkpoints

This document records implementation evidence, not native-platform or creative-quality acceptance.

## M0 — Independent, pinned migration

- Migrated 144 explicitly selected source/rule/icon/help files from `K1rsam/director-web` commit `2ac7c216183f936aa08f16a93b2a24130cf5ebcd`
- Every input byte sequence matched that commit's Git blob before adaptation; see `MIGRATED_SOURCE.json`
- Retained the existing shared `director_core`, SQLite workflow/cache/recovery logic and four-stage React UI
- Excluded old product scripts/updaters, tool executables, private production reports, media, databases, credentials and environment files
- New application ID `app.director.next`, credential namespace `director-desktop-next`, package/binary `director-desktop-next`, CLI binary `director-next-core`
- A foreign application registry cannot be adopted automatically as the new app's home
- The 22 original Stage3 candidates remain byte-identical and inactive. Migrated older desktop rules are reference/regression assets; they are not evidence that the adopted Mac creative runtime has been integrated

## M1 — Offline vertical slice

The new core can create an independent project, import synthetic two-episode source with original UTF-16 content, persist and reopen it, edit local assets/units, and preserve inherited workflow evidence. The default binary refuses real generation, provider calls, credential operations and unknown IPC actions before dispatch. Its scheduler cannot dispatch external work.

- UI and Rust enforce separate fail-closed boundaries
- Existing Stage2 styles/layout and all four stages are preserved; SDK is a distinct stored preference, not a silent CLI fallback
- The webview's default asset scope is empty; registered project media/thumbnails alone are added by the native adapter
- Private compile-time production fixtures have synthetic replacements retaining their assertions; see `FIXTURE_MIGRATION.md`
- A versioned frozen text attempt separates semantic request identity from backend/model/runtime identity and preserves unknown submission/thread identity across restart
- The first SDK transport seam explicitly reports unavailable rather than pretending to run. Actual official SDK bridge work is tracked separately in `SDK_BRIDGE.md`

## Verification commands

From repository root:

```sh
python scripts/verify_source_lock.py
npm --prefix apps/desktop ci
npm --prefix apps/desktop run test:frontend
npm --prefix apps/desktop run build
cargo test --release --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --no-default-features -- --test-threads=1
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --no-default-features --bin director-next-core
python scripts/verify_offline_smoke.py --core apps/desktop/src-tauri/target/debug/director-next-core
```

On Windows, use `director-next-core.exe`. Do not pass `--ignored`. The offline smoke owns a new temporary directory and never imports a production project.

The root manual CI workflow repeats shared tests and compiles the native shell on Windows/macOS. Creating this workflow does not mean it has run; it is manual to avoid silently assuming the private repository's runner budget.

## Recorded M1 result

[Machine-readable evidence](verification/M1.json) records the current Linux run: **716 Rust tests passed, zero failed, 24 deliberately ignored** across 22 suites; **81 frontend tests passed**, typecheck/build passed; 22 inactive creative candidates matched their hashes. The real release Rust CLI passed an 11-process synthetic create/import/reopen/blocked-execution smoke with zero generation jobs. The official SDK bridge passed 41 offline tests using the real SDK with synthetic subprocesses; see `SDK_BRIDGE.md`. It is not yet connected to the Rust product execution path and has not made a live SDK call.

The first debug library run reported 550 passed, 6 failed, 20 ignored. Five failures were Unix fake-executable fixture permissions; one was the Linux keyring mock's non-persistence across Entry instances. They were diagnosed and fixed without removing assertions or running real credentials/models. The final release aggregate above includes those regressions. Native and creative acceptance remain open.

## Remaining gates

1. Actual Mac inspection has established that all 22 original rule files match the adopted tasks and current working tree. Runtime adapter integration still requires the separately permissioned transfer of 29 runtime/source files; private source/adopted drafts remain outside the repository. Candidate export hashes alone do not establish implementation equivalence
2. Faithful Stage3 adapter integration, precise stage/scene/state facts and asset supply, refinement/adoption flow, with frozen creative files untouched
3. Complete official SDK bridge-to-SavedTextExecutor wiring, backend-specific recovery/cancellation and packaging; SDK live calls remain off for this round
4. A separately bounded native CLI acceptance path tied to the approved input hash, full-script Stage1/2 text scope and Stage3 EP1/EP2 only, fixed `gpt-6.1-sol` / `medium`, no silent fallback
5. Native Mac and Windows builds, startup/IPC/media/keyring/process-tree and restart checks on the same final revision; Linux tests cannot prove these
6. Actual CLI EP1/EP2 creative comparison against the verified adopted draft. Any image sample additionally needs local proof of exact official Jimeng model and free eligibility; real video remains outside this round

## Verified limits and deviations

Cloud Chromium startup failed under the environment's Unix-socket restriction, including an escalated attempt. Component rendering/mocked IPC passed, but browser layout, screenshots and native interaction are not claimed.

Initial frontend dependency reuse via symlink let Vite write `.vite` / `.vite-temp` caches inside a cloud reference dependency directory. Source and package files were not changed. That server was stopped; dependencies were copied to a new isolated toolchain directory and final checks ran there. No attempt was made to delete the old caches. Dependencies, build artifacts and symlinks are excluded from publication.

The inherited debug 80-episode capacity tests are synthetic and comparatively slow. They do not make product model calls. Test logs must distinguish completed, failed and still-running checks; a selected suite is not the whole suite.

## Rollback

This project has its own repository, application ID, directory, registry and keyring namespace. Roll back this repository's code revision or discard its newly created synthetic test home. Never point the new binary at the original application's registry, migrate its credentials implicitly, or alter original databases to perform a rollback.
