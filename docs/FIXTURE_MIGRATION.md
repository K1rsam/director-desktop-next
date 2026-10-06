# Synthetic foundation regression fixtures

## Source and boundary

The new app baseline is pinned to `K1rsam/director-web@2ac7c216183f936aa08f16a93b2a24130cf5ebcd`. That reference contained test dependencies on uncommitted/private `reports/foundation-real-a-20261005` files. The new repository does not import those reports, source scripts, databases, credentials, media, or saved production requests.

Only test code and newly authored files in `apps/desktop/fixtures/synthetic/` and root `fixtures/synthetic/` are changed by this migration. Production portions of `foundation_delivery.rs`, `foundation_wire.rs`, and `scene_edits.rs` remain unchanged. The fixture change itself does not rewrite production text execution. Subsequent integration work moved credential access behind `platform` and added an explicit unsupported-backend rejection before dispatch; these are recorded separately in `IMPLEMENTATION_PROGRESS.md`. Stage 3 candidates are not activated by this work.

## Replacement data

- `apps/desktop/fixtures/synthetic/foundation-source.txt`: ten short invented episodes, one scene and eight lines per episode; no production story text
- `apps/desktop/fixtures/synthetic/foundation.rs`: test-only builders for a 47-asset F2 catalogue, a second response under the same snapshot, a six-cut Q1 reply adding one asset, and a final Q1 keyed base-only edit
- `fixtures/synthetic/text-executor-q1-rejected.json`: invented frozen Q1 snapshot and rejected result for exact replay/binding tests
- `fixtures/synthetic/text-executor-q1-scope-rejected.json`: invented frozen Q1 snapshot/result containing an untouched out-of-batch variant for the narrow scope-label compatibility tests

The foundation builders deliberately create one invalid legacy group reference spanning two complete adjacent same-scene rows. The ordinary validator rejects that reference; the legacy normalizer must expand only exact boundaries. Negative cases use a truncated end, an interior start, and a wrong source prefix. The builder uses current request construction for setup; it is not a byte-for-byte historical request replay.

The six synthetic split lines are 4, 12, 20, 28, 36, and 44. These replace production-specific coordinates while preserving exact-cut, child-identity, text/hash/row invariance, metadata, and missing-plan rejection assertions. The final base edit has an explicit `SCENES` range with an empty local delta and genuine out-of-batch use; all omitted children must remain byte-identical.

## Covered regressions

### Foundation delivery and wire transport

- `synthetic_ten_episode_catalogue_recovers_all_47_assets_without_guessing_source`: rejects unprovided grouped IDs before normalization; accepts exactly 47 synthetic assets after exact expansion; creates no facts; retains SOURCE_SCOPE classification; refuses invalid boundaries atomically; accepts a second response with the same frozen snapshot
- `compact_profile_fits_synthetic_ten_episode_catalogue_in_one_editor_request`: keeps ten episodes in one compact Q1 batch, preserves exact source rows, retains legacy input construction, and round-trips aliases without changing source data
- `synthetic_editor_patches_base_and_exact_split_scenes_without_a_new_model_call`: retains keyed base and omitted children across two batch scopes, adds six proved scene children and one asset, preserves source text/hash, and rejects the same child references when split declarations are removed
- `synthetic_f2_q1_replies_and_inputs_roundtrip_without_text_changes`: round-trips F2, first Q1, and final Q1 inputs/results; repeats restoration to verify idempotence
- `synthetic_q1_plans_prove_only_the_six_exact_cut_line_children`: verifies six exact children and scoped metadata without CLI, model, or database access; this was previously an ignored private-fixture probe and now runs normally

All original assertions in these three pipeline files remain: 362 in foundation delivery, 49 in wire transport, and 44 in scene edits. These totals include unchanged tests and are not a claim that assertion count alone proves equivalence.

### Text executor

Fourteen active Q1 tests use the new JSON fixtures. All 131 original assertions in the affected test block remain. Coverage includes exact request/workflow/node/hash/raw/checkpoint binding, current error eligibility, full caller validation before acceptance, automatic interpretation policy/profile checks, still-invalid rejection, database rollback, idempotent restart, original snapshot/raw preservation, zero generation, and the one-way untouched external-variant scope exception.

