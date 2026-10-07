# M3 native startup diagnostics

This is an opt-in evidence collection change for the existing isolated `--verify-startup` smoke test. It does not identify or fix the cause of the native crash. Ordinary application startup is unchanged. No account, model, image generation, video, CUA, signing change, sandbox override, or additional Mac source export is part of this runner.

The prior Mac M1 direct launch of the M1 `d2ad2bd6a21de9293f51c5b8296f6ac800ce4d9a` bundle exited with SIGABRT after approximately 1.17 seconds. Its stdout and stderr were empty; no startup report, diagnostic home marker, registry, PNG, or SQLite appeared. These observations do not prove where the abort occurred or that AppKit, signing, permissions, or the executor sandbox caused it. The seven separately observed Rust fixture failures at `TcpListener::bind` with `PermissionDenied` are a different finding.

## What M3 records

The Rust entry point creates `startup.events.ndjson` and durably writes `main_entered` before Tauri construction. Each bounded record has `schema: startup-diagnostics/v1`, a sequence, a fixed stage, PID, platform, Unix milliseconds, and elapsed milliseconds. Frontend checkpoints are fixed names. A verification-mode Rust panic logs its compiler location and omits the payload. The journal has at most 144 records of at most 1,024 bytes, with terminal capacity reserved after routine events.

The stages locate entry, argument acceptance, desktop build, setup, engine initialization, synthetic fixture creation, media scopes, page load, probe injection, fixed frontend checkpoints, report receipt/persistence, and shutdown. They are evidence of reached code, not a causal explanation of a later crash. `report_persisted` appears only after the existing `startup.json` has been successfully written. The report adds `diagnosticPid` and `diagnosticSchema` for journal/PID correlation.

`scripts/diagnose_native_startup.py` requires Python 3.9+ and macOS for an actual launch. It uses only the Python standard library. For each invocation it:

1. Checks the exact `.app/Contents/MacOS/director-desktop-next` executable and its `Info.plist`, requires the independent app identifier `app.director.next`, hashes both, reads Mach-O architecture headers without executing the binary, and records the supplied reviewed commit reference. The reference is explicitly caller supplied, not a binary provenance attestation; the whole bundle is not claimed to have one comprehensive hash
2. Exclusively creates the requested, previously nonexistent absolute directory with private permissions. Its parent must already exist. A prior run directory, including an empty one, cannot be reused. The probe home is always `<run-directory>/startup-verification-home`
3. Writes `launch.json` before one `Popen` call and `spawn.json` after obtaining its exact child PID. No shell, environment dump, retry loop, account/model call, or broad process-name search is used. The inherited environment is not changed
4. Drains stdout and stderr concurrently. Each local log retains at most 64 KiB while recording observed byte count, truncation, EOF state, and SHA-256. These logs are kept locally; do not commit private native run directories or raw logs to the repository
5. Writes a new `result.json` with UTC start/observed-exit timestamps, nanosecond monotonic durations, child role/PID, return code or signal, capped-output accounting, report presence, and a validated journal summary. Exit timestamps are observations by the runner, not kernel-certified exit times. Arbitrary report error text, model parameters, environment values, credentials, and private input are not copied into the summary

The default outer observation budget is 110 seconds; the CLI allows 100–600 seconds. This leaves margin for the existing 30-second probe watchdog and 60-second shutdown watchdog. If the exact owned direct child remains running at the deadline, the runner requests termination of that child only, with up to two seconds for TERM and two seconds for KILL. It records timeout/termination and never converts that outcome into permission for another launch. Descendants holding pipes open can also make capture incomplete and the outcome unknown.

For LaunchServices, `/usr/bin/open` is the runner's child, not the app. The runner never signals an app PID learned from a journal or a process-name lookup, and does not kill `open` on a timeout. A timeout may leave the launcher/app running; preserve the evidence and report the exact known PID and uncertainty. Do not start another probe or guess an app PID. Interrupted/crashed runners can leave only partial evidence and likewise do not authorize another launch.

## Local command plan after independent review and publication

Run from the approved new local checkout, built from the exact reviewed M3 commit. First verify the checkout SHA and the built app path; do not substitute the prior M2 app or an existing user's production app. Set the three placeholders below to those verified values. The `M3_COMMIT` value must be the complete 40-character SHA. These commands are a plan; cloud offline tests do not claim that they ran on the Mac.

```sh
M3_CHECKOUT='/absolute/path/to/the/new/m3-checkout'
M3_COMMIT='REPLACE_WITH_REVIEWED_40_CHARACTER_M3_SHA'
M3_APP='/absolute/path/to/the/rebuilt/导演工作台 Next.app'
M3_EVIDENCE_ROOT="$(mktemp -d /tmp/director-next-m3-native.XXXXXX)"
test "$(git -C "$M3_CHECKOUT" rev-parse HEAD)" = "$M3_COMMIT" || exit 1
(
  cd "$M3_CHECKOUT/apps/desktop" || exit 1
  npm ci --no-audit --no-fund && npm run desktop:build:mac-dev
) || exit 1
# Confirm M3_APP is the exact verified .app path printed by this build.
python3 "$M3_CHECKOUT/scripts/diagnose_native_startup.py" \
  --mode direct \
  --app "$M3_APP" \
  --build-ref "$M3_COMMIT" \
  --architecture arm64 \
  --report-dir "$M3_EVIDENCE_ROOT/direct" \
  --timeout 110
python3 -m json.tool "$M3_EVIDENCE_ROOT/direct/result.json"
```

