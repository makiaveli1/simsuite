# SimSuite Technical Debt Cleanup Plan

Date: 2026-05-31 20:42 IST
Repo: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort`
Baseline branch: `main`
Baseline commit: `8dbd196` — `harden ApplyPlan preview snapshot identity`

## Purpose

Clean up warning and maintainability debt without weakening the current preview-only safety boundary.

This plan is deliberately conservative. SimSuite is now carrying enough ApplyPlan safety logic that random cleanup is risky. The right order is:

1. remove mechanical warning noise;
2. mark intentional future-facing schema/API fields explicitly;
3. tighten lint gates only after the warning budget reaches zero;
4. then refactor module boundaries.

## Non-negotiable boundaries

This cleanup must not enable or expose:

- real Apply;
- Restore;
- backup execution against user files;
- file movement/copy/delete/quarantine/replacement;
- result-log writes from the frontend;
- restore-entry writes from the frontend;
- confirmation-token issuance;
- legacy mutating Tauri commands.

Any cleanup that touches command registration, command gating, ApplyPlan validation, path validation, backup/restore prototype code, or move-engine code must prove that the boundary stays closed.

## Current measured warning baseline

Fresh measurements from this working tree:

```bash
cargo check --manifest-path src-tauri/Cargo.toml --message-format=json
# exit 0; 15 rustc warnings, all dead_code

cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 80 warnings; lib test generated 82 warnings, 74 duplicates

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# exit 0; 104 passed; 30 warning lines emitted by lib-test compile
```

### Warning concentration

`cargo check` warnings:

- `src/models.rs`: 6
- `src/core/special_mod_versions/mod.rs`: 3
- `src/seed/mod.rs`: 3
- `src/core/install_profile_engine/mod.rs`: 2
- `src/core/scanner/mod.rs`: 1

`cargo test apply_plan --lib` warning lines:

- `src/core/file_inspector/mod.rs`: 13
- `src/models.rs`: 6
- `src/core/downloads_watcher/mod.rs`: 3
- `src/core/special_mod_versions/mod.rs`: 3
- `src/seed/mod.rs`: 3
- `src/commands/mod.rs`: 1
- `src/core/apply_plan_persistence.rs`: 1

`cargo clippy --all-targets --all-features` warning headers, counting lib plus lib-test additions:

- `src/core/file_inspector/mod.rs`: 21
- `src/models.rs`: 14
- `src/core/downloads_watcher/mod.rs`: 10
- `src/core/install_profile_engine/mod.rs`: 7
- `src/core/rule_engine/sorting_plan.rs`: 6
- `src/core/library_index/mod.rs`: 5
- `src/core/special_mod_versions/mod.rs`: 4
- `src/seed/mod.rs`: 4
- `src/commands/mod.rs`: 3
- `src/core/scanner/mod.rs`: 3
- `src/core/validator/mod.rs`: 3
- remaining one-off files: `command_gate.rs`, `apply_plan_persistence.rs`, `content_versions`, `move_engine`, `lib.rs`, `apply_plan_backup_prototype`, `filename_parser`

Main Clippy categories:

- behavior-free mechanical lints: derivable `Default`, needless borrow, redundant closure, `repeat_n`, `clamp`, `?`, `is_empty`, needless lifetimes, unnecessary casts, string replace collapse;
- API-shape lints: too-many-arguments helpers;
- dead-code/unused fields that may represent future contracts and need explicit decisions, not blind deletion.

## Phase A — mechanical Clippy cleanup

Goal: remove behavior-free Clippy warnings first.

Likely edits:

- Replace manual `Default` impls with `#[derive(Default)]` where Clippy identified derivable implementations.
- Remove the needless borrow in `src/lib.rs` without changing command-gate behavior.

Files likely touched:

- `src-tauri/src/models.rs`
- `src-tauri/src/seed/mod.rs`
- `src-tauri/src/lib.rs`

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:rust -- apply_plan --lib
git diff --check
```

Commit shape:

```text
chore: remove mechanical Rust clippy warnings
```

## Phase A landing result

Phase A removed the targeted derivable-default and `src/lib.rs` needless-borrow Clippy findings without changing serialized field names, command registration, ApplyPlan validation, command gating, or file-operation boundaries.

Fresh post-Phase-A Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 70 warnings; lib test generated 72 warnings, 64 duplicates
```

