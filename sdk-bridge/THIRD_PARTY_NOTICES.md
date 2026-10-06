# Third-party dependency notices

This private sidecar depends on official OpenAI packages, not a locally renamed implementation.

- @openai/codex-sdk 0.160.1 — Apache-2.0 according to its package metadata and included LICENSE
  - https://registry.npmjs.org/@openai/codex-sdk/-/codex-sdk-0.160.1.tgz
  - https://github.com/openai/codex/tree/main/sdk/typescript
- @openai/codex 0.160.1 — Apache-2.0 according to its package metadata
  - https://registry.npmjs.org/@openai/codex/-/codex-0.160.1.tgz
  - https://github.com/openai/codex

Exact SHA-512 package integrity is recorded in package-lock.json. Installed SDK implementation SHA-256 and verified tarball integrity are recorded in verification/package-provenance.json.

No SDK source, runtime binary or helper is vendored here. The installed dependency directory is excluded from version control. Platform runtime packages were omitted during this offline prototype installation. Final binary distribution must include the appropriate licenses/notices for every actual distributed dependency and helper; this file does not substitute for that release review.
