# Third-party dependency notices

This private sidecar depends on official OpenAI packages, not a locally renamed implementation.

- @openai/codex-sdk 0.160.1 — Apache-2.0 according to its package metadata and included LICENSE
  - https://registry.npmjs.org/@openai/codex-sdk/-/codex-sdk-0.160.1.tgz
  - https://github.com/openai/codex/tree/main/sdk/typescript
- @openai/codex 0.160.1 — Apache-2.0 according to its package metadata
  - https://registry.npmjs.org/@openai/codex/-/codex-0.160.1.tgz
  - https://github.com/openai/codex

Exact SHA-512 package integrity is recorded in package-lock.json. Installed SDK implementation SHA-256 and verified tarball integrity are recorded in verification/package-provenance.json.

Installed dependencies and generated bundle resources are excluded from version control. The native app's readiness bundle includes the unmodified official SDK 0.160.1 JavaScript implementation, package metadata, and exact upstream Apache-2.0 LICENSE (SHA-256 `d17f227e4df5da1600391338865ce0f3055211760a36688f816941d58232d8dc`), along with this notice and the dependency lock. Its implementation imports only Node built-ins during the health check. The required @openai/codex runtime package remains recorded in the lock; its optional platform executable packages are omitted from this health-only bundle. Neither Node nor a Codex runtime binary is redistributed here. Node must be supplied by a verified existing native installation. SDK construction, runtime resolution, account access, and model execution are not part of readiness. Future distribution of additional dependencies or helpers requires their applicable licenses/notices.
