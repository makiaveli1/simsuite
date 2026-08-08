# Backup / Restore / Result Log Design v1

Date: 2026-05-16

This document defines the future backup, restore, and result-log contract for
SimSuite ApplyPlan work.

No files changed. Apply is not ready. This document does not add a SQLite
migration, runtime command, UI, file movement, file copy, folder creation,
cleanup, delete, quarantine, replacement, auto-sort execution, restore action,
backup execution, or AI decision.

Current implementation note: the follow-up
`codex/result-restore-schema-foundation-v1` sprint adds the DB-only schema,
models, helper logic, commands, and TypeScript API/mock support for
`apply_plan_runs`, `apply_plan_results`, and `apply_plan_restore_entries`.
Those records are metadata/log foundation rows only. They do not execute Apply,
backup, restore, file movement, file copying, folder creation, deletion,
cleanup, quarantine, replacement, auto-sort execution, or AI decisions.

Current implementation note: the follow-up
`codex/fixture-only-backup-prototype-v1` sprint adds a private backend
prototype for copy-backup-first behavior against temporary test files only. It
verifies size and SHA-256 hash, records safe DB result/restore metadata, and
does not expose a command, UI, real Apply, user-file backup execution, restore
execution, movement, deletion, cleanup, quarantine, replacement, or AI decision.

Current implementation note: the follow-up
`codex/fixture-only-restore-prototype-v1` sprint adds a private backend restore
prototype for temporary test files only. It copies from a recorded backup
reference to a temp fixture restore target, verifies size and SHA-256 hash,
records safe DB metadata, and does not expose a command, UI, real Apply, user
Restore, user-file backup/restore execution, movement, deletion, cleanup,
quarantine, replacement, or AI decision.

Current implementation note: the follow-up
`codex/fixture-backup-restore-integration-proof-v1` sprint proves the private
fixture backup and restore prototypes together as one temporary-file recovery
chain. It verifies backup and restore copies, checks one-run restore-map scope,
records only safe DB metadata, and still does not expose a command, UI, real
Apply, user Restore, user-file backup/restore execution, movement, deletion,
cleanup, quarantine, replacement, or AI decision.

Current implementation note: Organize `Saved plans` now includes a read-only
`Recovery history` section for existing DB-only ApplyPlan run logs, result
logs, and restore-map records. It only displays metadata, does not create
records, does not run backup or restore, does not expose fixture proof helpers,
and does not make Apply or Restore ready.

Current planning note: dry-run Apply design now lives in
`docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`. Dry-run can mention that backup,
restore-map, and result-log support are required before future Apply, but it
must not create backups, restore entries, result rows, confirmation tokens, or
file-changing work.

Current implementation note: `preview_apply_plan_dry_run` now exists as a
backend/API-only read-only command. It can list `Backup required`, `Restore map
required`, and `Result log required` as future safety steps, but it creates no
backup, result-log row, restore-entry row, confirmation token, folder, or file
change.

Current implementation note: Organize `Saved plans` now shows a read-only
`Dry-run preview` UI that can display those future recovery requirements for a
selected saved draft. It does not create recovery history records, result logs,
restore-map records, backups, folders, Apply, Restore, or confirmation.

## 1. Purpose

Before SimSuite ever changes files, it must know how to record what happened
and how to recover from failures.

Backup, restore, and result logs are the recovery contract for future
user-confirmed file-changing work. They must make every future operation
auditable, explainable, and limited to the files that the same confirmed run
actually changed.

This design is required before any visible Apply workflow can exist.

## 2. Current State

Current SimSuite state:

- saved draft ApplyPlan records exist.
- saved ApplyPlan item, signal, and blocker snapshots exist.
- Organize can save and review draft preview records.
- Organize can run read-only validation/conflict preview.
- validation preview keeps `canProceedToConfirmation=false`.
- real Apply does not exist.
- DB-only result-log foundation tables now exist.
- DB-only restore-entry foundation tables now exist.
- fixture-only backup prototype tests now exist for temporary files only.
- fixture-only restore prototype tests now exist for temporary files only.
- fixture-only backup + restore integration proof tests now exist for temporary
  files only.
