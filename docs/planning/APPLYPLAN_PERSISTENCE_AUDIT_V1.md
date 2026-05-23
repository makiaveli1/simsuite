# ApplyPlan Persistence Audit v1

Date: 2026-05-15

This audit designs how SimSuite should eventually persist reviewed `ApplyPlan`
data, blocked items, evidence snapshots, validation and conflict results,
backup/restore references, and per-file result logs before any real Apply
workflow exists.

Real Apply is not implemented. No files changed. This document does not add a
SQLite migration, Tauri command, UI, file movement, cleanup, delete,
quarantine, replacement, auto-sort, or AI decision.

Current implementation note: the follow-up sprint
`codex/applyplan-persistence-foundation-v1` implemented the first DB-only
foundation from this audit. SimSuite now has runtime storage for draft/preview
ApplyPlan records, items, source-signal snapshots, and blocker snapshots. That
foundation still does not implement real Apply, file movement, result logs,
restore entries, or visible saved-plan UI.

Current implementation note: the follow-up builder sprint
`codex/applyplan-builder-from-stagingplan-v1` added a backend-owned DB-only path
that generates a read-only sorting preview plan and saves it as a draft
ApplyPlan snapshot. It still does not implement real Apply, file movement,
result logs, restore entries, validation/conflict execution, or visible
saved-plan UI.

Current implementation note: the saved-plan UI review sprint
`codex/saved-plan-ui-review-v1` adds the first visible Organize review surface
for saved draft preview records. Users can save a generated preview plan, list
saved drafts, inspect details/evidence/blockers, and cancel drafts. This UI
uses the existing DB-only builder and persistence commands and still does not
implement real Apply, file movement, result logs, restore entries, or
validation/conflict execution.

Current planning note: validation/conflict preview design now lives in
`docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`. That design defines the
future statuses, checks, command shape, and UI expectations needed before saved
draft plans can ever be considered for confirmation. It does not add runtime
validation, conflict checks, result logs, restore entries, or real Apply.

Current implementation note: `codex/read-only-applyplan-validation-preview-v1`
adds the first read-only `preview_apply_plan_validation` command. It loads
saved draft records, compares item snapshots to current Library/settings
evidence, returns validation/conflict preview results, and keeps
`canProceedToConfirmation=false`. It does not persist validation state, create
folders or backups, expose Apply, or change files.

Current implementation note: Organize `Saved plans` now exposes the first
visible validation/conflict preview UI for saved draft records. It calls
`previewApplyPlanValidation`, shows blockers, conflicts, caveats, and summary
counts, and keeps `canProceedToConfirmation=false` visible as `Future
confirmation blocked`. It does not add Apply, persist validation state, or
change files.

Current planning note: backup/restore/result-log design now lives in
`docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`. That design refines the
future `apply_plan_runs`, `apply_plan_results`, and
`apply_plan_restore_entries` approach and recommends copy-backup-first recovery
for a future v1. It does not implement those tables or real Apply.

Current implementation note: `codex/result-restore-schema-foundation-v1` adds
the DB-only foundation for `apply_plan_runs`, `apply_plan_results`, and
`apply_plan_restore_entries`. The new records are future run/result/restore
metadata only. They do not execute Apply, backup, restore, file movement, file
copying, folder creation, deletion, cleanup, quarantine, replacement, or AI
decisions.

Current implementation note: `codex/fixture-only-backup-prototype-v1` adds a
private backend prototype that copies and verifies temporary test files only,
then records safe `pending_log` / `design_only` metadata. It does not expose
commands or UI, does not touch user files, and does not make Apply or Restore
ready.

Current implementation note: `codex/fixture-only-restore-prototype-v1` extends
that private prototype with restore-copy proof from recorded backup references
to temporary fixture targets only. It records only safe `pending_log`,
`failed_before_change`, and `design_only` metadata, does not expose commands or
UI, does not touch user files, and does not make Apply or Restore ready.

