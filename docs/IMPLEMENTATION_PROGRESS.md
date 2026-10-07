# Cloud implementation checkpoints

This document records implementation evidence, not native-platform or creative-quality acceptance.

## M5 — Explicit Mac development bundle signing

Adds `desktop:build:mac-dev` and a read-only bundle verifier after M4 native evidence showed the same missing resource seal in raw and copied artifacts. The helper builds into a fresh target, bypasses Tauri's attribute-clearing signing branch, applies credential-free Apple ad-hoc signing to the known helper and app, preserves extended attributes, and requires strict signatures, hardened runtime and exact SDK resources. It does not change shared product runtime code or diagnose the separate NSApplication abort.

Linux evidence: 16 focused synthetic packaging/process tests and the existing 17 startup-runner tests pass; source lock passes. Mac signing itself, rebuilt GUI/startup behavior and provider readiness require the next native run. See [native build instructions](MACOS_DEVELOPMENT_BUNDLE.md) and [machine-readable evidence](verification/M5.json).

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

## M2 — Shared Stage2 contracts, source fixes and official SDK integration

M2 is based on the published offline M1 commit `d2ad2bd6a21de9293f51c5b8296f6ac800ce4d9a`. M1 evidence below remains a historical checkpoint; it is not a claim about the current test run.

- Stage2 uses the actual art rule, explicit beauty/factual constraints, default character portrait plus front/side/back layout, bounded comparison context, versioned design/cache receipts and frozen image-generation settings. See [Stage2 implementation](STAGE2_IMPLEMENTATION.md)
- Source v6 preserves prior parser replays and adds explicit source-bound duplicate-episode confirmation, caption/scene boundary corrections and unproduced-episode selection. Selectively reviewed upstream `8548e9f`; this does not repin the whole source baseline. See [parser contracts](SOURCE_PARSER_M2.md)
- `SavedTextExecutor` now invokes the pinned genuine official SDK bridge through the shared Rust prompt/schema, job, checkpoint, domain-validation and recovery paths. Synthetic subprocess tests exercise the actual SDK. Production SDK remains closed; backend changes cannot release unresolved ownership. See [SDK integration](SDK_CORE_INTEGRATION.md)
- An explicit, source-hash/model/scope-bound native CLI mode is separate from default offline startup. Original-workflow ownership survives reservation/creation/binding interruptions; Stage2 resumes the original workflow with frozen inputs. The one-shot core rejects native asynchronous generation before allocating state. See [native acceptance](NATIVE_ACCEPTANCE.md)
- CI installs both pinned frontend and SDK dependencies before required tests. Its synthetic SDK launcher uses native Node on Windows/macOS rather than a POSIX-only script. This implementation change is not proof of a Windows run

M2 native/creative acceptance remains pending. [Final M2 cloud evidence](verification/M2.json): **775 Rust tests passed, zero failed, 24 deliberately ignored** across 24 suites; **84 frontend tests and build passed**; **41 official SDK bridge tests passed without skips**; 11-process core smoke created zero generation jobs. Independent read-only review gave GO for the offline checkpoint only after all five integration blockers and the Stage2 atomic-publication edge were resolved. The first aggregate had four stale synthetic CLI preflight fixtures; those were corrected without weakening the production identity boundary, and the complete aggregate was rerun.

## M3 — Opt-in native startup diagnostics

Based on published M2 `41887d1c19fe547d1c75ebc7bc0beb5dc9f5a4a2`. The prior isolated Mac M1 probe returned SIGABRT without a report. M3 adds observability rather than assuming a cause or treating an existing ordinary GUI window as a successful isolated probe.

- Only `--verify-startup` creates a bounded, durable journal before Tauri construction and installs a panic hook that records compiler location without payload
- Fixed frontend progress checkpoints preserve every original probe action and assertion; removing those additions reproduces the exact M2 probe bytes
- One native runner invocation owns one fresh report directory. A separately requested LaunchServices comparison requires a completed matching direct failure with no Rust/report/home evidence; timeouts and unknowns do not permit another launch
- Both exit identity and evidence integrity are checked. An `open` waiter is never called the app, and its exit code cannot establish the app's actual exit status
- Five Rust diagnostics tests, fifteen existing isolation/execution-boundary tests, and seventeen Python synthetic-runner tests pass. Shared core type checking and Rust/JavaScript syntax checks pass. The unchanged M2 UI/SDK/creative suites were not rerun as if this were another full implementation checkpoint
- Linux desktop-feature compilation is blocked by missing `gobject-2.0.pc`; no M3 native shell compilation or macOS/Windows execution is claimed. A build from the reviewed M3 commit remains part of the bounded Mac handoff

See [commands and result meanings](NATIVE_STARTUP_DIAGNOSTICS.md) and [M3 evidence](verification/M3.json). Product execution policy, SDK live closure, Stage3 inactive rules, credentials and workflow recovery code are unchanged.

## M4 — Usable explicit provider readiness

Based on published M3 `09c8d58f369fad90819b1852e1d5d3c3ca2683d3`. Native Settings now permits explicit non-generating checks for Codex CLI, official Dreamina CLI and the actual packaged Codex SDK worker, independently of the generation gate. Opening Settings no longer automatically reads accounts. Results keep binary/package installation, protocol, account, catalog, network and real generation separate.