The remaining warning debt is Phase B/C/D territory, not part of this mechanical landing slice.

## Phase B — test-build warning cleanup

Goal: remove unused imports, unused locals, and stale parser scratch values that appear only in lib-test builds.

Likely edits:

- `src/core/file_inspector/mod.rs`
  - remove unused imports such as `time::Duration` and `BufReader` if no longer needed;
  - remove or underscore unused locals such as `group_id`, `pos_before`, `start_pos`, and `first_entry_values`;
  - remove dead parser scaffolding only if tests prove no behavior changed.
- `src/core/downloads_watcher/mod.rs`
  - remove unnecessary `mut`;
  - rename truly intentional ignored values with `_` prefix.
- `src/core/apply_plan_persistence.rs`
  - either remove unread fields from `PreviewSnapshotRow` or keep them with explicit intent if they are needed for near-term expiry/audit work.
- `src/commands/mod.rs`
  - decide whether `emit_downloads_progress` is dead or future event plumbing. Delete only if no command/event path needs it.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
cargo test --manifest-path src-tauri/Cargo.toml file_inspector --lib
cargo test --manifest-path src-tauri/Cargo.toml downloads_watcher --lib
npm run test:rust
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
```

Commit shape:

```text
chore: clean Rust test-build warning noise
```

## Phase B landing result

Phase B removed low-risk test-build warning noise in `file_inspector` and `downloads_watcher`: unused imports, stale scratch locals, an unused local parser constant, a no-op Windows timeout scaffold, an intentionally ignored ZIP count binding, and an unnecessary mutable closure binding.

Fresh post-Phase-B Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 60 warnings; lib test generated 62 warnings, 54 duplicates
```

The remaining warning debt is mostly future-contract/dead-code decisions and broader mechanical lints that belong to Phase C/D, not this narrow test-build cleanup slice.

## Phase C — intentional future-contract fields

Goal: distinguish real dead code from fields intentionally kept for future Apply/Restore/API contracts.

Do not blindly delete future-facing fields just to make Rust quiet. Some warning sources are schema/API contract placeholders and should either be used, documented, or explicitly allowed.

Review carefully:

- `PersistedApplyPlanRun.confirmation_token`
- `LibraryFileRow.installed_version`
- `McccUpdateInfo`
- `ApplyMcccUpdateResult`
- seed structs with fields used for external seed shape rather than internal reads
- `ProblemSignalSeverity::Severe`
- `ProblemSignalProofLevel::{Inferred, Unknown}`

Preferred outcomes, in order:

1. if field is truly obsolete: delete it and update tests/API types;
2. if field is part of a serialized or future DB/API contract: add a narrow `#[allow(dead_code)]` with a comment explaining the contract;
3. if field should be displayed or consumed soon: add a test-backed usage path instead of an allow.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:unit
npm run test:rust
git diff --check
```

Commit shape:

```text
chore: document intentional future Rust contracts
```

## Phase C landing result

Phase C classified the future-contract/dead-code warning surface instead of deleting safety vocabulary blindly:

- kept and documented command-gate capability taxonomy variants used for safety review vocabulary;
- kept and documented preview snapshot audit/expiry row hooks without enabling expiry enforcement;
- kept `PersistedApplyPlanRun.confirmation_token` as a future backend-issued token contract and still skipped serialization;
- kept future serialized problem-signal vocabulary with narrow comments/allow markers;
- kept external seed-shape provenance/help/example fields so curated catalog metadata remains required by serde;
- removed stale direct-MCCC update result structs that had no command/API references;
- removed the inert, skipped `LibraryFileRow.installed_version` field from list rows rather than pretending list queries populate it.

Fresh post-Phase-C Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 49 warnings; lib test generated 52 warnings, 45 duplicates
```