Current implementation note:
`codex/fixture-backup-restore-integration-proof-v1` proves the private backup
and restore prototypes together as one temporary-file recovery chain. It
verifies backup and restore copies, proves restore-map scope against one
run/plan/item/result context, records only safe metadata, does not expose
commands or UI, does not touch user files, and does not make Apply or Restore
ready.

Current implementation note: `codex/result-restore-review-ui-v1` exposes the
first read-only Organize `Recovery history` UI for existing DB-only run,
result, and restore-map records. It reuses result/restore list APIs only. It
does not create records, run backup, run restore, expose fixture helpers,
execute Apply, or change files.

This audit follows:

- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`

## 1. Purpose

ApplyPlan persistence is how SimSuite will remember a reviewed plan, what would
change, what is blocked, what needs confirmation, and what happened later.

The future persisted model must be separate from `StagingPlan`.

- `StagingPlan` is preview-only and keeps `wouldTouchFiles=false`.
- Future `ApplyPlan` is the confirmed file-changing contract only after the
  Apply Safety Contract is implemented and proven.
- Saving an ApplyPlan draft must not move, copy, delete, quarantine, clean up,
  replace, or auto-sort files.

The main persistence goal is to make future Apply work reviewable,
recoverable, and auditable before a file-changing executor is ever exposed.

## 2. Current Source Systems

Future ApplyPlan persistence must reuse existing SimSuite systems before adding
new data paths.

| Source system | Existing evidence | How future ApplyPlan persistence should use it |
| --- | --- | --- |
| `StagingPlan` | Preview-only plan items, evidence levels, buckets, caveats, source signals, blocked reasons, current root, suggested destination strings, `wouldTouchFiles=false` | Source data for a future ApplyPlan draft. Do not treat it as Apply-ready by itself. |
| `generate_sorting_preview_plan` | Read-only organization suggestions for selected Library files or bounded folder scopes | Source of reviewed organization suggestions. Persist a snapshot only after user review or explicit save. |
| Library file identity | `files.id`, filename, extension, path, source root, size/date, hash where indexed | Stable references for items, validation, and later current-state comparison. |
| Library file detail | Deeper evidence surface with duplicate, review, update, preview, and inspection facts | Optional evidence snapshot for items that need richer explanation or blockers. |
| Scanner/indexed paths | Current indexed paths, source roots, folder metadata, scan-time facts | Input for path validation and stale-plan detection. |
| Duplicate detector | Exact duplicate proof and review-only comparison rows | Exact duplicate proof may block or annotate items; name/version rows remain review-only. |
| Review queue | Manual review membership and reasons | Block or mark items as review-only before any future Apply. |
| Parser and inspection warnings | Scan/file-inspector warning facts | Persist as blocked or review-only context; never convert to file-changing certainty. |
| Updates/watch state | Watch source state, checker state, no-source state, failed checks | Persist as context/caveat only; not update replacement proof. |
| Inbox intake state | New/downloaded/imported batch state | May seed future reviewed plans, but intake data is not an organization plan by itself. |
| Snapshot/restore prior art | `snapshot_manager`, `snapshots`, `snapshot_items`, and `move_engine::restore_snapshot` | Useful recovery primitives and schema patterns, but not sufficient for visible Apply v1. |
| Apply Safety Contract | Required preview, confirmation, backup/restore, path validation, conflicts, result logs, and proof | The gate future persistence and executor work must satisfy. |
| Existing Systems Integration Contract | Anti-duplication and reuse rules | Prevents parallel parsers, duplicate truth, folder truth, or UI-owned file-action logic. |

## 3. Existing systems reused

| System | What it already provides | How ApplyPlan persistence should use it | What not to duplicate |
| --- | --- | --- | --- |
| Scanner | Indexed file identity, roots, size/date, hash state, real folder metadata, parser warning context | Store file id/path snapshots and scan/session timestamps when available | Do not rescan or infer roots in UI or persistence code. |
| File inspector | Package/script/Tray metadata, fingerprints, inspection warnings, preview clues | Snapshot trusted metadata and warnings for explanation and blockers | Do not reimplement DBPF or script archive parsing. |
| Library index | Paged rows, direct folder files, detail surfaces, review/update/duplicate cues | Use Library ids and detail facts as the source of current truth | Do not create new unreviewed file lists. |
| Duplicate detector | Exact duplicate truth and review-only comparison rows | Store exact duplicate context as a blocker/caveat; store name/version rows as review-only context | Do not reimplement duplicate truth. |
| Review queue | Manual review reasons and membership | Persist review queue membership as a blocked or review-only reason | Do not create feature-private review queues. |
| Updates/watch | Source setup and checker state | Persist update/watch context as a caveat for future review | Do not invent update-source truth. |
| Inbox | Download/import intake state | Link future plans to intake origin where helpful | Do not present raw intake batches as saved organization plans. |
| Organize | Preview organization planning workspace | Own future saved-plan and ready-for-confirmation surfaces | Do not expose direct file actions from UI. |
| Plan Preview/Pending Plans | Preview/pending work only | May display saved preview or pending plan state later | Do not call staging commit/cleanup from this surface. |
| Snapshot/restore prior art | Existing snapshot tables and restore helper | Inform backup/restore table design and restore-map requirements | Do not assume current snapshots are a complete Apply safety model. |

## 4. Data That Must Be Persisted Later

Future ApplyPlan persistence should preserve these concepts:

- saved preview plan.
- `ApplyPlan`.
- `ApplyPlanItem`.
- blocked item.
- review-only item.
- caveat and source-signal snapshot.
- confirmation state.
- path validation result.
- conflict result.
- backup/restore reference.
- per-file execution result.
- result log.
- restore/undo map.

Persisted data must distinguish three things clearly:

- what the preview suggested.
- what the future ApplyPlan decided was eligible, blocked, or review-only.
- what actually happened during a future Apply run.

## 5. Proposed Future Schema

Audit-time decision: this schema was design-only and no migration was added in
the audit sprint. Follow-up status: `codex/applyplan-persistence-foundation-v1`
implemented the first four foundation tables and deferred result/restore
tables.

### `apply_plans`

Purpose: one saved reviewed plan, including source plan metadata, lifecycle
state, item counts, confirmation requirements, backup requirements, and plan
caveats.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `source_staging_plan_id TEXT`
- `source_plan_kind TEXT NOT NULL`
- `title TEXT NOT NULL`
- `summary TEXT NOT NULL`
- `status TEXT NOT NULL`
- `would_touch_files INTEGER NOT NULL DEFAULT 1`
- `confirmation_required INTEGER NOT NULL DEFAULT 1`
- `backup_required INTEGER NOT NULL DEFAULT 1`
- `restore_available INTEGER NOT NULL DEFAULT 0`
- `total_items INTEGER NOT NULL DEFAULT 0`
- `applyable_items INTEGER NOT NULL DEFAULT 0`
- `blocked_items INTEGER NOT NULL DEFAULT 0`
- `review_only_items INTEGER NOT NULL DEFAULT 0`
- `caveats_json TEXT NOT NULL DEFAULT '[]'`
- `source_scope_json TEXT`
- `scan_session_id INTEGER REFERENCES scan_sessions(id) ON DELETE SET NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `idx_apply_plans_status_created_at(status, created_at)`
- `idx_apply_plans_source_staging_plan_id(source_staging_plan_id)`

