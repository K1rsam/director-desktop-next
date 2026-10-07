#!/usr/bin/env python3
"""Offline synthetic-process tests; never launches a native app or /usr/bin/open."""
import argparse
import contextlib
import copy
import io
import json
import os
import plistlib
import signal
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import diagnose_native_startup as runner


def event(pid=123, stage="main_entered", sequence=1):
    return {"schema": runner.EVENT_SCHEMA, "sequence": sequence, "stage": stage,
            "pid": pid, "platform": "macos", "unixMillis": 1000 + sequence,
            "elapsedMillis": sequence}


class DiagnosticTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="synthetic-startup-runner-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.app = self.root / "Synthetic.app"
        self.binary = self.app / "Contents/MacOS/director-desktop-next"
        self.binary.parent.mkdir(parents=True)
        self.binary.write_bytes(b"\xcf\xfa\xed\xfe" + struct.pack("<I", 0x0100000C) + b"fixture")
        self.binary.chmod(0o700)
        (self.app / "Contents/Info.plist").write_bytes(plistlib.dumps({
            "CFBundleExecutable": self.binary.name, "CFBundleIdentifier": "app.director.next",
            "CFBundleVersion": "0", "CFBundleShortVersionString": "0"}))
        self.identity = runner.app_identity(str(self.app), "a" * 40, "arm64")

    def args(self, name="direct", **changes):
        result = dict(app=str(self.app), report_dir=str(self.root / name), build_ref="a" * 40,
                      architecture="arm64", mode="direct", prior_direct=None, timeout=110)
        result.update(changes)
        return argparse.Namespace(**result)

    def synthetic_binary(self, body):
        self.binary.write_text("#!" + sys.executable + "\n" + body)
        self.binary.chmod(0o700)
        self.identity["binarySha256"] = runner.digest(self.binary)

    def run_fixture(self, args):
        with patch.object(runner.platform, "system", return_value="Darwin"), \
                patch.object(runner, "app_identity", return_value=copy.deepcopy(self.identity)), \
                contextlib.redirect_stdout(io.StringIO()):
            status = runner.run(args)
        return status, json.loads((Path(args.report_dir) / "result.json").read_text())

    def failing_direct(self):
        self.synthetic_binary("import sys\nsys.exit(17)\n")
        _, result = self.run_fixture(self.args())
        return self.root / "direct/result.json", result

    def test_identity_reads_actual_binary_hash_and_architecture_without_launch(self):
        self.assertEqual(self.identity["machOArchitectures"], ["arm64"])
        self.assertEqual(self.identity["binarySha256"], runner.digest(self.binary))
        self.assertEqual(self.identity["buildRefSource"], "caller_supplied_not_embedded_attestation")
        with self.assertRaises(runner.Refused):
            runner.app_identity(str(self.app), "a" * 40, "x86_64")
        with self.assertRaises(runner.Refused):
            runner.app_identity(str(self.app), "arbitrary private metadata", "arm64")
        target = self.root / "elsewhere"
        self.binary.rename(target)
        self.binary.symlink_to(target)
        with self.assertRaises(runner.Refused):
            runner.app_identity(str(self.app), "a" * 40, "arm64")

    def test_fat_macho_architecture_headers(self):
        self.binary.write_bytes(b"\xca\xfe\xba\xbe" + struct.pack(">I", 2)
            + struct.pack(">IIIII", 0x0100000C, 0, 0, 0, 0)
            + struct.pack(">IIIII", 0x01000007, 0, 0, 0, 0))
        self.assertEqual(runner.macho_architectures(self.binary), ["arm64", "x86_64"])
        self.binary.write_bytes(b"\xca\xfe\xba\xbe" + struct.pack(">I", 1000000))
        self.assertEqual(runner.macho_architectures(self.binary), [])

    def test_other_app_identifier_is_refused_before_launch(self):
        plist = self.app / "Contents/Info.plist"
        info = plistlib.loads(plist.read_bytes())
        info["CFBundleIdentifier"] = "invalid.other-app"
        plist.write_bytes(plistlib.dumps(info))
        with patch.object(runner.subprocess, "Popen") as spawn:
            with self.assertRaises(runner.Refused):
                runner.app_identity(str(self.app), "a" * 40, "arm64")
            spawn.assert_not_called()

    def test_direct_failure_records_exit_identity_and_allows_only_this_prior(self):
        path, result = self.failing_direct()
        self.assertEqual(result["classification"], "failed_before_rust_marker")
        self.assertTrue(result["eligibleForLaunchServices"])
        self.assertEqual(result["process"]["returncode"], 17)
        self.assertGreater(result["process"]["durationNs"], 0)
        self.assertEqual(result["appPid"], result["process"]["childPid"])
        self.assertEqual(result["command"], [str(self.binary), "--verify-startup", str(path.parent / "startup.json")])
        self.assertEqual(runner.validate_prior(str(path), self.identity)["returncode"], 17)
        identity = {**self.identity, "binarySha256": "b" * 64}
        with self.assertRaises(runner.Refused):
            runner.validate_prior(str(path), identity)

    def test_reused_directory_is_refused_without_spawn_or_overwriting(self):
        path, _ = self.failing_direct()
        original = path.read_bytes()
        with patch.object(runner.subprocess, "Popen") as spawn:
            with self.assertRaises(FileExistsError):
                self.run_fixture(self.args())
            spawn.assert_not_called()
        self.assertEqual(path.read_bytes(), original)

    def test_sigabrt_is_signal_evidence_not_an_assumed_cause(self):
        directory = self.root / "signal"
        directory.mkdir()
        result = runner.collect_process([sys.executable, "-c", 
            "import os,signal,resource; resource.setrlimit(resource.RLIMIT_CORE,(0,0)); os.kill(os.getpid(),signal.SIGABRT)"],
            directory, "direct", 2)
        self.assertEqual(result["returncode"], -signal.SIGABRT)
        self.assertEqual(result["signal"], {"number": signal.SIGABRT, "name": "SIGABRT"})
        self.assertTrue(result["completed"])

    def test_bounded_dual_stream_capture_drains_flood(self):
        directory = self.root / "flood"
        directory.mkdir()
        result = runner.collect_process([sys.executable, "-c",
            "import os; [(os.write(1,b'A'*8192),os.write(2,b'B'*8192)) for _ in range(32)]"],
            directory, "direct", 3, capture_limit=1024)
        self.assertEqual(result["returncode"], 0)
        for name in ("stdout", "stderr"):
            self.assertEqual((directory / (name + ".log")).stat().st_size, 1024)
            self.assertEqual(result["capture"][name]["observedBytes"], 32 * 8192)
            self.assertTrue(result["capture"][name]["truncated"])
            self.assertTrue(result["capture"][name]["eof"])

    def test_direct_timeout_remains_unknown_even_after_owned_child_termination(self):
        directory = self.root / "timeout"
        directory.mkdir()
        result = runner.collect_process([sys.executable, "-c", "import time; time.sleep(30)"],
                                        directory, "direct", 0.05)
        self.assertTrue(result["timedOut"])
        self.assertTrue(result["terminationRequested"])
        self.assertTrue(result["completed"])
        classification, _ = runner.classify("direct", result,
            runner.summarize_events(directory / "startup.events.ndjson"),
            runner.summarize_report(directory / "startup.json"), False)
        self.assertEqual(classification, "unknown_process_or_timeout")

    def test_launchservices_timeout_never_signals_launcher_or_unproven_app(self):
        directory = self.root / "open-timeout"
        directory.mkdir()
        actual_popen = subprocess.Popen
        children = []
        def fixture_spawn(*args, **kwargs):
            child = actual_popen(*args, **kwargs)
            child.terminate = unittest.mock.Mock(wraps=child.terminate)
            child.kill = unittest.mock.Mock(wraps=child.kill)
            children.append(child)
            return child
        with patch.object(runner.subprocess, "Popen", side_effect=fixture_spawn):
            result = runner.collect_process([sys.executable, "-c", "import time; time.sleep(30)"],
                                            directory, "launchservices", 0.05)
        child = children[0]
        try:
            self.assertTrue(result["timedOut"])
            self.assertFalse(result["completed"])
            self.assertFalse(result["terminationRequested"])
            self.assertIsNone(child.poll())
            child.terminate.assert_not_called()
            child.kill.assert_not_called()
        finally:
            # Test owns this synthetic Python child; the production runner did not kill it.
            child.terminate()
            child.wait(timeout=2)

    def test_pending_timeout_success_and_tampered_prior_cannot_launch(self):
        path, original = self.failing_direct()
        variants = [
            {"completed": False}, {"timedOut": True}, {"interrupted": True},
            {"returncode": 0}, {"returncode": None}, {"terminationRequested": True},
            {"childPid": original["process"]["childPid"] + 1},
        ]
        for mutation in variants:
            value = copy.deepcopy(original)
            value["process"].update(mutation)
            path.write_text(json.dumps(value))
            with patch.object(runner.subprocess, "Popen") as spawn:
                with self.assertRaises(runner.Refused):
                    self.run_fixture(self.args("fallback", mode="launchservices", prior_direct=str(path)))
                spawn.assert_not_called()
            self.assertFalse((self.root / "fallback").exists())
        path.write_text(json.dumps(original))
        for name in ("startup.events.ndjson", "startup.json", "startup-verification-home"):
            late = path.parent / name
            late.touch()
            with self.assertRaises(runner.Refused):
                runner.validate_prior(str(path), self.identity)
            late.unlink()

    def test_launchservices_needs_prior_and_minimum_budget(self):
        for args in (self.args(mode="launchservices"), self.args(timeout=99),
                     self.args(report_dir="relative"), self.args(prior_direct="unused")):
            with patch.object(runner.subprocess, "Popen") as spawn:
                with self.assertRaises(runner.Refused):
                    self.run_fixture(args)
                spawn.assert_not_called()

    def test_launchservices_once_exact_command_and_open_zero_is_not_app_success(self):
        path, _ = self.failing_direct()
        actual_popen = subprocess.Popen
        commands = []
        def fake_open(command, **kwargs):
            commands.append(command)
            return actual_popen([sys.executable, "-c", "pass"], **kwargs)
        with patch.object(runner.subprocess, "Popen", side_effect=fake_open):
            status, result = self.run_fixture(self.args("fallback", mode="launchservices", prior_direct=str(path)))
        self.assertEqual(commands, [["/usr/bin/open", "-n", "-W", str(self.app), "--args",
                                     "--verify-startup", str(self.root / "fallback/startup.json")]])
        self.assertEqual(status, 1)
        self.assertEqual(result["classification"], "failed_or_inconclusive_evidence")
        self.assertEqual(result["process"]["childRole"], "open_launcher")
        self.assertIsNone(result["appPid"])
        self.assertIsNone(result["appExitCode"])
        with patch.object(runner.subprocess, "Popen") as spawn:
            with self.assertRaises(FileExistsError):
                self.run_fixture(self.args("fallback-again", mode="launchservices", prior_direct=str(path)))
            spawn.assert_not_called()

    def test_valid_journal_and_report_require_pid_binding_and_persistence(self):
        directory = self.root / "proof"
        directory.mkdir()
        journal = directory / "startup.events.ndjson"
        report = directory / "startup.json"
        records = [event(), event(stage="report_persisted", sequence=2)]
        journal.write_text("".join(json.dumps(value) + "\n" for value in records))
        report.write_text(json.dumps({"ok": True, "diagnosticSchema": runner.EVENT_SCHEMA,
                                     "diagnosticPid": 123, "error": "PRIVATE_SENTINEL"}))
        events, summary = runner.summarize_events(journal), runner.summarize_report(report)
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(summary))
        process = {"childPid": 123, "completed": True, "returncode": 0, "timedOut": False,
                   "interrupted": False, "terminationRequested": False}
        self.assertEqual(runner.classify("direct", process, events, summary, True), ("startup_verified", True))
        process["childPid"] = 124
        self.assertEqual(runner.classify("direct", process, events, summary, True)[0], "failed_or_inconclusive_evidence")
        self.assertEqual(runner.classify("launchservices", process, events, summary, True),
                         ("startup_report_verified_app_exit_unobserved", True))
        summary["diagnosticPid"] = 125
        self.assertFalse(runner.classify("launchservices", process, events, summary, True)[1])
        summary["diagnosticPid"] = 123
        events["stages"] = ["main_entered"]
        self.assertEqual(runner.classify("launchservices", process, events, summary, True)[0], "failed_or_inconclusive_evidence")

    def test_stubbed_launcher_records_distinct_actual_app_pid_and_no_app_exit_claim(self):
        self.synthetic_binary("""import json,os,sys,time
from pathlib import Path
report=Path(sys.argv[-1])
if report.parent.name=='direct': sys.exit(17)
pid=os.getpid()
events=[{'schema':'startup-diagnostics/v1','sequence':i,'stage':stage,'pid':pid,
         'platform':'macos','unixMillis':int(time.time()*1000),'elapsedMillis':i}
        for i,stage in enumerate(['main_entered','report_persisted'],1)]
report.with_suffix('.events.ndjson').write_text(''.join(json.dumps(e)+'\\n' for e in events))
report.write_text(json.dumps({'ok':True,'diagnosticPid':pid,'diagnosticSchema':'startup-diagnostics/v1'}))
""")
        self.run_fixture(self.args())
        actual_popen = subprocess.Popen
        def fake_open(command, **kwargs):
            return actual_popen([sys.executable, "-c", 
                "import subprocess,sys; sys.exit(subprocess.call(sys.argv[1:]))",
                str(self.binary), "--verify-startup", command[-1]], **kwargs)
        with patch.object(runner.subprocess, "Popen", side_effect=fake_open):
            status, result = self.run_fixture(self.args("fallback", mode="launchservices",
                prior_direct=str(self.root / "direct/result.json")))
        self.assertEqual(status, 1)
        self.assertEqual(result["classification"], "startup_report_verified_app_exit_unobserved")
        self.assertNotEqual(result["appPid"], result["process"]["childPid"])
        self.assertEqual(result["appPid"], result["report"]["diagnosticPid"])
        self.assertIsNone(result["appExitCode"])
        self.assertEqual(result["process"]["returncode"], 0)

    def test_journal_corruption_mixed_pid_unknown_stage_partial_and_oversize_fail_closed(self):
        journal = self.root / "startup.events.ndjson"
        variants = [
            b"", b"{}\n", b"{bad}\n", json.dumps(event()).encode(),
            (json.dumps(event()) + "\n" + json.dumps(event(pid=456, stage="report_persisted", sequence=2)) + "\n").encode(),
            (json.dumps(event(stage="PRIVATE_SENTINEL")) + "\n").encode(),
            b"x" * (runner.EVENT_BYTES + 1),
        ]
        for data in variants:
            journal.write_bytes(data)
            result = runner.summarize_events(journal)
            self.assertTrue(result["present"])
            self.assertFalse(result["valid"])
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))

    def test_real_direct_synthetic_report_matches_owned_child(self):
        self.synthetic_binary("""import json,os,sys,time
from pathlib import Path
report=Path(sys.argv[-1]); pid=os.getpid()
events=[{'schema':'startup-diagnostics/v1','sequence':i,'stage':stage,'pid':pid,
         'platform':'macos','unixMillis':int(time.time()*1000),'elapsedMillis':i}
        for i,stage in enumerate(['main_entered','report_persisted'],1)]
report.with_suffix('.events.ndjson').write_text(''.join(json.dumps(e)+'\\n' for e in events))
report.write_text(json.dumps({'ok':True,'diagnosticPid':pid,'diagnosticSchema':'startup-diagnostics/v1'}))
""")
        status, result = self.run_fixture(self.args())
        self.assertEqual(status, 0)
        self.assertEqual(result["classification"], "startup_verified")
        self.assertEqual(result["appPid"], result["process"]["childPid"])
        self.assertFalse(result["eligibleForLaunchServices"])

    def test_panic_summary_retains_only_valid_bounded_compiler_location(self):
        journal = self.root / "startup.events.ndjson"
        panic = event(stage="rust_panic", sequence=3)
        panic.update(panicLocation={"file": "main.rs", "line": 12, "column": 3},
                     panicPayloadRecorded=False, payload="PRIVATE_SENTINEL")
        records = [event(), event(stage="desktop_build_failed", sequence=2), panic]
        def write():
            journal.write_text("".join(json.dumps(value) + "\n" for value in records))
        write()
        result = runner.summarize_events(journal)
        self.assertTrue(result["valid"])
        self.assertEqual(result["records"][-1]["panicLocation"], {"file": "main.rs", "line": 12, "column": 3})
        self.assertFalse(result["records"][-1]["panicPayloadRecorded"])
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        for mutation in ({"file": "/private/main.rs"}, {"file": "秘密.rs"}, {"file": "x" * 97},
                         {"line": 0}, {"line": True}, {"line": 0x100000000}, {"column": -1}):
            panic["panicLocation"] = {"file": "main.rs", "line": 12, "column": 3, **mutation}
            write()
            self.assertFalse(runner.summarize_events(journal)["valid"])
        panic["panicLocation"] = {"file": "main.rs", "line": 12, "column": 3}
        panic["panicPayloadRecorded"] = True
        write()
        self.assertFalse(runner.summarize_events(journal)["valid"])


if __name__ == "__main__":
    unittest.main()
