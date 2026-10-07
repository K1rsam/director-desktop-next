#!/usr/bin/env python3
"""One explicit, isolated macOS startup probe; no retry, login or sandbox override."""
import argparse
import datetime
import hashlib
import json
import os
import platform
import plistlib
import selectors
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

SCHEMA = "native-startup-run/v1"
EVENT_SCHEMA = "startup-diagnostics/v1"
CAPTURE_BYTES = 64 * 1024
EVENT_BYTES = 144 * 1024
REPORT_BYTES = 1024 * 1024
STAGES = frozenset("main_entered arguments_accepted desktop_build_started desktop_build_failed setup_entered "
    "engine_ready fixture_seeded media_scope_ready setup_complete page_loaded probe_injected "
    "probe_injection_failed frontend report_received report_persisted watchdog_timeout "
    "exit_requested shutdown_started shutdown_finished native_exit_issued rust_panic".split())
CHECKPOINTS = frozenset("started project_round_trip asset_protocol stage_one source_review "
    "stage_three settings asset_lifecycle unit_editor project_lifecycle inline_create "
    "completed failed".split())


class Refused(Exception):
    pass


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="microseconds")


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def write_new(path, value):
    # A crash leaves incomplete evidence in place; a later run cannot replace it.
    with path.open("x", encoding="utf-8") as output:
        json.dump(value, output, indent=2, sort_keys=True)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())


def bounded_read(path, limit):
    if path.is_symlink() or not path.is_file():
        raise Refused("Evidence must be a regular non-symlink file")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise Refused("Evidence exceeds its size limit")
    return data


def absolute_path(value):
    path = Path(value)
    if not path.is_absolute() or ".." in path.parts:
        raise Refused("Use an absolute path without parent traversal")
    return path


def macho_architectures(path):
    """Read headers only; never invoke the supplied executable for identification."""
    with path.open("rb") as source:
        data = source.read(4096)
    cpu_names = {0x0100000C: "arm64", 0x01000007: "x86_64", 12: "arm", 7: "x86"}
    magic = data[:4]
    endian = {b"\xcf\xfa\xed\xfe": "<", b"\xce\xfa\xed\xfe": "<",
              b"\xfe\xed\xfa\xcf": ">", b"\xfe\xed\xfa\xce": ">"}.get(magic)
    if endian and len(data) >= 8:
        return [cpu_names.get(struct.unpack(endian + "I", data[4:8])[0], "unknown")]
    fat = {b"\xca\xfe\xba\xbe": (">", 20), b"\xbe\xba\xfe\xca": ("<", 20),
           b"\xca\xfe\xba\xbf": (">", 32), b"\xbf\xba\xfe\xca": ("<", 32)}.get(magic)
    if fat and len(data) >= 8:
        endian, width = fat
        count = struct.unpack(endian + "I", data[4:8])[0]
        if not 1 <= count <= 16 or len(data) < 8 + width * count:
            return []
        return sorted({cpu_names.get(struct.unpack_from(endian + "I", data, 8 + i * width)[0],
                                     "unknown") for i in range(count)})
    return []