- Organize can show read-only Recovery history metadata for saved plans.
- Organize can show read-only Dry-run preview classifications for saved plans.
- user-file backup execution does not exist.
- user-file restore execution does not exist.
- confirmation workflow does not exist.
- existing snapshot and restore code is prior art only.

No future Apply can proceed until this backup, restore, result-log, validation,
confirmation, and proof contract is implemented and tested.

## 3. Existing systems reused

| System | What it already provides | How backup/restore/result logs should use it | What not to duplicate |
| --- | --- | --- | --- |
| Saved ApplyPlan records | Draft plan metadata, caveats, counts, source scope, and safety flags | Parent record for future runs and result summaries | Do not create a separate file-action plan outside ApplyPlan. |
| ApplyPlan items | Saved file id, source path, destination preview string, bucket, evidence level, blocked/review-only state | One future result row should map back to one reviewed item | Do not build operation rows from frontend-only arrays. |
| ApplyPlan item signals | Evidence snapshots explaining why the suggestion exists | Preserve evidence context when a result is shown later | Do not reclassify files during result logging. |
| ApplyPlan item blockers | Saved blocked/review-only reasons | Force blocked items to skip future execution and log why | Do not silently drop blockers at confirmation time. |
| Validation preview results | Read-only stale/missing/conflict/review-only/backup blockers | Future confirmed runs must start from validation output or persisted validation proof | Do not treat validation preview as Apply authorization. |
| Library file identity/current paths | Current indexed `files.id`, path, source root, size/date/hash where available | Compare execution-time state against saved item snapshots | Do not create unindexed private file lists. |
| Scanner roots and settings | Configured Mods/Tray roots and scan-owned folder facts | Bound source/destination validation and path privacy | Do not infer roots from raw strings when settings exist. |
| Existing snapshot prior art | `snapshots` and `snapshot_items` schema, snapshot manager helpers | Inform backup and restore reference design | Do not assume snapshots are a complete ApplyPlan run model. |
| Existing restore prior art | `move_engine::restore_snapshot`, rollback helpers, file hash checks | Inform recovery checks and failure handling | Do not expose current restore helpers as ApplyPlan restore. |
| Move-engine prior art | Preflight checks, file moves, hash helpers, DB path update helpers | Reuse only after future ApplyPlan safety gates are implemented | Do not call move-engine apply paths from preview, validation, or UI. |
| Apply Safety Contract | Required preview, confirmation, backup/restore, conflicts, recoverable errors, result logs, and proof | Gate every future file-changing phase | Do not skip safety steps for convenience. |
| Existing Systems Integration Contract | Evidence reuse and anti-duplication rules | Prevent parallel parsers, duplicate truth, folder truth, or UI-owned file actions | Do not add new metadata paths without justification. |

## 4. Existing Restore/Snapshot Prior Art Inventory