The check route never invokes Dreamina OAuth polling/job resumption, login/logout, credential writes or generation. The isolated startup probe explicitly disables host readiness. CLI output is bounded before complete line allocation, and a failed app-server preserves the independently read CLI version without claiming the account is logged out. Observed Dreamina logger-write restrictions halt further checks without changing paths or security settings.

The SDK bundle contains the genuine pinned package and a health-only worker; hello/inspect/shutdown is verified without constructing the inference runtime, account calls or network. Native application resources are used in release builds. Model generation remains untested and disabled for SDK this round.

[Final relevant checks](verification/M4.json) cover the real frontend handlers, synthetic native CLI side effects and overlong outputs, genuine SDK resources, existing cache/recovery/isolation contracts and clean packaging. This is not a repeated claim that every older creative fixture or native platform has run. The same M4 commit must now be built and tested in the native GUI; separately installed dependencies and host-state initialization issues are reported rather than guessed or bypassed. See [readiness contract](READINESS_CONTRACT.md).

## Verification commands

From repository root:

```sh
python scripts/verify_source_lock.py
npm --prefix apps/desktop ci
npm --prefix apps/desktop run test:frontend
npm --prefix apps/desktop run build
npm --prefix sdk-bridge ci --ignore-scripts --omit=optional --no-audit --no-fund
npm --prefix sdk-bridge run verify
cargo test --release --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --no-default-features -- --test-threads=1
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --no-default-features --bin director-next-core
python scripts/verify_offline_smoke.py --core apps/desktop/src-tauri/target/debug/director-next-core
```

On Windows, use `director-next-core.exe`. Do not pass `--ignored`. The offline smoke owns a new temporary directory and never imports a production project.

The root manual CI workflow repeats shared tests and compiles the native shell on Windows/macOS. Creating this workflow does not mean it has run; it is manual to avoid silently assuming the private repository's runner budget.

## Recorded M1 result

[Machine-readable evidence](verification/M1.json) records the historical M1 Linux run: **716 Rust tests passed, zero failed, 24 deliberately ignored** across 22 suites; **81 frontend tests passed**, typecheck/build passed; 22 inactive creative candidates matched their hashes. The real release Rust CLI passed an 11-process synthetic create/import/reopen/blocked-execution smoke with zero generation jobs. The official SDK bridge passed 41 offline tests using the real SDK with synthetic subprocesses; see `SDK_BRIDGE.md`. At the M1 checkpoint it was not connected to the Rust product execution path. M2 adds that wiring; live SDK calls remain unperformed.

The first debug library run reported 550 passed, 6 failed, 20 ignored. Five failures were Unix fake-executable fixture permissions; one was the Linux keyring mock's non-persistence across Entry instances. They were diagnosed and fixed without removing assertions or running real credentials/models. The final release aggregate above includes those regressions. Native and creative acceptance remain open.

## Remaining gates

1. Actual Mac inspection has established that all 22 original rule files match the adopted tasks and current working tree. Runtime adapter integration still requires the separately permissioned transfer of 29 runtime/source files; private source/adopted drafts remain outside the repository. Candidate export hashes alone do not establish implementation equivalence
2. Faithful Stage3 adapter integration, precise stage/scene/state facts and asset supply, refinement/adoption flow, with frozen creative files untouched
3. Native validation and packaging of the implemented official SDK bridge-to-SavedTextExecutor path; verify runtime/config/credential isolation and process-tree cancellation before any later live SDK permission
4. Native exercise of the implemented bounded CLI acceptance path tied to the approved input hash, full-script Stage1/2 text scope and Stage3 EP1/EP2 only, fixed `gpt-6.1-sol` / `medium`, no silent fallback; Stage3 dispatch waits for faithful adapter integration
5. Native Mac and Windows builds, startup/IPC/media/keyring/process-tree and restart checks on the same final revision; Linux tests cannot prove these
6. Actual CLI EP1/EP2 creative comparison against the verified adopted draft. Any image sample additionally needs local proof of exact official Jimeng model and free eligibility; real video remains outside this round

## Verified limits and deviations

Cloud Chromium startup failed under the environment's Unix-socket restriction, including an escalated attempt. Component rendering/mocked IPC passed, but browser layout, screenshots and native interaction are not claimed.

Initial frontend dependency reuse via symlink let Vite write `.vite` / `.vite-temp` caches inside a cloud reference dependency directory. Source and package files were not changed. That server was stopped; dependencies were copied to a new isolated toolchain directory and final checks ran there. No attempt was made to delete the old caches. Dependencies, build artifacts and symlinks are excluded from publication.

The inherited debug 80-episode capacity tests are synthetic and comparatively slow. They do not make product model calls. Test logs must distinguish completed, failed and still-running checks; a selected suite is not the whole suite.

## Rollback

This project has its own repository, application ID, directory, registry and keyring namespace. Roll back this repository's code revision or discard its newly created synthetic test home. Never point the new binary at the original application's registry, migrate its credentials implicitly, or alter original databases to perform a rollback.
