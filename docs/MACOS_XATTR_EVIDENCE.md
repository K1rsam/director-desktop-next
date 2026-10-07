# M5.1 attribute evidence and strict comparison

The M5 native build produced a bundle whose separate strict signature, hardened-runtime and seven SDK resource checks passed. Its attribute gate failed. The original pre-sign listing was not saved and cannot be reconstructed. Later repeated listings are observations of the later state only; they do not prove the original failure was an ordering difference, provenance change, quarantine change or tool failure.

M5.1 fixes the known diagnostic and comparison defects without choosing a cause or relaxing the gate. It does not add Canvas/provider changes or alter shared application runtime code.

## Byte-correct observations

`macos_xattr_snapshot.c` uses descriptor-based list/get APIs for one canonical file or directory. It rejects symlinks in every path component, includes Darwin compression attributes, and compares two complete reads plus descriptor/path metadata. Names and values are lossless bytes in a bounded private subprocess pipe. The Node caller immediately converts values to byte counts and SHA-256 hashes before writing evidence. The app root and all directories are included.

Snapshots are sorted by relative path and raw attribute-name encoding, so enumeration order is irrelevant. Embedded NULs, non-UTF-8 names and empty values are retained. Read failure, timeout, output overflow, race, unsupported metadata and malformed/partial output never become an empty successful snapshot.

Bounds: 128 tree nodes, depth 12, 64 attributes per node, 64 KiB name list and individual value, and 256 KiB combined names/values per node. A snapshot permits at most 1,024 attributes and 4 MiB value bytes overall, with a 30-second collection budget and at most five seconds per native read. Collector compilation has a separate 30-second bound and owned-process-group cleanup, including compiler/linker descendants. Evidence files are limited to 2 MiB. No attribute-write/removal operation exists in the collector. These are checked observations, not atomic filesystem snapshots; modifications undone between observations cannot be ruled out.

## Comparison policy

- Every attribute addition, removal or byte change on an existing node fails, including all `com.apple.*`, quarantine and security attributes. Existing nodes cannot disappear or change type
- The only expected new nodes are the `Contents/_CodeSignature` directory and its regular `CodeResources` file. They are accepted only with no attributes and with `CodeResources` present
- Any attribute on either new signature node requires explicit review. This includes `com.apple.provenance`; there is no blanket exception. Current native evidence suggests this review stop may occur, so the next run is useful evidence collection even if it remains blocked
- Any other new node fails. Strict signature, hardened-runtime and SDK checks remain independent and still required

Collection failures use `MAC_DEV_XATTR_COLLECTION_*`. Actual existing-node/attribute differences use `MAC_DEV_XATTR_CHANGED`. New signature-node attributes use `MAC_DEV_XATTR_NEW_SIGNATURE_ATTRIBUTES_REVIEW`; unexpected new nodes use `MAC_DEV_XATTR_UNEXPECTED_NODES`. None authorizes repair, removal, retry or launch.

## Native handoff

Use the reviewed published M5.1 commit and the existing `npm run desktop:build:mac-dev` command once. It creates a fresh target, then prints a fresh private evidence directory before collecting attributes. The existing system C compiler builds the read-only collector there; no new package install or security setting is required by the implementation. Compilation or execution denied by the environment remains a reported failure, never a reason to alter permissions, quarantine or signing policy.

The evidence directory contains:

- `collector.json`: source/binary hashes and compiler result, or sanitized compilation failure
- `xattrs-before.json`: complete structured pre-sign observation, or a clearly incomplete failure record
- `xattrs-after.json`: post-attempt observation, including failures. It is intentionally absent if the pre-sign collection prevented signing
- `xattrs-decision.json`: before/after file hashes, collection/difference classification, exact relative paths and attribute-name encodings, and whether signing was attempted/succeeded
- `bundle.json`: only when the attribute gate and independent signature/resource verifier all pass

Values and raw collector output are never persisted. Failed captures retain partial hashed records with `complete:false`; they must not be presented as complete observations. Reports use exclusive creation and cannot overwrite an earlier attempt. Preserve them when a gate fails and return their sanitized classification/difference records for review. Do not manufacture a historical before-state from the old M5 app or its current attributes.

If new signature-node provenance is the only difference, stop and assess the recorded bytes and signing behavior before deciding any narrower policy. Do not clear provenance or quarantine, add a general ignore rule, rerun signing, rebuild automatically, or launch the app to get past the failure.

## Verification limits

Cloud checks run the native collector on isolated Linux fixtures and exercise the real collector-to-Node hash protocol; synthetic native signing responses cover ordering, added nodes, directory mutations, quarantine changes and command failures. They do not establish macOS collector behavior, signature success, startup success or the old SIGABRT cause. The native collector compilation and bounded first snapshot must succeed on the Mac before its evidence is usable.

Apple's [attribute-name API documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/listxattr.2.html) says enumeration order is arbitrary. The [published command implementation](https://github.com/apple-oss-distributions/file_cmds/blob/659a8a301e2acf0343f8b8673a154a2ca4d07084/xattr/xattr.c#L164-L312) also illustrates why a formatted recursive listing is not the new byte-level evidence format. Neither source identifies what happened in the previous native attempt.
