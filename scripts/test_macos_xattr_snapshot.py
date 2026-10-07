#!/usr/bin/env python3
"""Native read-only xattr protocol tests; synthetic files only, no Mac installs.

Run: python3 scripts/test_macos_xattr_snapshot.py
Real filesystem cases compile and execute the collector. A separate test-only C
translation unit replaces xattr syscalls to exercise limits/errors/races that a
particular filesystem cannot reliably produce. No test hooks enter production.
"""

import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SOURCE = Path(__file__).with_name("macos_xattr_snapshot.c").resolve()
FLAGS = ["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-pedantic"]


class FixtureXattrs:
    """Test fixture access independent of the collector, with no deletion API.

    Python's os xattr functions are Linux-only. Darwin uses the public libc
    signatures from sys/xattr.h; flags expose compression metadata while reads
    and create-only synthetic writes never follow a final symlink.
    """

    def __init__(self):
        self.libc = None
        if sys.platform == "darwin":
            self.libc = ctypes.CDLL(None, use_errno=True)
            common = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_void_p,
                      ctypes.c_size_t, ctypes.c_uint32, ctypes.c_int]
            self.libc.setxattr.argtypes = common
            self.libc.setxattr.restype = ctypes.c_int
            self.libc.getxattr.argtypes = common
            self.libc.getxattr.restype = ctypes.c_ssize_t
            self.libc.listxattr.argtypes = [ctypes.c_char_p, ctypes.c_void_p,
                                          ctypes.c_size_t, ctypes.c_int]
            self.libc.listxattr.restype = ctypes.c_ssize_t

    @staticmethod
    def checked(result):
        if result < 0:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error))
        return result

    def create(self, target, name, value):
        # Refuse collisions and non-test names; never overwrite an OS attribute.
        if not name.startswith(b"user."):
            raise ValueError("Only synthetic user.* fixture attributes may be created")
        if self.libc is None:
            os.setxattr(target, name, value, flags=os.XATTR_CREATE, follow_symlinks=False)
            return
        buffer = ctypes.create_string_buffer(value, max(1, len(value)))
        self.checked(self.libc.setxattr(os.fsencode(target), name, buffer, len(value),
                                       0, 0x0001 | 0x0002))

    def get(self, target, name):
        if self.libc is None:
            return os.getxattr(target, name, follow_symlinks=False)
        options = 0x0001 | 0x0020  # XATTR_NOFOLLOW | XATTR_SHOWCOMPRESSION
        expected = self.checked(self.libc.getxattr(os.fsencode(target), name,
                                                 None, 0, 0, options))
        buffer = ctypes.create_string_buffer(max(1, expected))
        actual = self.checked(self.libc.getxattr(os.fsencode(target), name, buffer,
                                               len(buffer), 0, options))
        if actual != expected:
            raise RuntimeError("Fixture attribute changed during baseline read")
        return buffer.raw[:actual]

    def names(self, target):
        if self.libc is None:
            return [os.fsencode(name) for name in os.listxattr(target, follow_symlinks=False)]
        options = 0x0001 | 0x0020  # XATTR_NOFOLLOW | XATTR_SHOWCOMPRESSION
        expected = self.checked(self.libc.listxattr(os.fsencode(target), None, 0, options))
        buffer = ctypes.create_string_buffer(max(1, expected))
        actual = self.checked(self.libc.listxattr(os.fsencode(target), buffer,
                                                len(buffer), options))
        if actual != expected:
            raise RuntimeError("Fixture attribute names changed during baseline read")
        raw = buffer.raw[:actual]
        if not raw:
            return []
        if not raw.endswith(b"\0") or b"\0\0" in raw:
            raise RuntimeError("Malformed fixture attribute names")
        return raw[:-1].split(b"\0")

    def snapshot(self, target):
        return {name: self.get(target, name) for name in self.names(target)}