def app_identity(app, build_ref, expected_architecture):
    app = absolute_path(app).resolve(strict=True)
    if app.suffix != ".app" or not app.is_dir():
        raise Refused("--app must identify the exact built .app directory")
    if len(build_ref) != 40 or any(c not in "0123456789abcdef" for c in build_ref):
        raise Refused("--build-ref must be the reviewed 40-character commit SHA")
    binary = app / "Contents/MacOS/director-desktop-next"
    if binary.is_symlink() or not binary.is_file() or not os.access(binary, os.X_OK):
        raise Refused("The expected bundle executable must be a regular executable file")
    if binary.resolve().parent != (app / "Contents/MacOS").resolve() or not binary.resolve().is_relative_to(app):
        raise Refused("Bundle executable resolves outside the supplied app")
    plist_path = app / "Contents/Info.plist"
    plist_bytes = bounded_read(plist_path, 64 * 1024)
    info = plistlib.loads(plist_bytes)
    if info.get("CFBundleExecutable") != binary.name:
        raise Refused("Info.plist does not identify the expected bundle executable")
    if info.get("CFBundleIdentifier") != "app.director.next":
        raise Refused("Info.plist must identify the independent app.director.next application")
    architecture = macho_architectures(binary)
    if expected_architecture not in architecture:
        raise Refused("Observed Mach-O architecture does not match --architecture")
    bundle = {}
    for key in ("CFBundleIdentifier", "CFBundleVersion", "CFBundleShortVersionString"):
        value = info.get(key)
        bundle[key] = value if isinstance(value, str) and len(value) <= 256 else None
    return {"appPath": str(app), "binaryPath": str(binary), "binarySha256": digest(binary),
            "infoPlistSha256": hashlib.sha256(plist_bytes).hexdigest(), "bundle": bundle,
            "machOArchitectures": architecture, "expectedArchitecture": expected_architecture,
            "buildRef": build_ref, "buildRefSource": "caller_supplied_not_embedded_attestation"}


def summarize_events(path):
    result = {"present": path.exists() or path.is_symlink(), "valid": False,
              "mainEntered": False, "pid": None, "stages": [], "records": []}
    if not result["present"]:
        return result
    try:
        data = bounded_read(path, EVENT_BYTES)
        result["sha256"] = hashlib.sha256(data).hexdigest()
        lines = data.splitlines()
        if not data.endswith(b"\n") or not 1 <= len(lines) <= 144:
            raise ValueError()
        pid, previous_elapsed = None, -1
        records = []
        for index, line in enumerate(lines, 1):
            if len(line) + 1 > 1024:
                raise ValueError()
            event = json.loads(line)
            if not isinstance(event, dict):
                raise ValueError()
            event_pid = event.get("pid")
            elapsed = event.get("elapsedMillis")
            unix_ms = event.get("unixMillis")
            if (event.get("schema") != EVENT_SCHEMA or event.get("sequence") != index
                    or event.get("stage") not in STAGES
                    or type(event_pid) is not int or event_pid <= 0
                    or type(elapsed) is not int or elapsed < previous_elapsed
                    or type(unix_ms) is not int or unix_ms <= 0
                    or event.get("platform") not in ("macos", "windows", "linux")):
                raise ValueError()
            if index == 1:
                if event["stage"] != "main_entered":
                    raise ValueError()
                pid = event_pid
            if event_pid != pid:
                raise ValueError()
            record = {k: event[k] for k in ("sequence", "stage", "pid", "unixMillis", "elapsedMillis")}
            if event["stage"] == "frontend":
                if event.get("checkpoint") not in CHECKPOINTS:
                    raise ValueError()
                record["checkpoint"] = event["checkpoint"]
            if event["stage"] == "rust_panic":
                if event.get("panicPayloadRecorded", False) is not False:
                    raise ValueError()
                record["panicPayloadRecorded"] = False
                if "panicLocation" in event:
                    location = event["panicLocation"]
                    if not isinstance(location, dict):
                        raise ValueError()
                    name = location.get("file")
                    if (not isinstance(name, str) or not 1 <= len(name) <= 96
                            or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-" for c in name)
                            or any(type(location.get(k)) is not int or not 1 <= location[k] <= 0xFFFFFFFF
                                   for k in ("line", "column"))):
                        raise ValueError()
                    record["panicLocation"] = {k: location[k] for k in ("file", "line", "column")}
            records.append(record)
            previous_elapsed = elapsed
        result.update(valid=True, mainEntered=True, pid=pid, records=records,
                      stages=[r["stage"] for r in records], lastStage=records[-1]["stage"])
    except (OSError, ValueError, TypeError, Refused):
        result["problem"] = "invalid_or_unreadable_journal"
    return result