Privacy and retention notes:

- Store plan-level metadata only here.
- Item paths and restore paths belong to item/result tables.
- Draft retention should be a product decision; do not silently delete plan
  records that may be needed for audit or restore.

Why existing tables are not enough:

- `StagingPlan` is not persisted and is preview-only.
- `snapshots` describe recovery prior art, not a reviewed future Apply
  contract.

### `apply_plan_items`

Purpose: one row per future file action candidate, blocked item, or
review-only item.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_id INTEGER NOT NULL REFERENCES apply_plans(id) ON DELETE CASCADE`
- `source_item_id TEXT`
- `file_id INTEGER REFERENCES files(id) ON DELETE SET NULL`
- `file_name TEXT NOT NULL`
- `current_path TEXT NOT NULL`
- `current_root TEXT NOT NULL`
- `destination_path TEXT`
- `destination_root TEXT`
- `action_kind TEXT NOT NULL`
- `evidence_level TEXT NOT NULL`
- `bucket TEXT`
- `confidence_label TEXT`
- `item_status TEXT NOT NULL`
- `blocked INTEGER NOT NULL DEFAULT 0`
- `review_only INTEGER NOT NULL DEFAULT 0`
- `validation_status TEXT`
- `conflict_status TEXT`
- `path_privacy_level TEXT NOT NULL DEFAULT 'local_full_path_required'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `idx_apply_plan_items_plan_id(apply_plan_id)`
- `idx_apply_plan_items_file_id(file_id)`
- `idx_apply_plan_items_status(item_status)`

