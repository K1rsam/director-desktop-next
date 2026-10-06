#!/usr/bin/env python3
"""Verify frozen candidate bytes and migration hygiene without invoking any runtime."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "rules/stage3/manifest.json").read_text(encoding="utf-8"))
assert manifest["active"] is False, "This migration milestone must not activate the unverified candidate"
assert manifest["activePackId"] is None
for entry in manifest["files"]:
    path = root / entry["path"]
    assert path.resolve().is_relative_to(root.resolve()), path
    payload = path.read_bytes()
    assert len(payload) == entry["bytes"], path
    assert hashlib.sha256(payload).hexdigest() == entry["sha256"], path

source = json.loads((root / "docs/MIGRATED_SOURCE.json").read_text(encoding="utf-8"))
assert source["commit"] == "2ac7c216183f936aa08f16a93b2a24130cf5ebcd"
assert len({row["destinationPath"] for row in source["files"]}) == len(source["files"])
for row in source["files"]:
    assert (root / row["destinationPath"]).is_file(), row["destinationPath"]

for path in (root / "apps/desktop/src-tauri").rglob("*.rs"):
    content = path.read_text(encoding="utf-8")
    assert "foundation-real-a-20261005" not in content, f"Private report dependency: {path}"
for pattern in ["*.sqlite", "*.sqlite-wal", "*.sqlite-shm", ".env", ".env.local", "auth.json"]:
    assert not list(root.glob(f"**/{pattern}")), f"Private runtime file found: {pattern}"
config = json.loads((root / "apps/desktop/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
assert config["identifier"] == "app.director.next"
assert config["app"]["security"]["assetProtocol"]["scope"] == []
print(json.dumps({"status":"passed", "frozenCandidates":len(manifest["files"]),
    "pinnedSourceFiles":len(source["files"]), "candidateRuntimeActive":False,
    "checks":["candidate bytes", "pinned migration index", "private fixture exclusion", "new app identity", "empty default asset scope"]}))