Fresh post-Phase-C `cargo check` summary:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 13 warnings
```

The remaining warning debt is now mostly broader mechanical Clippy cleanup, test helper argument-shape warnings, and a few deferred dead-code decisions (`emit_downloads_progress`, downloads watcher source rows, thumbnail/cache helpers, special-mod version helpers).

## Phase D-prep landing result

Phase D-prep removed the low-risk mechanical Clippy warning surface that was still obscuring review output without installing a hard warning gate yet:

- replaced manual clamp, `repeat().take()`, redundant closure, `?`, `is_empty`, needless lifetime, needless borrow, unnecessary cast, collapsed replace, and `div_ceil` patterns;
- kept command registration, command gating, ApplyPlan validation/provenance, backup/restore prototypes, and user-file operation boundaries unchanged;
- deliberately left dead-code/future-contract decisions and too-many-arguments helper refactors for separate, higher-context slices.

Fresh post-Phase-D-prep Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 20 warnings; lib test generated 22 warnings, 16 duplicates
```

Fresh post-Phase-D-prep `cargo check` summary:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
# exit 0; simsuite lib generated 13 warnings
```

The remaining backlog is now intentionally concentrated: dead-code/future-contract decisions plus too-many-arguments helper shape. A budget gate should still wait until those are resolved or explicitly budgeted.

## Phase D-classification landing result

Phase D-classification resolved the remaining warning noise without widening file-operation behavior or external command contracts:

- deleted stale thumbnail/cache scaffolding that had no call sites (`MAX_THUMBNAIL_BYTES`, `resource_label`, and the unused `DbpfIndexEntry` wrapper);
- moved test-only wrappers behind `#[cfg(test)]` where production code already uses the cached/context-aware path (`collect_supported_files_with_progress`, `assess_download_item`, `evaluate_download_item`);
- kept intentional future/API contracts with explicit local `#[allow(dead_code)]` annotations and comments (`downloads-progress` event emission, staged download source-row audit fields, thumbnail resource diagnostics, and special-mod version comparison details/helpers);
- kept helper-shaped `too_many_arguments` functions as explicit local Clippy allows where grouping would be a behavior-risking refactor rather than a cleanup.

Fresh post-Phase-D-classification `cargo check` summary:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
# exit 0; no warnings
```

Fresh post-Phase-D-classification Clippy summary:

```bash
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
# exit 0; no warnings
```

The Rust warning surface is now clean enough for a zero-warning budget gate.

## Phase D — warning budget gate

Goal: make new warnings harder to introduce after the backlog is cleaned.

Only do this when `cargo check`, `cargo test apply_plan --lib`, and `cargo clippy` are clean or have an intentionally documented tiny budget.

Options:

- add a repo script that counts warnings and fails above the current budget;
- later upgrade to `-D warnings` for selected modules;
- keep full `-D warnings` for the whole crate as a final step, not the first step.

Verification gate:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
npm run test:rust
```

Commit shape:

```text
chore: add Rust warning budget gate
```

## Phase D landing result

Phase D added a small repo-local zero-warning gate without hiding the underlying Cargo commands:

- added `scripts/test/run-rust-warning-budget.mjs`;
- added focused Node tests for the warning-budget command construction;
- added `npm run test:rust:warnings` as the local/CI entry point;
- the gate runs `cargo check --manifest-path src-tauri/Cargo.toml` with `RUSTFLAGS=-Dwarnings`, then `cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml -- -D warnings`.

Fresh Phase-D gate evidence:

```bash
node --test scripts/test/run-rust-warning-budget.test.mjs
# 3 passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed
```

## Phase E — ApplyPlan module boundary cleanup

Only start this after warning cleanup and the zero-warning gate. This is where risk increases.

Likely extraction targets:

- persistence row mapping / strict JSON parsing helpers;
- preview snapshot repository functions;
- ApplyPlan item insert/load helpers;
- hash/provenance helpers that are currently stable enough to isolate;
- command wrappers vs core functions.

Rules:

- one extraction per commit;
- no behavior change without a failing test first;
- keep command-gate tests green after every extraction;
- do not rename externally serialized fields unless frontend/API tests prove the contract.