Privacy and retention notes:

- Full paths are needed for future validation and restore.
- UI should show shortened paths by default and reveal full paths only in
  details.

Why existing tables are not enough:

- `files` stores current indexed facts, not a reviewed future action contract.
- `StagingPlanItem` is a preview suggestion, not a persisted Apply candidate.

### `apply_plan_item_signals`

Purpose: normalized evidence/source-signal snapshots for each item.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items(id) ON DELETE CASCADE`
- `signal_kind TEXT NOT NULL`
- `signal_label TEXT NOT NULL`
- `signal_value TEXT`
- `evidence_level TEXT`
- `source_system TEXT NOT NULL`
- `created_at TEXT NOT NULL`

Indexes:

- `idx_apply_plan_item_signals_item_id(apply_plan_item_id)`
- `idx_apply_plan_item_signals_kind(signal_kind)`

Why existing tables are not enough:

- The Library can change after a plan is saved. A source-signal snapshot keeps
  the reviewed explanation tied to the plan.

### `apply_plan_item_blockers`

Purpose: normalized blocked reasons, review-only reasons, conflict reasons, and
validation failures.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items(id) ON DELETE CASCADE`
- `blocker_kind TEXT NOT NULL`
- `reason_code TEXT NOT NULL`
- `message TEXT NOT NULL`
- `source_system TEXT NOT NULL`
- `created_at TEXT NOT NULL`

Indexes:

- `idx_apply_plan_item_blockers_item_id(apply_plan_item_id)`
- `idx_apply_plan_item_blockers_reason_code(reason_code)`

Why existing tables are not enough:

- Review queue rows and warnings are source facts. ApplyPlan blockers need to
  preserve the decision made for this plan at this time.

### `apply_plan_results`