def summarize_report(path):
    result = {"present": path.exists() or path.is_symlink(), "valid": False, "ok": None,
              "diagnosticPid": None, "diagnosticSchema": None}
    if not result["present"]:
        return result
    try:
        data = bounded_read(path, REPORT_BYTES)
        value = json.loads(data)
        if not isinstance(value, dict) or type(value.get("ok")) is not bool:
            raise ValueError()
        pid = value.get("diagnosticPid")
        result.update(valid=True, sha256=hashlib.sha256(data).hexdigest(), ok=value["ok"],
                      diagnosticPid=pid if type(pid) is int and pid > 0 else None,
                      diagnosticSchema=value.get("diagnosticSchema") if value.get("diagnosticSchema") == EVENT_SCHEMA else None)
    except (OSError, ValueError, TypeError, Refused):
        result["problem"] = "invalid_or_unreadable_report"
    return result


def validate_prior(prior_path, identity):
    """Only our completed nonzero direct child, with no Rust evidence, permits open."""
    prior_path = absolute_path(prior_path)
    if prior_path.name != "result.json":
        raise Refused("--prior-direct must name the direct run's result.json")
    prior = json.loads(bounded_read(prior_path, REPORT_BYTES))
    launch = json.loads(bounded_read(prior_path.parent / "launch.json", REPORT_BYTES))
    spawn = json.loads(bounded_read(prior_path.parent / "spawn.json", REPORT_BYTES))
    if not all(isinstance(value, dict) for value in (prior, launch, spawn)):
        raise Refused("Prior run evidence must contain JSON objects")
    process = prior.get("process", {})
    if not all(isinstance(value, dict) for value in
               (process, prior.get("events"), prior.get("report"))):
        raise Refused("Prior run lacks structured process, journal or report evidence")
    expected_dir = str(prior_path.parent.resolve(strict=True))
    expected_command = [identity["binaryPath"], "--verify-startup", str(prior_path.parent / "startup.json")]
    if (prior.get("schema") != SCHEMA or launch.get("schema") != SCHEMA or prior.get("mode") != "direct"
            or prior.get("classification") != "failed_before_rust_marker"
            or prior.get("identity") != identity or launch.get("identity") != identity
            or prior.get("runDirectory") != expected_dir or launch.get("runDirectory") != expected_dir
            or launch.get("mode") != "direct" or launch.get("command") != expected_command
            or process.get("completed") is not True or process.get("timedOut") is not False
            or process.get("interrupted") is not False or process.get("terminationRequested") is not False
            or type(process.get("returncode")) is not int or process["returncode"] == 0
            or type(process.get("childPid")) is not int or process["childPid"] <= 0
            or spawn.get("childPid") != process["childPid"]
            or prior.get("events", {}).get("present") is not False
            or prior.get("report", {}).get("present") is not False
            or process.get("runnerError") is not None or not process.get("observedEndUtc")):
        raise Refused("Prior evidence does not prove a completed, matching, pre-marker direct failure")
    for name in ("startup.events.ndjson", "startup.json", "startup-verification-home"):
        path = prior_path.parent / name
        if path.exists() or path.is_symlink():
            raise Refused("Prior run now has Rust/report/home evidence; no LaunchServices attempt")
    return {"resultPath": str(prior_path), "resultSha256": digest(prior_path),
            "childPid": process["childPid"], "returncode": process["returncode"]}