SHIM = r'''
#define main collector_main
#define flistxattr fixture_flistxattr
#define fgetxattr fixture_fgetxattr
#include SOURCE_PATH
#undef main
#undef flistxattr
#undef fgetxattr

static const char *scenario;
static const char *target;
static unsigned int lists;
static unsigned int values;

static int is_case(const char *expected)
{
    return strcmp(scenario, expected) == 0;
}

#if defined(__APPLE__)
ssize_t fixture_flistxattr(int fd, char *out, size_t size, int options)
#else
ssize_t fixture_flistxattr(int fd, char *out, size_t size)
#endif
{
    const char *names = "user.a\0";
    size_t length = 7;
    (void)fd;
#if defined(__APPLE__)
    if (options != XATTR_SHOWCOMPRESSION) { errno = EINVAL; return -1; }
#endif
    if (is_case("unsupported")) { errno = ENOTSUP; return -1; }
    if (is_case("permission")) { errno = EACCES; return -1; }
    if (is_case("io")) { errno = EIO; return -1; }
    if (is_case("names_limit")) return 65537;
    if (is_case("total_limit")) {
        names = "user.a\0user.b\0user.c\0user.d\0user.e\0";
        length = 35;
    }
    if (is_case("duplicate")) { names = "user.a\0user.a\0"; length = 14; }
    if (is_case("malformed")) { names = "user.a"; length = 6; }
    if (is_case("empty_name")) { names = "\0"; length = 1; }
    if (is_case("names_race") && lists > 0) names = "user.b\0";
    if (is_case("order")) {
        names = lists == 0 ? "user.b\0user.a\0" : "user.a\0user.b\0";
        length = 14;
    }
    if (is_case("empty_list_grows") && out == NULL) return 0;
    if (out == NULL) return (ssize_t)length;
    if (is_case("range") || size < length) { errno = ERANGE; return -1; }
    memcpy(out, names, length);
    ++lists;
    return (ssize_t)length;
}

#if defined(__APPLE__)
ssize_t fixture_fgetxattr(int fd, const char *name, void *out, size_t size,
                        u_int32_t position, int options)
#else
ssize_t fixture_fgetxattr(int fd, const char *name, void *out, size_t size)
#endif
{
    size_t length = 3;
    (void)fd;
    (void)name;
#if defined(__APPLE__)
    if (position != 0 || options != XATTR_SHOWCOMPRESSION) {
        errno = EINVAL; return -1;
    }
#endif
    if (is_case("missing")) {
#if defined(__APPLE__)
        errno = ENOATTR;
#else
        errno = ENODATA;
#endif
        return -1;
    }
    if (is_case("value_limit")) return 65537;
    if (is_case("value_boundary") || is_case("total_limit")) length = 65536;
    if (is_case("empty_value")) length = 0;
    if (out == NULL) return (ssize_t)length;
    if (size < length) { errno = ERANGE; return -1; }
    if (is_case("value_resize")) length = 2;
    if (is_case("metadata_race") && values == 0) {
        int writer = open(target, O_WRONLY | O_APPEND | O_CLOEXEC);
        if (writer == -1 || write(writer, "!", 1) != 1 || close(writer) == -1)
            exit(70);
    }
    if (is_case("path_race") && values == 0) {
        char *old = malloc(strlen(target) + 5);
        int writer;
        if (old == NULL) exit(70);
        (void)sprintf(old, "%s.old", target);
        if (rename(target, old) == -1) exit(70);
        free(old);
        writer = open(target, O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC, 0600);
        if (writer == -1 || close(writer) == -1) exit(70);
    }
    memset(out, is_case("value_race") && values > 0 ? 'b' : 'a', length);
    ++values;
    return (ssize_t)length;
}

int main(int argc, char **argv)
{
    scenario = getenv("XATTR_FIXTURE_SCENARIO");
    if (scenario == NULL || argc != 2) return 70;
    target = argv[1];
    return collector_main(argc, argv);
}
'''