Verification gate:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
npm run test:rust
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
git diff --check
```

Commit shape:

```text
refactor: split ApplyPlan persistence boundaries
```

## Recommended immediate next step

Phase E slices completed so far:

- slice 1 added `src-tauri/src/core/apply_plan_persistence_json.rs`;
- slice 1 moved persisted JSON parsing/error-message construction for ApplyPlan rows and preview snapshot rows behind typed helpers;
- slice 2 added `src-tauri/src/core/apply_plan_persistence_rows.rs`;
- slice 2 moved ApplyPlan list/full-plan/item row mapping and persisted status/boolean decode helpers behind the row-mapping module;
- slice 3 added `src-tauri/src/core/apply_plan_preview_snapshots.rs`;
- slice 3 moved preview snapshot row mapping, snapshot loading, and consumed-marker update behind a narrow preview snapshot repository module;
- slice 4 added `src-tauri/src/core/apply_plan_items.rs`;
- slice 4 moved ApplyPlan item preparation, insert/load repository helpers, item row mapping, signal loading, and blocker loading behind a dedicated item persistence module;
- slice 5 added `src-tauri/src/core/apply_plan_hash_persistence.rs`;
- slice 5 moved ApplyPlan hash/provenance storage and loaded-plan verification behind a narrow repository helper module;
- slice 6 added `src-tauri/src/core/apply_plan_records.rs`;
- slice 6 moved ApplyPlan list summaries, saved-plan list-item lookup, status string mapping, status updates, and draft cancellation behind a narrow record/lifecycle helper module;
- slice 7 added `src-tauri/src/core/apply_plan_loading.rs`;
- slice 7 moved full saved ApplyPlan row fetch, JSON rehydration, persisted status/boolean conversion, item loading, and final `PersistedApplyPlan` assembly behind a narrow loading module while keeping public `get_apply_plan` as the compatibility entry point;
- slice 8 added `src-tauri/src/core/apply_plan_saves.rs`;
- slice 8 moved preview ApplyPlan draft save orchestration and preview source file-touch rejection behind a narrow save helper module while keeping public save entry points compatibility-stable;
- slice 9 moved backend sorting-preview snapshot creation/persistence into `src-tauri/src/core/apply_plan_preview_snapshots.rs` while keeping public `generate_sorting_preview_snapshot` compatibility-stable;
- slice 10 added `src-tauri/src/core/apply_plan_preview_snapshot_consumption.rs`;
- slice 10 moved preview-snapshot consumption/save validation, snapshot JSON rehydration, provenance recheck, draft save, and consumed-marker update behind a focused helper while keeping public `save_apply_plan_from_preview_snapshot` compatibility-stable;
- slice 11 added `src-tauri/src/core/apply_plan_builders.rs`;
- slice 11 moved backend builder generate-and-save orchestration behind a focused helper while keeping public `build_apply_plan_from_staging_plan` compatibility-stable and still binding builder saves to backend preview snapshots;
- slice 12 moved the backend builder boundary test out of the public façade test module and into `src-tauri/src/core/apply_plan_builders.rs`, then removed the now-unused façade test helper;
- slice 13 moved the preview snapshot row-mapper and backend preview snapshot creation tests into `src-tauri/src/core/apply_plan_preview_snapshots.rs`, and moved the preview snapshot consumption test into `src-tauri/src/core/apply_plan_preview_snapshot_consumption.rs`;
- slice 14 moved the draft-save helper boundary test into `src-tauri/src/core/apply_plan_saves.rs`, renamed it to `helper_persists_draft_items_and_hash_metadata`, and kept the public façade free of that helper-owned assertion surface;
- slice 15 moved the ApplyPlan hash-stamp persistence boundary test into `src-tauri/src/core/apply_plan_hash_persistence.rs`, renamed it to `helper_persists_hash_columns_without_rewriting_updated_at`, and kept the public façade free of that helper-owned assertion surface;
- slice 16 moved the remaining record/lifecycle helper tests into `src-tauri/src/core/apply_plan_records.rs`, covering status updates, cancellation, and list-summary cancellation visibility from the record module;
- slice 17 moved persisted JSON parser, ApplyPlan row mapper, and item row mapper tests into `src-tauri/src/core/apply_plan_persistence_json.rs`, `src-tauri/src/core/apply_plan_persistence_rows.rs`, and `src-tauri/src/core/apply_plan_items.rs`;
- slice 18 moved save-helper rejection and draft-candidate item tests into `src-tauri/src/core/apply_plan_saves.rs`;
- slice 19 moved saved-plan loading and malformed persisted JSON failure tests into `src-tauri/src/core/apply_plan_loading.rs`;
- slice 20 moved ApplyPlan hash verification tamper tests into `src-tauri/src/core/apply_plan_hash_persistence.rs` and preview snapshot provenance hash binding into `src-tauri/src/core/apply_plan_provenance.rs`;
- all twenty slices kept stored column names, externally serialized fields, hash/provenance semantics, command gates, and file-operation boundaries unchanged;
- focused tests now cover strict persisted JSON context, ApplyPlan row mapper status/flag/JSON-column preservation, preview snapshot hash/audit-column row mapping from the preview snapshot module's own tests, backend preview snapshot creation/persistence from the preview snapshot module's own tests, preview snapshot consumption from the snapshot consumption module's own tests, backend builder snapshot-bound draft creation from the builder module's own tests, draft save orchestration and rejection behavior from the save helper module's own tests, ApplyPlan item status/validation-column row mapping, hash/provenance column persistence and tamper detection from the hash persistence module's own tests, preview snapshot provenance binding from the provenance module's own test, status-update/list/cancel behavior through the record helper module, and full saved ApplyPlan loading/malformed JSON failure through the loading module.

Fresh Phase-E slices 16-20 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml helper_updates_status_timestamp_without_rewriting_hash --lib
# 1 passed; test now runs from core::apply_plan_records::tests

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_records --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml "parser_reports_column_and_record_context" --lib
cargo test --manifest-path src-tauri/Cargo.toml "row_mappers_preserve_status_flags_and_json_columns" --lib
cargo test --manifest-path src-tauri/Cargo.toml "row_mapper_preserves_status_and_validation_columns" --lib
# each focused relocated helper test passed from its helper module

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_saves --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_loading --lib
# 2 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_hash_persistence --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_provenance --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml helper_malformed_persisted_plan_json_fails_closed --lib
# 1 passed; test now runs from core::apply_plan_loading::tests

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# 3 files passed; 39 tests passed

cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy --all-targets --all-features -D warnings passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 15 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml helper_persists_hash_columns_without_rewriting_updated_at --lib
# 1 passed; test now runs from core::apply_plan_hash_persistence::tests

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_hash --lib
# 2 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 14 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml helper_persists_draft_items_and_hash_metadata --lib
# 1 passed; test now runs from core::apply_plan_saves::tests

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 13 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml row_mapper_preserves_hash_and_audit_columns --lib
# 1 passed; test now runs from core::apply_plan_preview_snapshots::tests

cargo test --manifest-path src-tauri/Cargo.toml creator_persists_backend_preview_hash_and_audit_columns --lib
# 1 passed; test now runs from core::apply_plan_preview_snapshots::tests

cargo test --manifest-path src-tauri/Cargo.toml consumer_saves_backend_snapshot_and_marks_consumed --lib
# 1 passed; test now runs from core::apply_plan_preview_snapshot_consumption::tests

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 12 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml builder_module_generates_snapshot_bound_draft --lib
# 1 passed; test now runs from core::apply_plan_builders::tests

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 11 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml apply_plan_builder_module_generates_snapshot_bound_draft --lib
# RED before helper extraction: failed to resolve `apply_plan_builders` in `core`; GREEN after extraction: 1 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# 3 files passed; 39 tests passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 10 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_consumer_module_saves_backend_snapshot_and_marks_consumed --lib
# RED before helper extraction: failed to resolve `apply_plan_preview_snapshot_consumption` in `core`; GREEN after extraction: 1 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 114 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# 3 files passed; 39 tests passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 380 passed; 0 failed; 2 ignored
```