def collect_process(command, directory, mode, budget, capture_limit=CAPTURE_BYTES):
    """Drain both pipes fairly while retaining only a bounded prefix of each."""
    started_utc, started_unix_ns, started = stamp(), time.time_ns(), time.monotonic_ns()
    result = {"startedUtc": started_utc, "startedUnixNs": started_unix_ns,
              "childPid": None, "childRole": "app" if mode == "direct" else "open_launcher",
              "completed": False, "timedOut": False, "interrupted": False,
              "terminationRequested": False, "returncode": None, "signal": None}
    captures = {name: {"retainedBytes": 0, "observedBytes": 0, "truncated": False, "eof": False}
                for name in ("stdout", "stderr")}
    handles = {name: (directory / (name + ".log")).open("xb") for name in captures}
    selector = selectors.DefaultSelector()
    child = None
    try:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, cwd=directory, shell=False)
        result["childPid"] = child.pid
        write_new(directory / "spawn.json", {"childPid": child.pid, "startedUtc": started_utc,
                                             "childRole": result["childRole"]})
        for name in captures:
            stream = getattr(child, name)
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, name)
        while True:
            returncode = child.poll()
            if returncode is not None and not result["completed"]:
                result.update(completed=True, returncode=returncode, observedEndUtc=stamp(),
                              observedEndUnixNs=time.time_ns(),
                              durationNs=time.monotonic_ns() - started)
            if result["completed"] and not selector.get_map():
                break
            remaining = budget - (time.monotonic_ns() - started) / 1e9
            if remaining <= 0:
                result["timedOut"] = True
                break
            for key, _ in selector.select(min(remaining, 0.1)):
                try:
                    block = os.read(key.fileobj.fileno(), 8192)
                except BlockingIOError:
                    continue
                capture = captures[key.data]
                if not block:
                    capture["eof"] = True
                    selector.unregister(key.fileobj)
                    continue
                capture["observedBytes"] += len(block)
                retained = block[:max(0, capture_limit - capture["retainedBytes"])]
                handles[key.data].write(retained)
                capture["retainedBytes"] += len(retained)
                capture["truncated"] = capture["observedBytes"] > capture_limit
    except KeyboardInterrupt:
        result["interrupted"] = True
    except OSError as error:
        result["runnerError"] = {"type": type(error).__name__, "errno": error.errno}
    finally:
        # Direct Popen ownership proves this exact child's identity. Never signal an
        # app PID obtained from LaunchServices, a journal, ps, or a process name.
        if child is not None and child.poll() is None and mode == "direct":
            result["terminationRequested"] = True
            try:
                child.terminate()
                try:
                    child.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    child.kill()
                    try:
                        child.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        pass
            except OSError as error:
                result["terminationError"] = {"type": type(error).__name__, "errno": error.errno}
        if child is not None:
            returncode = child.poll()
            if returncode is not None and not result["completed"]:
                result.update(completed=True, returncode=returncode, observedEndUtc=stamp(),
                              observedEndUnixNs=time.time_ns(), durationNs=time.monotonic_ns() - started)
            for stream in (child.stdout, child.stderr):
                stream.close()
        selector.close()
        for handle in handles.values():
            handle.flush()
            os.fsync(handle.fileno())
            handle.close()
    result["observationEndedUtc"] = stamp()
    result["observationDurationNs"] = time.monotonic_ns() - started
    if result["returncode"] is not None and result["returncode"] < 0:
        number = -result["returncode"]
        try:
            name = signal.Signals(number).name
        except ValueError:
            name = "UNKNOWN"
        result["signal"] = {"number": number, "name": name}
    for name, capture in captures.items():
        capture["sha256"] = digest(directory / (name + ".log"))
    result["capture"] = captures
    return result


def classify(mode, process, events, report, home_present):
    pid_matches = (events["valid"] and report["valid"]
                   and report["diagnosticSchema"] == EVENT_SCHEMA
                   and events["pid"] == report["diagnosticPid"]
                   and (mode != "direct" or events["pid"] == process["childPid"]))
    if process["timedOut"] or process["interrupted"] or process.get("runnerError") or not process["completed"]:
        return "unknown_process_or_timeout", pid_matches
    if process["returncode"] == 0 and pid_matches and report["ok"] and "report_persisted" in events["stages"]:
        return ("startup_verified" if mode == "direct" else "startup_report_verified_app_exit_unobserved"), pid_matches
    if (mode == "direct" and process["returncode"] != 0 and not events["present"]
            and not report["present"] and not home_present and not process["terminationRequested"]):
        return "failed_before_rust_marker", pid_matches
    if report["valid"] and report["ok"] is False and pid_matches:
        return "startup_report_failed", pid_matches
    return "failed_or_inconclusive_evidence", pid_matches


