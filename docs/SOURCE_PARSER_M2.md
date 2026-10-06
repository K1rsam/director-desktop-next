# M2 source parsing and explicit source decisions

## Scope and source locks

This implementation is confined to the independent M2 desktop worktree. The M1 worktree, original web repository, creative rule candidates, and existing source-lock manifests are unchanged. No model, provider account, user computer, real source fixture, or remote write was used for this work.

The pinned desktop baseline remains `K1rsam/director-web@2ac7c216183f936aa08f16a93b2a24130cf5ebcd`. The GitHub connector compare operation verified that `8548e9f5917b8960f693a052e395f3b6e4048b1e` is exactly one commit ahead, with exactly three changed files: `director-desktop/src-tauri/src/pipeline/source.rs`, `director-desktop/src/Stage3Panel.tsx`, and `docs/DESKTOP-UPDATE-20261007.md`. The commit and exact file contents were fetched read-only through the GitHub connector, not a browser or branch checkout. This is a selective delta review, not a baseline repin.

| Upstream file | Old Git blob | Old SHA-256 | New Git blob | New SHA-256 |
|---|---|---|---|---|
| source.rs | `1236b45e2c4cf91109fd40e70afe715946065b14` | `77997aac7bc348713f6e34700daf9133502a8ef7cc0c89bb69d9554a070df231` | `04a03b1d50118666408e1992b7b7917f664497eb` | `45e8055020334df90eab16ee264635794a4796854b52ad62fa82919143bfa69d` |
| Stage3Panel.tsx | `93c1acd102cf94b6655deb178bd712ec7b22ceba` | `17782df37c8aabfa8b9c59b4538af3f3cb5a22b5a24cb4e969c85dd00f7c6074` | `7367e0caea5a360c0038a0a3c394c3861f7eb43d` | `abaa3b539b8485b62c6d19998317a0f2eef79553a7c1286190c957c9e1dc78c5` |

The new upstream source bytes were also checked against their Git blob SHA: 84,867 bytes for source.rs; 18,051 bytes for Stage3Panel.tsx. Source text is not normalized for this check. M1's Stage3Panel already contains independent offline-policy controls, so its local pre-change SHA-256 is `6ede608291152ecdfbfc209a285fdd706bbcafb6c817edf7ec75f77e14c97fb8`, rather than the original upstream hash. The local pre-change SourceStructure SHA-256 is `e9161dd8df238c30bab34e64ae4a2b6b7fa7da34d1cc439263a377776bccfaf2`.

### Actual reuse from 8548e9f

- Retained the upstream v5 display-block header and narrow numbered-field classifier, version-aware structural parsing and integrity checks, and the two synthetic tests for silent captions versus genuine voices/registered cast
- Preserved that v5 parsing contract and extended it into the distinct v6 contract for this task's additional fixes
- Adapted the zero-unit episode range selector and existing-unit navigation semantics in Stage3Panel, while preserving M1's execution-policy buttons and adding an immediate in-flight guard
- Did not import the upstream private saved-source ignored test, its private data paths, any real source/result material, or its production report
- Read the upstream report only as attributed context. Its 721 Rust tests and Windows/production claims are the upstream author's evidence, not this work's results

## Parser contracts and exact source preservation

- Saved `source-parser/v1` through `v4` continue to validate against their original parser. The upstream `v5` caption contract is separately supported
- `source-parser/v6` is selected only when its complete deterministic projection differs from the previously selected v3/v4/v5 projection. Unaffected new imports keep their established parser version. Opening/upgrading saved sources never calls the fresh-version selector
- Latin suffix names such as Carlos, Bravo, Ríos and decomposed combining-mark forms stay complete. An attached OS/VO suffix is recognized after a Han character; explicit space/bracket cues still work for Latin names
- Numbered scene headers support commas, Chinese commas and enumeration commas. The decimal-looking `1.2 秒后 日 内 大厅` remains narrative. Existing dashed location-first headings retain their old contract
- `1-1 日 内 大厅 甲：快走` retains one original row, its scene identity, exact speaker, original quote, and UTF-16 coordinates. Prefix removal is only an in-memory slice for recognition; the original text/rows/hash are never rewritten
- Inline recognition requires a distinct place token and an unambiguous speaker start. Unresolved scene payloads and multiple lexical speakers remain blocking diagnostics. There is no automatic rewrite or ambiguity waiver
- Full integrity and canonical reconstruction checks remain in place, including SHA-256, complete row coverage/order, original IDs, exact CRLF/emoji/combining-character ranges, dialogue segments, episodes, scenes, diagnostic sets and saved edits
- The narrow existing system-announcement wrapper compatibility remains, including its rejection of altered ranges/segments

