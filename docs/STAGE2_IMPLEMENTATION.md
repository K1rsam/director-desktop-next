# Stage2 M2 implementation

Status: cloud/offline implementation, synthetic regression evidence only. No real model, image, account, CLI, SDK, or video generation was called by this work. No claim of native rendering or artistic-quality acceptance is made.

## Version and contract

- `apps/desktop/rules/asset-art-director/SKILL.md` is an explicit **2.0.0 Stage2 creative upgrade**, implementing the complete Appendix A proposal in `IMPLEMENTATION.md`. The wire result remains `asset-design/v1`; identity, source-row and exact direct-reference validation remain in Rust.
- Age, skin tone, body, scars, period and source facts remain authoritative. Attractive, differentiated facial structure, hair silhouette, cut and materials are design goals. BJD/DD is conditional on the selected style; it is not a universal face/body/skin preset. Derived designs retain the identified parent and only source-supported changes.
- Stage3 rule originals are unchanged by this patch.
- Current Stage2 rule SHA-256: `9193cd59b44bdc77fb413fb8c40bb6531c2cfb9377ca17fafc0749808ba6cd82`.

## Inputs and presentation

The request now includes `projectVisualStyle`, confirmed `productionDirection`, `imagePresentation`, and `comparisonDesigns`. Existing direct parents and INLINE reuse remain explicit. The model supplies asset design prose only; the image compiler assembles style, prose and presentation. It does not remove words such as `第十集`, even when they resemble episode metadata.

CH base and derived appearances default to a large portrait at left and front/side/back complete figures at right, clean neutral background, no information panel, with identity/hair/clothing/state consistency. An explicit per-asset `imagePresentation.composition` (or legacy `composition`) wins over the project CH override. Per-asset presentation ratio, then asset ratio, then saved project ratio wins; only a missing CH ratio falls back to 16:9. GROUP, BEAST, LOC and PROP receive no CH layout. Explicit invalid provider parameters remain subject to the provider validator; the compiler does not silently substitute them.

The UI exposes project style, a separately confirmed art direction, and optional CH presentation. Editing direction clears its confirmation. The asset panel separately copies saved design prose and the final prompt, with the latter compiled by the same Rust function used for submission through read-only `images.previewPrompt`. Unsaved prose disables final-copy. Historical prompts and images are not rewritten by these settings controls.

## Bounded aesthetic comparisons

- Stable formal identity (identityId, then baseAssetId, then asset ID) deduplication; deterministic identity/asset-ID ordering
- Excludes the current identity, legal direct/source-chain identities, deleted assets, non-CH kinds and invalid/stale AI designs
- At most 8 records and 1,600 Unicode characters of prose per record; at most 6,400 Unicode characters for the serialized comparison array, including metadata/escaping
- The current asset's required facts, direct parent, evidence and rules are not truncated to make room
- Comparison records carry aesthetic-only provenance. They are not facts, reference images, dependency edges, or design-validity dependencies
- The actual selected comparisons are saved with the design context and original workflow; subsequent comparison-character edits do not make the current design stale

## Separate design validity

`pipeline/asset_design_contract.rs` owns `asset-design-context/v2` receipts. The semantic basis binds:

1. Actual Stage2 art-rule SHA-256
2. Locked source SHA-256, selected asset semantics and relevant normalized fact semantics
3. Ordered legal direct/source chain with semantic and saved design hashes
4. Effective project style, confirmed production direction and presentation/ratio

Save counters, adopted images, quote materialization, fact-extractor cache rule hashes and other characters' designs are excluded. Fact order is normalized. A new AI design also retains its actual comparisons and model/reasoning/channel context. Existing AI designs without this receipt become stale for a user-initiated automatic design/produce operation; manual and unverified historical prose are not relabeled or automatically rewritten. Explicit design remains an explicit request to produce a new candidate.

Publication still uses optimistic source/foundation/asset versions and an atomic transaction. It now checks the complete prospective batch's semantic receipts before writing anything, including direct-source designs not being regenerated in that batch. A source edited during a child design cannot sneak through adoption.

## Submission, gates and recovery

- Direct `images.run` and one-click paths independently gate unresolved visual facts and invalid/not-covered CURRENT_SCOPE_NOT_APPLICABLE receipts, even with an existing manual prompt
- Known in-flight/unknown submissions remain owned by their original jobs; these gates do not cause them to be resubmitted
- The final queued image snapshot contains compiled prompt and original prose, effective style/presentation, provider/model, ratio/resolution, actual provider configuration, saved settings, ordered references, dependency job IDs, design receipt/context and source-chain versions
- One-click queueing receives the workflow's saved settings directly, instead of calling `store.settings()` again for the submission parameters
- Scheduler configuration reloads the full original job because `pending_jobs` intentionally exposes only a compact projection; it does not infer that a missing projected field means there was no frozen setting
- Existing frozen images keep their original generation parameters when defaults change; account ownership is still checked against current identity. Legacy unfrozen jobs retain their older invalidation behavior
- Waiting dependencies must match the frozen source-design semantics and original dependency jobs before references can be filled
- Resume preserves `imageResumeOrigin`, original selection and settings, and the frozen comparison context. Selected asset/source-chain semantic changes block continuation. Related fact changes outside the original workflow's own published result block continuation; quote materialization and unrelated-character changes do not. New contexts also freeze the art-rule hash so an upgrade requires a new operation rather than silently rewriting an old one
- Original workflow snapshots, raw results, task IDs, unresolved states and revision budgets are not rewritten or reset