def run(args):
    if platform.system() != "Darwin":
        raise Refused("Native startup launches require macOS; use the offline test suite elsewhere")
    identity = app_identity(args.app, args.build_ref, args.architecture)
    run_dir = absolute_path(args.report_dir)
    run_dir = run_dir.parent.resolve(strict=True) / run_dir.name
    if args.timeout < 100 or args.timeout > 600:
        raise Refused("--timeout must be 100–600 seconds (30s probe + 60s shutdown + margin)")
    prior = None
    if args.mode == "launchservices":
        if not args.prior_direct:
            raise Refused("LaunchServices requires --prior-direct completed failure evidence")
        prior = validate_prior(args.prior_direct, identity)
    elif args.prior_direct:
        raise Refused("--prior-direct is only valid with --mode launchservices")
    run_dir.mkdir(mode=0o700, exist_ok=False)
    report_path = run_dir / "startup.json"
    command = ([identity["binaryPath"], "--verify-startup", str(report_path)] if args.mode == "direct"
               else ["/usr/bin/open", "-n", "-W", identity["appPath"], "--args", "--verify-startup", str(report_path)])
    if prior is not None:
        # Consumed even if the later spawn fails: no automatic second open attempt.
        write_new(Path(prior["resultPath"]).parent / "launchservices-claim.json",
                  {"runDirectory": str(run_dir), "claimedUtc": stamp()})
    launch = {"schema": SCHEMA, "mode": args.mode, "runDirectory": str(run_dir),
              "command": command, "identity": identity, "budgetSeconds": args.timeout,
              "host": {"system": platform.system(), "release": platform.release(), "machine": platform.machine()},
              "preparedUtc": stamp(), "priorDirect": prior}
    write_new(run_dir / "launch.json", launch)
    process = collect_process(command, run_dir, args.mode, args.timeout)
    events = summarize_events(run_dir / "startup.events.ndjson")
    report = summarize_report(report_path)
    home = run_dir / "startup-verification-home"
    home_present = home.exists() or home.is_symlink()
    classification, pid_matches = classify(args.mode, process, events, report, home_present)
    result = {**launch, "process": process, "events": events, "report": report,
              "probeHomePresent": home_present, "journalReportPidMatches": pid_matches,
              "appPid": events["pid"] if events["valid"] else (process["childPid"] if args.mode == "direct" else None),
              "appExitCode": process["returncode"] if args.mode == "direct" else None,
              "classification": classification,
              "eligibleForLaunchServices": classification == "failed_before_rust_marker"}
    write_new(run_dir / "result.json", result)
    print(json.dumps({"classification": classification, "resultPath": str(run_dir / "result.json"),
                      "eligibleForLaunchServices": result["eligibleForLaunchServices"]}))
    return 0 if classification == "startup_verified" else 1


def parser():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--app", required=True, help="Absolute path to the reviewed .app bundle")
    cli.add_argument("--report-dir", required=True, help="Absolute, nonexistent directory; its parent must exist")
    cli.add_argument("--build-ref", required=True, help="Reviewed source commit SHA, recorded as caller supplied")
    cli.add_argument("--architecture", required=True, choices=("arm64", "x86_64"))
    cli.add_argument("--mode", choices=("direct", "launchservices"), default="direct")
    cli.add_argument("--prior-direct", help="Exact prior direct result.json; required for LaunchServices")
    cli.add_argument("--timeout", type=int, default=110)
    return cli


def main():
    try:
        return run(parser().parse_args())
    except (Refused, OSError, ValueError, KeyError, TypeError, plistlib.InvalidFileException) as error:
        # Do not print file contents, inherited environment, private arguments or tracebacks.
        detail = str(error) if isinstance(error, Refused) else type(error).__name__
        print("Startup diagnostic refused or unavailable: " + detail, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