@unittest.skipUnless(sys.platform in ("linux", "darwin"), "Darwin/Linux collector")
class NativeXattrSnapshotTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="director-xattr-native-test-")
        cls.directory = Path(cls.temporary.name).resolve()
        cls.xattrs = FixtureXattrs()
        cls.binary = cls.directory / "collector"
        cls.shim_binary = cls.directory / "fixture-collector"
        subprocess.run(["/usr/bin/cc", *FLAGS, str(SOURCE), "-o", str(cls.binary)],
                       check=True, capture_output=True, text=True)
        shim = cls.directory / "fixture.c"
        shim.write_text(SHIM.replace("SOURCE_PATH", json.dumps(str(SOURCE))))
        subprocess.run(["/usr/bin/cc", *FLAGS, str(shim), "-o", str(cls.shim_binary)],
                       check=True, capture_output=True, text=True)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def setUp(self):
        self.fixture = tempfile.TemporaryDirectory(dir=self.directory, prefix="fixture-")
        self.root = Path(self.fixture.name)
        self.file = self.root / "private-path-must-not-appear.bin"
        self.file.write_bytes(b"synthetic contents\x00\xff")

    def tearDown(self):
        self.fixture.cleanup()

    def run_collector(self, target=None, *, scenario=None, args=None):
        binary = self.shim_binary if scenario else self.binary
        arguments = [str(self.file if target is None else target)] if args is None else args
        environment = os.environ.copy()
        if scenario:
            environment["XATTR_FIXTURE_SCENARIO"] = scenario
        result = subprocess.run([str(binary), *arguments], env=environment,
                                capture_output=True, text=True, timeout=5)
        self.assertEqual(result.stderr, "")
        self.assertNotIn(str(self.root), result.stdout)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["schemaVersion"], 1)
        self.assertEqual(result.returncode, 0 if payload["ok"] else 1)
        return payload

    def assert_error(self, code, **kwargs):
        self.assertEqual(self.run_collector(**kwargs),
                         {"schemaVersion": 1, "ok": False, "error": code})

    def set_attribute(self, target, name, value):
        # Do not silently skip: both platforms require real native round trips.
        self.xattrs.create(target, name, value)

    def attributes(self, payload):
        self.assertTrue(payload["ok"])
        raw = payload["attributes"]
        self.assertEqual(raw, sorted(raw, key=lambda item: item["nameHex"]))
        return {bytes.fromhex(item["nameHex"]): bytes.fromhex(item["valueHex"])
                for item in raw}

    def test_fresh_file_and_directory_preserve_baseline(self):
        # Fresh Darwin nodes can already contain com.apple.provenance, etc.
        for target, kind in ((self.file, "file"), (self.root, "directory")):
            baseline = self.xattrs.snapshot(target)
            payload = self.run_collector(target)
            self.assertEqual(payload["kind"], kind)
            self.assertEqual(self.attributes(payload), baseline)
            self.assertEqual(self.xattrs.snapshot(target), baseline)

    def test_binary_and_empty_values_are_lossless_without_mutation(self):
        baseline = self.xattrs.snapshot(self.file)
        expected = {b"user.binary": bytes(range(256)), b"user.empty": b"",
                    b"user.newline\nname": b"\x00\n\xff\"\\"}
        if sys.platform == "linux":
            expected[b"user.nonutf8-\xff"] = b"name bytes preserved"
        for name, value in expected.items():
            self.set_attribute(self.file, name, value)
        expected = {**baseline, **expected}
        before = self.file.stat()
        self.assertEqual(self.attributes(self.run_collector()), expected)
        after = self.file.stat()
        self.assertEqual((before.st_ino, before.st_size, before.st_mtime_ns,
                          before.st_ctime_ns, before.st_mode),
                         (after.st_ino, after.st_size, after.st_mtime_ns,
                          after.st_ctime_ns, after.st_mode))
        self.assertEqual(self.xattrs.snapshot(self.file), expected)

    def test_root_and_nested_directory_attributes_are_collected(self):
        child = self.root / "nested-directory"
        child.mkdir()
        for target, value in ((self.root, b"root-value\x00"), (child, b"child-value\xff")):
            with self.subTest(target=target.name):
                baseline = self.xattrs.snapshot(target)
                self.set_attribute(target, b"user.directory", value)
                payload = self.run_collector(target)
                self.assertEqual(payload["kind"], "directory")
                expected = {**baseline, b"user.directory": value}
                self.assertEqual(self.attributes(payload), expected)
                self.assertEqual(self.xattrs.snapshot(target), expected)

    def test_symlinks_are_rejected_at_leaf_ancestor_and_trailing_slash(self):
        file_link = self.root / "file-link"
        file_link.symlink_to(self.file)
        directory_link = self.root / "directory-link"
        directory_link.symlink_to(self.root, target_is_directory=True)
        for target in (file_link, directory_link, str(directory_link) + "/",
                       directory_link / self.file.name):
            with self.subTest(target=str(target)):
                self.assert_error("XATTR_SYMLINK", target=target)

    def test_fifo_rejected_without_blocking(self):
        fifo = self.root / "fifo"
        os.mkfifo(fifo)
        self.assert_error("XATTR_KIND_UNSUPPORTED", target=fifo)

    def test_invalid_and_missing_paths_have_no_private_payload(self):
        for arguments in ([], ["relative-file"], [""], [str(self.file), "extra"],
                          [str(self.root) + "/../private-name"]):
            with self.subTest(arguments=arguments):
                self.assert_error("XATTR_INVALID_ARGUMENT", args=arguments)
        self.assert_error("XATTR_RACE", target=self.root / "missing-private-name")
        self.assert_error("XATTR_RACE", target=str(self.file) + "/")

    def test_actual_attribute_count_limit_does_not_return_partial_snapshot(self):
        baseline = self.xattrs.snapshot(self.file)
        added = {}
        for index in range(65):
            name = f"user.count{index:02d}".encode()
            self.set_attribute(self.file, name, b"")
            added[name] = b""
        self.assert_error("XATTR_COUNT_LIMIT")
        self.assertEqual(self.xattrs.snapshot(self.file), {**baseline, **added})

    def test_fixed_errors_limits_and_races_fail_closed(self):
        cases = {
            "unsupported": "XATTR_UNSUPPORTED",
            "permission": "XATTR_PERMISSION_DENIED",
            "io": "XATTR_IO_ERROR",
            "names_limit": "XATTR_NAMES_LIMIT",
            "value_limit": "XATTR_VALUE_LIMIT",
            "total_limit": "XATTR_TOTAL_LIMIT",
            "range": "XATTR_RACE",
            "missing": "XATTR_RACE",
            "names_race": "XATTR_RACE",
            "value_race": "XATTR_RACE",
            "value_resize": "XATTR_RACE",
            "empty_list_grows": "XATTR_RACE",
            "duplicate": "XATTR_MALFORMED",
            "malformed": "XATTR_MALFORMED",
            "empty_name": "XATTR_MALFORMED",
            "metadata_race": "XATTR_RACE",
            "path_race": "XATTR_RACE",
        }
        for scenario, expected in cases.items():
            with self.subTest(scenario=scenario):
                self.assert_error(expected, scenario=scenario)

    def test_boundary_value_empty_value_and_order_independence(self):
        self.assertEqual(self.attributes(self.run_collector(scenario="value_boundary")),
                         {b"user.a": b"a" * 65536})
        self.assertEqual(self.attributes(self.run_collector(scenario="empty_value")),
                         {b"user.a": b""})
        self.assertEqual(self.attributes(self.run_collector(scenario="order")),
                         {b"user.a": b"aaa", b"user.b": b"aaa"})


if __name__ == "__main__":
    unittest.main(verbosity=2)