Purpose: per-file execution/result-log entries for future Apply runs.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_id INTEGER NOT NULL REFERENCES apply_plans(id) ON DELETE CASCADE`
- `apply_plan_item_id INTEGER REFERENCES apply_plan_items(id) ON DELETE SET NULL`
- `run_id TEXT NOT NULL`
- `operation_kind TEXT NOT NULL`
- `result_status TEXT NOT NULL`
- `started_at TEXT`
- `finished_at TEXT`
- `source_path_at_execution TEXT`
- `destination_path_at_execution TEXT`
- `bytes_processed INTEGER`
- `error_code TEXT`
- `error_message TEXT`
- `created_at TEXT NOT NULL`

Indexes:

- `idx_apply_plan_results_run_id(run_id)`
- `idx_apply_plan_results_plan_id(apply_plan_id)`
- `idx_apply_plan_results_item_id(apply_plan_item_id)`
- `idx_apply_plan_results_status(result_status)`

Privacy and retention notes:

- Result logs should avoid unnecessary private path export.
- Local logs may need full paths to support recovery and debugging.

Why existing tables are not enough:

- Current snapshots do not record a full per-item Apply run result lifecycle.

### `apply_plan_restore_entries`

Purpose: restore/undo map entries scoped to one future Apply run and item.

Key columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_result_id INTEGER NOT NULL REFERENCES apply_plan_results(id) ON DELETE CASCADE`
- `apply_plan_item_id INTEGER REFERENCES apply_plan_items(id) ON DELETE SET NULL`
- `snapshot_id INTEGER REFERENCES snapshots(id) ON DELETE SET NULL`
- `snapshot_item_id INTEGER REFERENCES snapshot_items(id) ON DELETE SET NULL`
- `file_id INTEGER REFERENCES files(id) ON DELETE SET NULL`
- `original_path TEXT NOT NULL`
- `applied_path TEXT NOT NULL`
- `backup_path TEXT`
- `original_hash TEXT`
- `applied_hash TEXT`
- `restore_status TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `restored_at TEXT`

Indexes:

- `idx_apply_plan_restore_entries_result_id(apply_plan_result_id)`
- `idx_apply_plan_restore_entries_item_id(apply_plan_item_id)`
- `idx_apply_plan_restore_entries_snapshot_id(snapshot_id)`
- `idx_apply_plan_restore_entries_status(restore_status)`

Privacy and retention notes:

- Restore entries contain sensitive local paths and must remain local.
- Future export/report UI should shorten paths unless the user opens details.

Why existing tables are not enough:

- `snapshot_items` can store backup references, but it does not connect a
  restore entry to a reviewed ApplyPlan item and run result by itself.

## 6. ApplyPlan Lifecycle

V1 persistence design states:

- `draft`: a saved plan exists, but it is not ready for confirmation.
- `preview_only_source`: the plan came from preview-only data and has not been
  converted to an Apply candidate.
- `blocked`: every item is blocked or the plan lacks required safety data.
- `ready_for_confirmation`: the plan passed future validation and may be shown
  for explicit confirmation. No files have changed yet.
- `cancelled`: the user or system cancelled the saved plan before Apply.

Future executor states:

- `confirmed`: the user explicitly confirmed a ready plan.
- `applying`: a future executor is processing items.
- `partially_applied`: at least one item succeeded and at least one failed or
  was skipped.
- `applied`: all eligible items completed.
- `failed`: the Apply run failed before completing eligible items.
- `restore_available`: enough restore entries exist for recovery.
- `restoring`: restore is in progress.
- `restored`: restore completed for the tracked run.
- `restore_failed`: restore attempted but did not complete.

The executor states are future-only. They should not be used until backup,
restore, path validation, conflict handling, and result logging are implemented
and tested.

## 7. Evidence Snapshot Strategy

Future ApplyPlan persistence should snapshot evidence at plan creation time.

Required evidence snapshot fields:

- file id.
- file name.
- current path.
- destination path, if any.
- evidence level.
- source signals.
- blocked reasons.
- duplicate context.
- review queue context.
- update/watch context.
- scan session id or indexed timestamp when available.
- caveats.

Reason:

A user's Library can change after a plan is created. Future Apply must compare
persisted evidence against current indexed state before touching files. If the
source file moved, changed hash, disappeared, gained a review blocker, or now
conflicts with a destination, the plan item must be blocked or revalidated.

## 8. Path Privacy And Safety

Future persistence should store paths with deliberate scope:

- store full current paths only where validation and restore need them.
- store full destination paths only for planned file-changing candidates.
- store full backup paths only in restore entries and local result logs.
- store normalized roots such as `mods`, `tray`, `downloads`, `inbox`, or
  `unknown` for grouping and filtering.
- prefer shortened paths in UI and exported reports.
- reveal full paths only in details or explicit inspection views.
- do not write private user paths into committed fixtures, public reports, or
  telemetry.

Path validation remains required before any future Apply:

- source path must be under a configured safe root or known app-local intake
  root.
- destination path must be under a configured Mods/Tray root.
- no path traversal.
- no overwrite without conflict handling.
- no review-only, blocked, or weak heuristic item may move.
- no unsafe symlink/reparse-point behavior without explicit support and tests.

## 9. Migration Recommendation

Audit-time decision: no migration in the audit sprint.

Follow-up status: the DB-only ApplyPlan persistence foundation now has a
runtime migration and `ensure_schema` repair for draft/preview storage only.
Result-log and restore-entry tables should still wait for a later
backup/result-log sprint.

Future implementation should include:

- a proper migration entry under `database/migrations`.
- compatibility repair in `ensure_schema`.
- table and index tests.
- fixture database upgrade tests.
- no file movement from migration, save, list, get, or validate commands.

The existing `database/schema/simsuite-v1.sql` mirror may be stale relative to
runtime schema repair, so future migration work must inspect both the migration
file and `src-tauri/src/database/mod.rs`.

## 10. API And Command Recommendation

Audit-time command recommendation:

| Command | Classification | File movement allowed? | Notes |
| --- | --- | --- | --- |
| `save_apply_plan_preview` | State-changing DB-only | No | Saves reviewed preview data and evidence snapshots. |
| `list_saved_apply_plans` | Read-only | No | Lists saved plans and statuses. |
| `get_apply_plan` | Read-only | No | Loads one saved plan with items, blockers, signals, and result summaries. |
| `delete_draft_apply_plan` | State-changing DB-only | No | Deletes or cancels a draft that has never applied files. |
| `validate_apply_plan` | Read-only by default | No | May return validation/conflict results without saving. Saving validation output must be an explicit DB-only design. |
| `build_apply_plan_from_staging_plan` | Read-only draft or state-changing DB-only | No | Converts a reviewed `StagingPlan` snapshot into an ApplyPlan draft, while preserving blockers and review-only exclusions. |

Follow-up status: `save_apply_plan_preview`, `list_saved_apply_plans`,
`get_apply_plan`, and `delete_draft_apply_plan` now exist as DB-only/read-only
foundation commands. `build_apply_plan_from_staging_plan` now exists as a
backend-owned DB-only builder that generates a sorting preview plan and saves a
draft ApplyPlan snapshot. `validate_apply_plan` remains future work.

All future commands must avoid:

- move-engine apply paths.
- existing mutating staging/download commands.
- cleanup/reject/restore paths.
- shell operations.
- real file writes.

## 11. UI Recommendation

No UI is added in this sprint.

Current UI ownership now keeps the first saved-plan review surface in Organize:

- Organize `Create plan`: generate preview-only organization suggestions.
- Organize `Saved plans`: list saved draft preview records, load details,
  display evidence/blockers/caveats, and cancel drafts without touching files.
- Organize `Pending batches`: summarize imported/downloaded batch handoff state
  and point detailed intake review back to Inbox.
- Future `Ready for confirmation`: show exact source/destination changes,
  backup/restore status, excluded blocked/review-only items, and explicit
  confirmation.
- Future `Result log`: show per-file outcomes and restore availability.

The future UI must keep saying:

- No files changed.
- Preview only until confirmation.
- Backup required before applying.
- Review-only and blocked items are excluded.

## 12. Testing Strategy

Before implementation, future work needs tests for:

- schema migration and `ensure_schema` repair.
- saving an ApplyPlan from a `StagingPlan` without file movement.
- preserving evidence snapshots.
- preserving blocked and review-only items.
- list/get/delete draft commands staying DB-only.
- path validation recording results without moving files.
- conflict handling records.
- result logs representing partial failure.
- restore entries mapping only files from the same Apply run.
- no mutating command called from preview UI.
- trust-boundary copy remaining free of unsafe claims.

Desktop proof is required only when visible workflow behavior changes.

## 13. Decision Record

Decisions:

- Audit sprint decision: no runtime migration, command, or UI was added then.
- Follow-up foundation decision: add DB-only draft/preview persistence commands
  and tables, with no file movement and no visible UI.
- Result-log, restore-entry, saved-plan UI, and real Apply work remain blocked.
- Future ApplyPlan persistence must reuse `StagingPlan`, Library file identity,
  scanner/indexed paths, duplicate/review/update context, Inbox intake context,
  and Apply Safety Contract rules.
- Future ApplyPlan is separate from `StagingPlan`.
- Future ApplyPlan persistence must snapshot evidence because indexed Library
  state can change after plan review.

Recommended next implementation step:

- saved-plan UI review, or
- read-only validation/conflict design.

Still blocked:

- real Apply.
- file movement.
- delete, cleanup, quarantine, replacement, and auto-sort.
- AI decisions.
- exposing existing mutating backend commands in normal UI.