The exact test names are:

1. `q1_base_identity_rejection_requires_call_validated_before_accepting_the_original_raw`
2. `q1_base_identity_rejection_never_uses_scope_migration_or_changed_snapshot`
3. `q1_base_identity_rejection_requires_exact_current_error_and_full_binding`
4. `auto_evidence_q1_replays_same_frozen_result_with_zero_cli_and_keeps_raw`
5. `auto_evidence_q1_requires_exact_auto_policy_profile_and_same_binding`
6. `auto_evidence_q1_still_invalid_stops_and_preserves_original_output`
7. `exact_q1_synthetic_response_replays_without_cli_and_preserves_raw_and_binding`
8. `exact_q1_commit_failure_and_rejection_never_replace_raw_or_resubmit`
9. `exact_q1_drift_or_broken_binding_blocks_only_the_same_known_node`
10. `exact_q1_final_scope_response_preserves_original_error_and_replays_atomically`
11. `exact_q1_final_scope_error_with_ineligible_binding_never_resubmits`
12. `q1_outside_batch_scope_label_reuses_only_the_untouched_synthetic_result`
13. `q1_outside_batch_scope_label_refuses_material_or_target_operations`
14. `q1_outside_batch_scope_exception_is_not_available_for_other_errors`

The independent `foundation_rejection_replay.rs` already uses synthetic sources, temporary stores, and a native offline counter executable for its five normal tests. It has no compile-time private-file imports, so it is unchanged.

## Intentionally not claimed

- Synthetic coverage does not establish compatibility with the original private ten-episode story, all original 47 asset payloads, the six historical cut coordinates, or archived request byte hashes
- The smaller synthetic input does not reproduce the original source's exact token/character budget; it preserves the ten-episode capacity contract and leaves broader existing capacity tests intact
- A historical final replay's 51 flattened rows, original production generation latency, artistic quality, and paid-provider behavior are not covered by this migration
- Windows/macOS process, keychain, GUI, installer/signing, and native runtime validation require separate platform evidence

Existing opt-in tests that require caller-supplied private stores or real model access stay ignored. They are not run as part of fixture validation. The two legacy foundation speed probes now require `DIRECTOR_TEST_FOUNDATION_SOURCE` at runtime rather than embedding a private source in the test binary. Their reports, and the isolated-copy report, require `DIRECTOR_TEST_FOUNDATION_REPORT_DIR`; no report is copied into this repository. The separate integration replay still requires `DIRECTOR_FOUNDATION_REPLAY_FIXTURE`. These opt-in probes do not count as synthetic passes.

## Verification status

- Static scan: no remaining `foundation-real-a-20261005` references in app Rust sources
- Production/test boundary inspection: pipeline production sections unchanged; subsequent text executor integration changes are tracked separately from fixture migration
- Rust formatting: passed for migrated pipeline files and synthetic Rust builder
- First focused compilation: blocked by unrelated migration integration gaps (missing `text_transport.rs`, two Dreamina help resources, and two `Engine::build` call sites). No fixture compiler error was reported in that attempt
- Integration compile blockers were repaired by the implementation lead; focused fixture tests then passed on Linux with Rust 1.99.0
- Pipeline migration: 5/5 distinct tests passed. The `synthetic` filter ran 7/7 because it also matched two text executor tests; [pipeline log](verification/fixtures/pipeline.log)
- Text executor: 14/14 passed using the `text_executor::tests::q1_`, `text_executor::tests::exact_q1_`, and `text_executor::tests::auto_evidence_q1_` filters; [executor log](verification/fixtures/text-executor.log)
- Independent offline rejection replay: 5/5 passed, one pre-existing private-data test ignored; [replay log](verification/fixtures/rejection-replay.log)
- Total distinct focused tests: 24 passed, zero failed. The logs were produced from the test binaries compiled by `cargo test --offline --no-default-features`; no ignored test was selected
- The focused runs precede later credential-adapter/backend-gate integration edits. The implementation lead rebuilds and runs the aggregate suite against the final source
- Full-suite status is tracked separately by the implementation lead. An unrelated engine repair test failed during that run; these focused passes are not a full-suite or native-platform pass

No real model, image, video, account, or live CLI generation was invoked for this migration.
