#!/usr/bin/env python3
"""Verify reference snapshots and the currently compiled rule registry, without execution."""
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[1]
index = json.loads((root / "reference-skills/index.json").read_text(encoding="utf-8"))

def verify(entry):
    path = root / entry["path"]
    assert not path.is_symlink() and path.resolve().is_relative_to(root.resolve()), path
    data = path.read_bytes()
    assert len(data) == entry["bytes"], path
    assert hashlib.sha256(data).hexdigest() == entry["sha256"], path

entries = index["snapshots"] + index["registeredFiles"]
assert len({entry["path"] for entry in entries}) == len(entries)
for entry in entries:
    verify(entry)
assert len(index["snapshots"]) == 72
roles = {
    "desktop-original-2ac7c216": "reference_only_never_loaded",
    "mac-3008-git-2511c56": "earlier_git_reference_never_loaded",
    "mac-3008-production-20261005": "verified_legacy_production_not_loaded_by_next",
}
assert all(entry["runtimeRole"] == roles[entry["group"]] for entry in index["snapshots"])

registry = root / "apps/desktop/src-tauri/src/pipeline/mod.rs"
paths = re.findall(r'include_str!\("([^\"]+)"\)', registry.read_text(encoding="utf-8"))
compiled = {str((registry.parent / path).resolve().relative_to(root)) for path in paths}
assert compiled == {entry["path"] for entry in index["registeredFiles"]}
manifest = json.loads((root / "rules/stage3/manifest.json").read_text(encoding="utf-8"))
assert manifest["active"] is False and index["stage3ActiveInNext"] is False
production = index["groupRoots"]["mac-3008-production-20261005"]
for entry in manifest["files"]:
    verify(entry)
    relative = Path(entry["path"]).relative_to("rules/stage3/candidates/mac-3008-20261006")
    assert (root / production / relative).read_bytes() == (root / entry["path"]).read_bytes()
print(json.dumps({"status": "passed", "referenceFiles": 72, "compiledRules": len(compiled),
                  "productionCandidateMatches": len(manifest["files"]), "stage3Active": False}))