Historical Phase-E slice 9 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_creator_persists_backend_preview_hash_and_audit_columns --lib
# RED before helper extraction: failed to resolve `generate_sorting_preview_snapshot` in `core::apply_plan_preview_snapshots`; GREEN after extraction: 1 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save_binds_backend_snapshot_and_rejects_reuse --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml generated_preview_snapshots_saved_twice_have_same_plan_hash --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml saving_preview_plan_creates_plan_items_signals_and_blockers --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 113 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# 3 files passed; 39 tests passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 379 passed; 0 failed; 2 ignored

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0
```

Historical Phase-E slice 8 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml apply_plan_save_helper_persists_draft_items_and_hash_metadata --lib
# RED before helper extraction: failed to resolve `apply_plan_saves` in `core`; GREEN after extraction: 1 passed

cargo test --manifest-path src-tauri/Cargo.toml saving_preview_plan_creates_plan_items_signals_and_blockers --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save_binds_backend_snapshot_and_rejects_reuse --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml saved_plan_hash_verification_detects_tampered_context_and_destination --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 112 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# 3 files passed; 39 tests passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 378 passed; 0 failed; 2 ignored

cargo fmt --all --check --manifest-path src-tauri/Cargo.toml && git diff --check
# exit 0
```

Historical Phase-E slice 7 evidence:

```bash
cargo test --manifest-path src-tauri/Cargo.toml saved_apply_plan_loader_assembles_items_json_and_provenance_without_mutating_record --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml saving_preview_plan_creates_plan_items_signals_and_blockers --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml saved_plan_hash_verification_detects_tampered_context_and_destination --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml malformed_persisted_plan_json_fails_closed --lib
# 1 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan --lib
# 111 passed

cargo test --manifest-path src-tauri/Cargo.toml command_gate --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml apply_plan_path_validation --lib
# 4 passed

cargo test --manifest-path src-tauri/Cargo.toml preview_snapshot_save --lib
# 3 passed

npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/lib/api.test.ts src/trustBoundaryCopy.test.ts
# passed from a /tmp rsync copy after `npm ci --ignore-scripts`: 3 files passed; 39 tests passed

npm run test:rust:warnings
# exit 0; cargo check warning budget passed; cargo clippy warning budget passed

npm run test:rust
# 377 passed; 0 failed; 2 ignored
```

Phase E public façade review checkpoint:

- reviewed the remaining `src-tauri/src/core/apply_plan_persistence.rs` tests after slices 1-20;
- kept `preview_snapshot_save_binds_backend_snapshot_and_rejects_reuse` as public end-to-end compatibility coverage for façade snapshot creation -> snapshot save -> public load -> reuse rejection;
- kept `generated_preview_snapshots_saved_twice_have_same_plan_hash` as public deterministic-identity coverage across generated snapshots and saved drafts;
- kept `preview_snapshot_save_rejects_hash_mismatch` and `preview_snapshot_save_rejects_rule_drift_before_persistence` as public fail-closed façade coverage for caller hash mismatch and post-preview provenance drift;
- kept `saving_preview_plan_creates_plan_items_signals_and_blockers` as public save/get serialization compatibility coverage for client-supplied preview drafts, item persistence, signal persistence, blocker persistence, and hash metadata exposure through the façade;
- kept `no_execution_status_can_be_inserted_by_schema` as the façade-adjacent schema boundary proving execution statuses remain unavailable to saved draft records;
- kept `backend_generated_preview_saved_twice_has_same_hash` as public backend-builder compatibility coverage for generated draft source-kind enforcement and deterministic saved hashes;
- kept `persisted_plan_serialization_exposes_hash_metadata_not_full_provenance` as frontend/API contract coverage proving normal persisted-plan serialization exposes hash metadata while keeping full provenance server-side;
- no remaining public façade test is currently classified as helper-owned boundary coverage that should move before the checkpoint.

Recommended next Phase E checkpoint:

- if keeping this checkpoint, commit the Phase E extraction/test-colocation work before starting a new behavior slice;
- keep public save/get/list/cancel/status APIs compatibility-stable;
- keep externally serialized fields unchanged;
- keep command-gate, ApplyPlan validation/provenance, path-validation, `npm run test:rust`, and `npm run test:rust:warnings` green.

After that review/commit checkpoint, Operation-set Preview V1 can start without the ApplyPlan persistence façade carrying most helper-owned assertions. Tiny cleanup done; sharper knives may now leave the drawer, carefully.