| Name | Location | What it does | Does it touch files? | Can it be reused? | Safety gaps | Future role |
| --- | --- | --- | --- | --- | --- | --- |
| `snapshot_manager::create_snapshot` | `src-tauri/src/core/snapshot_manager/mod.rs` | Inserts `snapshots` and `snapshot_items` rows for supplied records | No | As schema and transaction prior art | It does not create backup copies or link to ApplyPlan runs/items | Inform future result/restore schema. |
| `snapshot_manager::list_snapshots` | `src-tauri/src/core/snapshot_manager/mod.rs` | Lists snapshot summaries | No | As read-only listing prior art | Snapshot summaries are not ApplyPlan result logs | Inform future read-only result lists. |
| `snapshots` | `database/migrations/0001_initial.sql` and `ensure_schema` | Stores snapshot name, description, created time | No by itself | As prior schema | Not scoped to ApplyPlan run, confirmation, or per-file result | Reference only. |
| `snapshot_items` | `database/migrations/0001_initial.sql` and `ensure_schema` | Stores original path, original hash, optional backup path | No by itself | As prior schema | Missing apply run id, item id, operation status, restore status, and result text | Reference only. |
| `move_engine::restore_snapshot` | `src-tauri/src/core/move_engine/mod.rs` | Approval-gated restore entrypoint for snapshot rollback | Yes | Not directly for v1 UI | Uses older snapshot model, changes files, rebuilds derived metadata | Prior art for recovery checks only. |
| `rollback_snapshot_internal` | `src-tauri/src/core/move_engine/mod.rs` | Reads snapshot items, verifies hashes, moves files back, updates DB rows | Yes | Not directly | Not scoped to ApplyPlan run/result rows; can error on changed contents | Prior art for future restore validation. |
| `rollback_applied_moves` | `src-tauri/src/core/move_engine/mod.rs` | Reverses a list of already-applied moves | Yes | Fixture-only later | Internal helper, no user-facing result log | Prior art for reverse-order recovery. |
| `rollback_guided_changes` | `src-tauri/src/core/move_engine/mod.rs` | Rolls back guided install moves and backup moves | Yes | Fixture-only later | Coupled to guided install, not ApplyPlan | Prior art for partial failure behavior. |
| `preflight_moves` and guided preflight helpers | `src-tauri/src/core/move_engine/mod.rs` | Checks missing source and destination-exists cases | Read-only checks | Yes, after extraction or wrapping | Current helpers are private to move-engine flows | Prior art for future validation and executor preflight. |
| `move_engine::move_single_file` | `src-tauri/src/core/move_engine/mod.rs` | Renames/moves one file after parent directory check | Yes | Only behind future confirmed executor | It is file-changing and must not be called by preview/validation/UI | Future executor primitive only. |
| `file_hash` | `src-tauri/src/core/move_engine/mod.rs` | Computes SHA-256 hash of a file | Reads file | Maybe | Must be bounded and not run from UI; hash timing and errors need tests | Future backup/restore verification helper. |
| `update_file_record_after_move` | `src-tauri/src/core/move_engine/mod.rs` | Updates Library row after a move and syncs category override paths | DB write | Maybe | Tied to move-engine path, no ApplyPlan result log | Future executor DB-update prior art. |
| `update_file_record_on_restore` | `src-tauri/src/core/move_engine/mod.rs` | Updates Library row after restore | DB write | Maybe | Not scoped to ApplyPlan restore entry | Future restore DB-update prior art. |
| `commands::restore_snapshot` | `src-tauri/src/commands/mod.rs` | Exposes snapshot restore with approval flag and workspace events | Yes | No for current UI | Existing command changes files and is not an ApplyPlan restore contract | Must remain unexposed. |
| `undo_applied_item` command block | `src-tauri/src/commands/mod.rs` | Moves applied download files back to origin paths | Yes | No | Download-specific, not ApplyPlan-scoped, file-changing | Prior art only. |
| `downloads_watcher::reject_download_item(s)` | `src-tauri/src/core/downloads_watcher/mod.rs` | Moves staged files into rejected app-local storage and updates status | Yes | No | Reject/quarantine-like intake path, not ApplyPlan restore | Must remain out of ApplyPlan UI. |
| `downloads_watcher::restore_rejected_item` | `src-tauri/src/core/downloads_watcher/mod.rs` | Moves rejected files back into staging and updates status | Yes | No | Intake-specific restore, not a future Apply restore map | Prior art only. |
| Guided/special apply paths | `move_engine` and `commands` | Install/replace guided download files, create snapshots, record events | Yes | No direct reuse | Coupled to Inbox/special-mod flows and existing apply-ready states | Prior art for fixture-only recovery tests. |

## 5. Backup Strategy Options

### Option A - Copy backup before move

Copy each original file to app-local backup storage before future movement.

Pros:

- strongest recovery story for user-owned files.
- can verify backup exists before moving.
- supports restore even if destination or moved file is later damaged.

Cons:

- uses more disk space.
- needs hash/size verification and retention rules.
- can fail on large files, permissions, or full disk.

### Option B - Journal-only restore map

Record source/destination paths and reverse moves later.

Pros:

- lower disk use.
- simpler schema.

Cons:

- weak recovery if files are overwritten, modified, deleted, or externally
  moved after Apply.
- cannot recover from some partial failures.
- poor fit for trust-sensitive user-owned folders.

### Option C - Hybrid

Use backups for higher-risk operations and a journal for lower-risk moves.

Pros:

- can reduce disk usage.
- may fit mature future workflows.

Cons:

- harder to explain.
- more edge cases.
- risky before the narrow v1 behavior is proven.

### Recommended future v1 strategy