## Explicit decisions and duplicate episodes

New decisions use `boundary-decision/v2`. The new optional fields are omitted for legacy decisions; saved old decision projections, including their original order, still replay unchanged.

New manual decisions rebuild episode/scene directories and memberships by original source position. Confirming a later fragment before an earlier one produces the same chronological directory. Different independent episodes cannot be merged solely because their displayed number matches.

`independent_episode` is a separate confirmation, not a body/context reinterpretation. It is bound to the original source SHA, parser version, exact heading row ID, exact UTF-16 row range, original episode number, the duplicate diagnostic, and the source version accepted by the engine. A confirmation for one repeated heading cannot resolve another. New body/context decisions cannot erase a duplicate-episode warning.

The UI offers “保留为独立制作段” per diagnostic. The review modal retains the exact row and proof. Changed version/hash/parser, changed row coordinates, removed diagnostic, cancellation or an already-pending write prevent a stale/double submission. The engine checks both expectedSourceVersion and the proof's sourceVersion before applying the decision; the source module separately checks its remaining evidence. Reconfirming the same established identity retains the first persisted proof rather than creating another source revision.

## Reparse and old scene edits

Both the saved source and the final reparsed result must pass full validation. Reparse first builds an independent in-memory result, never mutating the previous source on failure.

Explicit boundary decisions must retain the same exact rows, classifications and provable episode/scene membership. Scene edits are replayed one step at a time on both old and new projections. A target is mapped only if exactly one candidate has the same complete scene row set, same prior metadata, and the same substantive episode row set. Adjacent episode-heading-only format rows may coalesce; a same source hash alone is insufficient. A later metadata edit can target a split-derived scene only after the exact same original cut and anchor have replayed on both projections.

Membership drift, lost/ambiguous targets, altered cut anchors or changed source data reject the reparse and retain the old source and edits. No wholesale migration exemption was introduced. Store's existing narrow metadata-only v2→v3 rule is unchanged; new v6 structural or caption changes take its conservative dependency-invalidation path.

## Frontend range behavior

“还没有分镜的集数” selects only authoritative source episode IDs with zero current units. Even a partial existing episode is excluded; zero-unit selection does not certify other episodes as complete. Selection recomputes when source episodes or units change, can be empty, and does not hide existing units from the reading/navigation view. Offline execution-policy controls remain intact.

## Verification

Environment: isolated M2 Node dependencies; Rust shared toolchain and M2 target directory. No native desktop, real account, model or image/video execution was performed.

Commands used:

```sh
export CARGO_HOME=/workspace/shared/director-next-toolchain/cargo
export RUSTUP_HOME=/workspace/shared/director-next-toolchain/rustup
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_TARGET_DIR=/workspace/shared/director-next-m2-target
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --release --lib pipeline::source:: -- --nocapture
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --release --lib
cd apps/desktop
npm run typecheck
node --experimental-strip-types --test src/*.test.ts
npm run build
```

- Focused parser checkpoint: 35 passed, 0 failed, 1 pre-existing private test ignored
- Prior full-core checkpoint: 597 passed, 0 failed, 20 intentionally ignored. This was before the last membership/idempotence hardening; final results are recorded below
- Final frontend: 84 passed, 0 failed; typecheck and production build passed
- UI tests exercise actual shared components via SSR and the exact request/scope helpers; they do not claim mounted-browser, native dialog, or platform QA
- Final release core aggregate: 601 passed, 0 failed, 20 intentionally ignored; includes the final source membership/idempotence hardening; the engine duplicate-proof guard is compiled in this snapshot, with its dedicated stale-positive-version regression owned by the integration lead

Logs and exact local file hashes are stored under `docs/verification/source-parser-m2/`. All new test inputs are synthetic. Ignored private/live tests were not run. Existing compiler warnings about an unrelated unused import and mutable binding remain unrelated to this change.

## Known limits and rollback

Ambiguous inline payloads and multi-speaker lines remain blocked for explicit source-coordinate resolution; this change does not add a general dialogue-splitting editor. Decimal scene headers require a leading standalone scene metadata token. The source contract and TypeScript/Rust regression results do not establish Windows/macOS native behavior or creative quality.

Persisted v6 sources and boundary-decision/v2 confirmations require this M2-aware client. Rollback requires the previous project copy/source version plus the old application, not changing parserVersion on new saved data. Existing v1–v5 sources without these new decisions continue to validate through their original contracts.
