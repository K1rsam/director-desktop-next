# M6.1 bounded signature-metadata acceptance

The M5 native build produced a bundle whose separate strict signature, hardened-runtime and seven SDK resource checks passed. Its attribute gate failed. The original pre-sign listing was not saved and cannot be reconstructed. Later repeated listings are observations of the later state only; they do not prove the original failure was an ordering difference, provenance change, quarantine change or tool failure.

M5.1 fixes the known diagnostic and comparison defects without choosing a cause or relaxing the gate. It does not add Canvas/provider changes or alter shared application runtime code.

The subsequent M5.1 native observation was complete: every original node retained its attributes; only the new signature directory and `CodeResources` file had additional metadata. Their provenance digests matched the pre-sign app root and `Contents` digests. M6.1 admits that precisely bounded observation as a local development-artifact policy. It does not establish who added the metadata, inheritance, effective security equivalence or Gatekeeper trust, and does not explain the earlier M5 failure.

## Byte-correct observations

`macos_xattr_snapshot.c` uses descriptor-based list/get APIs for one canonical file or directory. It rejects symlinks in every path component, includes Darwin compression attributes, and compares two complete reads plus descriptor/path metadata. Names and values are lossless bytes in a bounded private subprocess pipe. The Node caller immediately converts values to byte counts and SHA-256 hashes before writing evidence. The app root and all directories are included.

Snapshots are sorted by relative path and raw attribute-name encoding, so enumeration order is irrelevant. Embedded NULs, non-UTF-8 names and empty values are retained. Read failure, timeout, output overflow, race, unsupported metadata and malformed/partial output never become an empty successful snapshot.

Bounds: 128 tree nodes, depth 12, 64 attributes per node, 64 KiB name list and individual value, and 256 KiB combined names/values per node. A snapshot permits at most 1,024 attributes and 4 MiB value bytes overall, with a 30-second collection budget and at most five seconds per native read. Collector compilation has a separate 30-second bound and owned-process-group cleanup, including compiler/linker descendants. Evidence files are limited to 2 MiB. No attribute-write/removal operation exists in the collector. These are checked observations, not atomic filesystem snapshots; modifications undone between observations cannot be ruled out.

## Comparison policy

- Every attribute addition, removal or byte change on an existing node fails, including all `com.apple.*`, quarantine and security attributes. Existing nodes cannot disappear or change type
- The original attribute-free case remains accepted for the exact `Contents/_CodeSignature` directory and regular `CodeResources` file
- A separate explicit comparison option permits exactly one `com.apple.provenance` attribute on each of those two nodes only when both nodes were absent before and they are exactly the only new nodes. Both pre-existing directory anchors, `.` and `Contents`, must each contain the same nonempty provenance byte count and SHA-256; both new values must match both anchors. The expected value comes from that run, with no user-specific constant, fixed byte length, arbitrary-file fallback or general `com.apple.*` exception
- Mixed empty/provenance nodes, additional keys, different values, missing/duplicate anchors, wrong types, other new paths or any changed existing attribute fail. New quarantine attributes are never permitted by this rule
- Any other new node fails. Strict signature, hardened-runtime and SDK checks remain independent and still required

Collection failures use `MAC_DEV_XATTR_COLLECTION_*`. Actual existing-node/attribute differences use `MAC_DEV_XATTR_CHANGED`. Unmatched new signature-node attributes use `MAC_DEV_XATTR_NEW_SIGNATURE_ATTRIBUTES_REVIEW`; unexpected new nodes use `MAC_DEV_XATTR_UNEXPECTED_NODES`. None authorizes repair, removal, retry or launch. Attribute comparison alone never accepts the whole artifact: strict signatures/hardened runtime/SDK resources, a fresh post-verification attribute snapshot, and a final identical file inventory must also pass.

## Native handoff

Use the reviewed published M6.1 commit and the existing `npm run desktop:build:mac-dev` command once. It creates a fresh target, then prints a fresh private evidence directory before collecting attributes. The existing system C compiler builds the read-only collector there; no new package install or security setting is required by the implementation. Compilation or execution denied by the environment remains a reported failure, never a reason to alter permissions or quarantine. M6 Canvas readiness and its native handoff are independent of this packaging-only patch.

The evidence directory contains:

- `collector.json`: source/binary hashes and compiler result, or sanitized compilation failure
- `xattrs-before.json`: complete structured pre-sign observation, or a clearly incomplete failure record
- `xattrs-after.json`: post-attempt observation, including failures. It is intentionally absent if the pre-sign collection prevented signing
- `xattrs-decision.json`: explicitly attribute-comparison-only; before/after hashes, differences, matching-anchor result, and whether signing was attempted/succeeded
- `xattrs-final.json` and `xattrs-final-decision.json`: a fresh observation after strict signature/resource verification, compared without a new-node exception
- `artifact-verdict.json`: overall local-artifact result, tying the three attribute-proof hashes and exact file-inventory digest to signature/resource/final-read checks. It remains false on failure
- `bundle.json`: only when the complete artifact verdict passes; uses `existingAttributesPreserved` and `newSignatureProvenanceMatched`, without claiming all attributes were always present

Values and raw collector output are never persisted. Failed captures retain partial hashed records with `complete:false`; they must not be presented as complete observations. Reports use exclusive creation and cannot overwrite an earlier attempt. Preserve them when a gate fails and return their sanitized classification/difference records for review. Do not manufacture a historical before-state from the old M5 app or its current attributes.

An accepted result means the stated local observations matched this policy. It does not establish inheritance, notarization, Gatekeeper acceptance or app startup. Notarization remains unverified, Gatekeeper assessment is not performed and the app is not launched by this command. If any condition fails, preserve the evidence; do not clear metadata, use a broader exception, re-sign, rebuild automatically or launch to get past the failure.

## Verification limits

Cloud checks run the native collector on isolated Linux fixtures and exercise the real collector-to-Node hash protocol; synthetic responses cover the full acceptance pipeline and independent adversarial cases cover the new policy. They do not establish native M6.1 signature/startup success or the old SIGABRT cause. Any later native result must identify the exact artifact and saved proof used.

Apple’s [attribute-name API documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/listxattr.2.html) says enumeration order is arbitrary. The [published command implementation](https://github.com/apple-oss-distributions/file_cmds/blob/659a8a301e2acf0343f8b8673a154a2ca4d07084/xattr/xattr.c#L164-L312) also illustrates why a formatted recursive listing is not the new byte-level evidence format. Neither source identifies what happened in the previous native attempt.

Apple documents that [Gatekeeper tracks provenance](https://support.apple.com/guide/security/gatekeeper-and-runtime-protection-sec5599b66df/web). This policy treats it as security metadata. The consulted official material did not establish a stable public contract for this xattr's byte format or inheritance. [Hardened Runtime](https://developer.apple.com/documentation/security/hardened-runtime) and [notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) remain separate from digest equality and ad-hoc signing.