There is exactly one direct app launch. Runner exit `0` means the direct child exited zero and a valid, persisted, successful startup report agrees with the journal and child PID. Runner exit `1` means diagnostic failure, incomplete evidence, or a LaunchServices result requiring the interpretation below. Exit `2` means refusal or runner unavailability. A nonzero exit is not an instruction to repeat the command.

For M5 and later local development builds, follow [the signature/resource gate](MACOS_DEVELOPMENT_BUNDLE.md) first and substitute the actual reviewed commit for the historical M3 placeholders. That gate does not establish startup success or authorize repeating an earlier probe.

Only if this direct invocation completed with a nonzero app return code, `classification` is `failed_before_rust_marker`, `eligibleForLaunchServices` is true, and its journal/report/probe home remain absent, an explicitly chosen second invocation may use LaunchServices:

```sh
python3 "$M3_CHECKOUT/scripts/diagnose_native_startup.py" \
  --mode launchservices \
  --prior-direct "$M3_EVIDENCE_ROOT/direct/result.json" \
  --app "$M3_APP" \
  --build-ref "$M3_COMMIT" \
  --architecture arm64 \
  --report-dir "$M3_EVIDENCE_ROOT/launchservices" \
  --timeout 110
python3 -m json.tool "$M3_EVIDENCE_ROOT/launchservices/result.json"
```

Its exact launch is `/usr/bin/open -n -W <app> --args --verify-startup <new-directory>/startup.json`. The runner re-reads the prior launch/spawn/result files, checks identical paths, commit, architecture and binary/plist hashes, requires completed nonzero direct exit without timeout/interruption/runner termination, and refuses if any Rust/report/home evidence exists. The previous M1 narrative or an incomplete report cannot substitute for M3 runner evidence. An exclusive `launchservices-claim.json` in the direct run consumes that one permitted comparison even if spawning subsequently fails. It is never overwritten or removed by the runner. A further attempt requires a new reviewed plan, not deletion of the claim or reuse of a probe home.

Do not schedule this command automatically, wrap it in retries, or treat a pending, unknown, interrupted, or timed-out direct process as a completed failure. If the direct run reaches `main_entered`, use its last valid stage to focus the next diagnosis; the optional launch comparison is then refused.

## Interpret the result precisely

- `startup_verified`: direct app exited zero; `startup.json` says `ok: true`; the journal begins with `main_entered` and contains `report_persisted`; journal PID, report diagnostic PID, and directly owned child PID agree
- `startup_report_verified_app_exit_unobserved`: LaunchServices waiter completed with zero and a persisted successful report agrees with the journal's app PID. Startup evidence passed, but the app's actual exit code is unavailable; `appExitCode` remains null. The waiter's return code alone never establishes success, and the runner intentionally returns `1` for this limited result
- `failed_before_rust_marker`: the owned direct app completed with a nonzero return code, with no journal, report or probe home. No Rust marker was observed. That alone does not prove Rust was never entered, because evidence creation itself could fail
- `startup_report_failed`: a valid report says false and its diagnostic PID agrees with the journal and, in direct mode, the child
- `failed_or_inconclusive_evidence`: missing, malformed, partial, mismatched, or insufficient evidence; no inferred success
- `unknown_process_or_timeout`: bounded observation ended, the runner was interrupted, or the process status is unresolved. Preserve evidence and report any remaining known process; no launch comparison is permitted

Keep separate the observed launcher PID, journal app PID, direct app exit/signal, and any unavailable LaunchServices app exit status. Retain original evidence without edits. Share only reviewed, non-private summaries. M3 is diagnostic coverage, not native acceptance or creative-quality acceptance; Windows native validation remains separate.

## Offline regression

```sh
python3 -m unittest discover -s scripts -p 'test_diagnose_native_startup.py' -v
```

The tests use temporary synthetic bundles, synthetic Python child processes, and an explicitly stubbed LaunchServices command. They never run `/usr/bin/open`, an actual native app, any local Mac command, CUA, a model, or a provider API. Coverage includes completed nonzero/signal exits, real bounded dual-pipe drainage, exact-child timeout cleanup, no LaunchServices timeout signaling, fresh-directory protection, immutable one-comparison claims, stale/tampered/partial prerequisite rejection, binary/architecture identity, journal bounds and corruption, report/PID binding, and the distinction between launcher completion and application success. These POSIX tests run on Linux or macOS; they do not establish native platform behavior.