Use copy-backup-first plus an explicit restore map and a per-file result log.

Journal-only restore is too weak for SimSuite's first visible file-changing
workflow. Hybrid backup can be revisited only after a fixture-backed prototype
proves the simple model.

Future Apply must not be exposed until backup creation, backup verification,
restore map recording, partial failure logging, and restore limitations are
tested.

## 6. Restore Map Design

Future restore map entries must record:

- apply run id.
- apply plan id.
- apply plan item id.
- result id.
- original source path at execution.
- destination path at execution.
- backup path when a backup copy exists.
- file hash before operation when available.
- file size before operation.
- operation kind.
- operation result status.
- restore status.
- restore error code and message.
- created, updated, restored, and failed timestamps.

Restore maps must be scoped to one Apply run. They must never be used to touch
unrelated user files.

## 7. Result Log Design

Future result logs must record:

- run id.
- apply plan id.
- apply plan item id.
- operation attempted.
- source path at execution.
- destination path at execution.
- backup path when relevant.
- status: `pending`, `skipped`, `applied`, `failed`, `restored`, or
  `restore_failed`.
- error code.
- error message.
- user-readable summary.
- started, finished, and updated timestamps.

The result log must clearly distinguish:

- items that were blocked before execution.
- items skipped by validation or conflict checks.
- items that changed files.
- items that failed before changing files.
- items that failed after a backup or partial move.
- items that were restored later.

### Pre-mutation attempt journal requirement

The existing result/restore foundation is not a sufficient ownership journal for
a future no-replace filesystem claim. `apply_plan_runs` identifies one run but
not which item has actually begun mutation. `apply_plan_results` is designed to
record an operation attempted/result and its execution paths. Restore entries
record recovery evidence. A saved ApplyPlan item plus a verified backup proves
what was planned and that recovery material exists, but it does not prove that
this run actually started or created an observed destination claim.

Do not overload `pending_log` as a hidden pre-mutation ownership marker merely
because the current schema accepts the value before a real executor exists. That
would mix two different meanings in one row: "this operation is about to start"
and "this operation/result is being logged." It would also leave recovery code
without an explicit attempt lifecycle.

Before any exclusive/no-replace primitive is adopted, SimSuite needs durable
per-item attempt evidence written before the first user-file mutation. The
smallest coherent future contract should bind one attempt identity to:

- Apply run id, plan id, and item id.
- exact source and destination paths after validation.
- expected source hash and size.
- the verified backup result and restore-entry identities.
- the selected move/claim strategy.
- the filesystem capability evidence that allowed that strategy.
- a state that explicitly means `prepared_before_change`.
- later states for claim observed, destination verified, source release
  completed, committed, blocked, failed, or recovery required.
- created/updated timestamps and a unique attempt identity suitable for
  reconciliation.

A dedicated future per-item attempt/journal record is the preferred direction.
An app-owned sidecar journal could be explored only if it provides the same
scope, durability, and crash-recovery guarantees; it must not place unexplained
control files in the player's Mods/Tray folders. No migration or sidecar is
introduced by this design note.

The attempt record still would not make arbitrary filesystem evidence safe by
itself. Recovery must compare the recorded attempt with the observed filesystem
state and fail closed when another process could have created or changed that
state.

## 8. Future Schema Design

These tables started as design-only and now have a DB-only foundation in
`database/migrations/0004_applyplan_result_restore_foundation.sql`. The
foundation stores metadata only and still does not prove files can move safely.

### `apply_plan_runs`

Purpose: one future confirmed execution attempt for one saved ApplyPlan.