## Offline verification

Toolchain: `CARGO_HOME=/workspace/shared/director-next-toolchain/cargo`, `RUSTUP_HOME=/workspace/shared/director-next-toolchain/rustup`, `CARGO_TARGET_DIR=/workspace/shared/director-next-m2-target`. Frontend dependencies were copied into M2's own `apps/desktop/node_modules` from the independent shared toolchain directory. M1 dependencies and frozen artifacts were not modified.

Commands (run from the M2 root unless noted):

```sh
cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --lib --no-fail-fast
cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --lib pipeline::asset --no-fail-fast
cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --test stage2_contracts --no-fail-fast
cd apps/desktop && npm run typecheck && npm run test:frontend && npm run build
```

Evidence:

- Full library run after the initial fixture adaptation: 572 passed, 4 failures in concurrently developed SDK-core tests, 20 intentionally ignored live tests. Stage2 tests passed. This is not a whole-M2 pass; the lead owns final aggregate rerun after SDK fixes
- `stage2_contracts`: 2/2 passed, proving accepted base/derived receipt publication and restart reuse, style-only redesign reuses visual-fact cache without extraction calls, unadopted results do not mutate saved design, and direct-source concurrent edit rejects atomically
- Focused Stage2 pipeline/fact/design run: 56 passed, 0 failed (including the final rule/fact resume guard and serialized comparison-budget tests)
- Frontend typecheck passed; existing 81 frontend tests passed; production Vite build passed (49 modules)
- Engine Stage2 restart/submission/gate run: 3 passed, 0 failed; the pre-existing live Stage2 handoff fixture remained ignored

Local logs: `/tmp/stage2-final-lib.log`, `/tmp/stage2-focused.log`, `/tmp/stage2-contracts.log`, `/tmp/stage2-contracts-final.log`, `/tmp/stage2-engine.log`, `/tmp/stage2-ts.log`, `/tmp/stage2-frontend.log`, `/tmp/stage2-build.log`. They contain synthetic/offline test output only. Copies are preserved in the ignored M2 `.local/stage2-validation/` directory for the final handoff bundle.

## Scope and remaining validation

The implementation establishes input, cache, publication, gate and recovery contracts. It does not prove generated faces/clothes are aesthetically successful, four views depict the same person, or a particular provider obeys layout. Those need the separately authorized native and carefully bounded real-image samples after exact model/free-account eligibility is verified. No free eligibility was assumed. Invalid or missing channels remain blocked. SDK live, additional Stage3 episodes and actual video generation were not part of this work.

Legacy workflow snapshots that predate a saved Stage2 context have no historical comparison selection to recover; their existing settings/request/identity protections are retained. New snapshots save this context explicitly. No arbitrary reconstruction of a missing old art-rule snapshot is claimed.

## Files owned by this Stage2 patch

- `apps/desktop/rules/asset-art-director/SKILL.md`
- `apps/desktop/src-tauri/src/pipeline/asset_design_contract.rs` (new)
- `apps/desktop/src-tauri/src/pipeline/assets.rs`
- `apps/desktop/src-tauri/src/pipeline/mod.rs` (module/export and Stage2 expected rule version)
- `apps/desktop/src-tauri/src/binding.rs`
- `apps/desktop/src-tauri/src/engine.rs` (shared with later lead integration)
- `apps/desktop/src-tauri/src/store.rs`
- `apps/desktop/src-tauri/src/workflow.rs` (`publish_asset_design` only)
- `apps/desktop/src-tauri/src/engine_repair_tests.rs`
- `apps/desktop/src-tauri/tests/stage2_contracts.rs` (new)
- `apps/desktop/src/AssetPanel.tsx`
- `apps/desktop/src/ProductionSettings.tsx`
- `apps/desktop/src/Panels.tsx` (remove old duplicate style control)
- `apps/desktop/src/types.ts` (controlled Stage2 settings fields)
- `docs/STAGE2_IMPLEMENTATION.md` (new)

## M2 评审后的文字设计恢复

`images.design` 保存 `stage2-runner/v1` 冻结上下文，续跑保持原父工作流和原设置。资产/变体、事实包及修复各有独立子 checkpoint；SDK 合成真实包的双资产重启测试证明不会互相覆盖或额外执行。部分阻塞的设计只保存原输出与已接受子步骤，暂不发布部分资产/事实，因而不推进版本。全部通过时资产/制作基础发布与原父工作流完成原子提交；父任务写入失败时整笔采用回滚。图片制作 `images.produce` 既有部分排队策略未改变。

最终整体验证以 `verification/M2.json` 为准；此节不宣称原生或真实模型已验收。