Important columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_id INTEGER NOT NULL REFERENCES apply_plans(id)`
- `status TEXT NOT NULL`
- `backup_strategy TEXT NOT NULL`
- `confirmation_token TEXT`
- `confirmed_at TEXT`
- `started_at TEXT`
- `finished_at TEXT`
- `total_items INTEGER NOT NULL DEFAULT 0`
- `skipped_items INTEGER NOT NULL DEFAULT 0`
- `applied_items INTEGER NOT NULL DEFAULT 0`
- `failed_items INTEGER NOT NULL DEFAULT 0`
- `restored_items INTEGER NOT NULL DEFAULT 0`
- `summary TEXT`

Indexes:

- `idx_apply_plan_runs_plan_id`
- `idx_apply_plan_runs_status_started_at`

Retention/privacy notes:

- Keep local only.
- Do not export full paths unless the user explicitly requests details.

Why existing tables are not enough:

- `apply_plans` stores the draft/reviewed plan, not an execution attempt.
- `snapshots` do not record confirmation, summary counts, or ApplyPlan item
  linkage.

### Future `apply_plan_attempts` candidate — not implemented

Purpose: one durable per-item execution-intent/journal row written after all
preflight and backup gates pass but before the first user-file mutation.

This is a future design requirement only. No table or migration exists yet.
A later schema design should prefer a dedicated row over overloading result or
restore records.

Minimum candidate fields:

- unique attempt id.
- `apply_plan_run_id`.
- `apply_plan_id`.
- `apply_plan_item_id`.
- exact validated source path.
- exact validated destination path.
- expected pre-change source hash and size.
- verified backup result id.
- verified backup restore-entry id.
- selected operation/claim strategy.
- serialized or normalized filesystem capability proof sufficient for that
  strategy.
- lifecycle state beginning with `prepared_before_change`.
- created and updated timestamps.

Future lifecycle states must be designed as a crash-recovery state machine, not
as user-facing result labels. At minimum, the design must distinguish prepared,
destination claim observed, destination verified, source release completed,
committed, blocked/failed, and recovery-required states. Allowed transitions,
idempotence rules, uniqueness constraints, retention, and recovery authority
must be specified before migration work begins.

The attempt identity is evidence that SimSuite prepared and owns one particular
item execution attempt. Its lifecycle state, not the row's mere existence, must
show whether filesystem mutation actually began. The record is not by itself
permission to delete or overwrite an observed path. Recovery must still prove the
recorded attempt and current filesystem state agree exactly.

### `apply_plan_results`

Purpose: one future per-file operation/result row.

Important columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_run_id INTEGER NOT NULL REFERENCES apply_plan_runs(id)`
- `apply_plan_id INTEGER NOT NULL REFERENCES apply_plans(id)`
- `apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items(id)`
- `operation_kind TEXT NOT NULL`
- `source_path_at_execution TEXT NOT NULL`
- `destination_path_at_execution TEXT`
- `backup_path TEXT`
- `status TEXT NOT NULL`
- `error_code TEXT`
- `error_message TEXT`
- `summary TEXT NOT NULL`
- `started_at TEXT`
- `finished_at TEXT`
- `updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP`

Indexes:

- `idx_apply_plan_results_run_id`
- `idx_apply_plan_results_item_id`
- `idx_apply_plan_results_status`

Retention/privacy notes:

- Full paths are necessary for local audit and restore.
- UI should prefer shortened paths, with full paths behind technical details.

Why existing tables are not enough:

- `apply_plan_items` records planned/saved snapshots, not execution outcomes.
- download events are not scoped to ApplyPlan items.

### `apply_plan_restore_entries`

Purpose: one future restore map row per changed file or backup reference.

Important columns:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `apply_plan_run_id INTEGER NOT NULL REFERENCES apply_plan_runs(id)`
- `apply_plan_result_id INTEGER NOT NULL REFERENCES apply_plan_results(id)`
- `apply_plan_item_id INTEGER NOT NULL REFERENCES apply_plan_items(id)`
- `original_source_path TEXT NOT NULL`
- `destination_path_at_execution TEXT`
- `backup_path TEXT`
- `file_hash_before TEXT`
- `file_size_before INTEGER`
- `operation_kind TEXT NOT NULL`
- `result_status TEXT NOT NULL`
- `restore_status TEXT NOT NULL`
- `restore_error_code TEXT`
- `restore_error_message TEXT`
- `created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP`
- `restored_at TEXT`
- `updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP`

Indexes:

- `idx_apply_plan_restore_entries_run_id`
- `idx_apply_plan_restore_entries_result_id`
- `idx_apply_plan_restore_entries_item_id`
- `idx_apply_plan_restore_entries_status`

Retention/privacy notes:

- Keep local and user-owned.
- Do not silently delete restore entries while Apply result logs are visible.

Why existing tables are not enough:

- `snapshot_items` can store original and backup paths, but it does not map
  restore data to one ApplyPlan run, one result row, and one reviewed item.

## 9. Failure Handling Design

Default future v1 rule: block or skip the item and log the reason. Do not
guess.

Required future behavior:

- source missing: skip item, log `source_missing`, keep run recoverable.
- destination exists: skip item, log conflict, do not overwrite.
- destination parent missing: block unless folder creation was explicitly
  previewed and confirmed by a future design.
- path too long: block item and log the exact path-length reason.
- permission denied: skip or fail item, log the OS error safely.
- file in use: skip or fail item, log that the file was unavailable.
- disk full: stop or fail safely, preserve completed result rows, keep restore
  entries for any changed files.
- backup copy failed: block the item before moving.
- partial success: log every item status and preserve restore entries for
  changed files.
- interrupted process: keep run status recoverable or failed with exact rows
  already written.
- restore failed: log restore failure without claiming recovery succeeded.

## 10. Restore / Undo Limitations

Future restore must explain its limits:

- Restore can only restore files changed by the same Apply run.
- Restore must not touch unrelated user files.
- Restore must not delete user-created files without explicit future
  confirmation.
- Restore cannot promise perfect recovery if files were changed externally
  after Apply.
- Restore cannot rely on filename guesses.
- Restore must verify path boundaries before changing files.
- Restore must explain what it restored, skipped, and could not restore.

User-facing copy must not say:

- guaranteed recovery.
- safe to move.
- safe to delete.
- fixed.
- auto-fixed.
- AI verified.

## 11. Future UI Requirements

Future UI must show:

- `No files changed yet` before confirmation.
- backup required and restore required states.
- exact operation preview.
- blocked and skipped items.
- per-file result logs.
- partial failure summary.
- restore availability.
- restore limitations.
- technical details behind collapsed disclosure where full paths are needed.

Future UI must not show enabled controls labeled:

- Apply.
- Move files.
- Delete.
- Clean up.
- Quarantine.
- Restore now.
- Undo changes.
- Ready to apply.

Those controls stay blocked until the full Apply Safety Contract is implemented
and proven.

## 12. Testing Strategy

Before implementation, future sprints must add tests proving:

- backup success is verified before any later move step.
- backup failure blocks Apply.
- result logs record skipped items.
- result logs record failed items.
- partial failure records exact per-file outcomes.
- restore map only contains items changed by one run.
- restore does not touch unrelated files.
- destination conflict prevents overwrite.
- interrupted runs leave auditable rows.
- restore failure is logged and does not claim success.
- no preview UI calls mutating commands.
- desktop proof shows result log and restore limitations.

## 13. Recommended Next Implementation Phases

Phase A - design only:

- Current sprint.
- Create this document, report, and doc guard.
- No migration or runtime behavior.

Phase B - result/restore schema foundation:

- Add `apply_plan_runs`, `apply_plan_results`, and
  `apply_plan_restore_entries`.
- Add DB-only models and tests.
- No file movement.

Phase C - backend fixture backup prototype:

- Prototype copy-backup-first and restore map behavior against fixture-only
  temporary files.
- Keep it hidden from UI.
- No real user Library files.
- Current implementation note: this phase now has a private backend helper and
  tests for backup and restore mechanics. It is not a user-facing backup or
  restore feature.

Phase D - apply dry-run with result-log preview:

- Produce a run/result preview without changing files.
- Prove paths, conflicts, and backup requirements.

Phase E - hidden fixture-only confirmed apply prototype:

- Run a narrow confirmed operation only against test fixtures.
- Prove partial failure and restore behavior.

Phase F - visible Apply v1:

- Only after preview, confirmation, backup, restore, result logs, tests, and
  desktop proof are complete.

## 14. Decision Record

- Future v1 should use copy-backup-first plus explicit restore maps and
  per-file result logs.
- Journal-only restore is not enough for first visible Apply.
- Hybrid backup remains future optimization after proof.
- Existing snapshot/restore code is prior art, not the final ApplyPlan
  recovery contract.
- No result/restore tables are implemented by this sprint.
- No Apply, restore, backup, file movement, folder creation, cleanup, delete,
  quarantine, replacement, auto-sort execution, or AI decision is implemented
  by this sprint.
