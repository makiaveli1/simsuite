# Validation & Conflict Preview v1

Date: 2026-05-15

This document defines the future validation and conflict-preview layer for
saved draft organization plans.

It does not implement Apply. No files changed. This document does not add a
SQLite migration, UI, file movement, folder creation, cleanup, delete,
quarantine, replacement, auto-sort execution, or AI decision.

Current implementation note: `preview_apply_plan_validation` now exists as a
read-only backend/API command. It returns response-only validation/conflict
preview data for saved draft ApplyPlans, keeps
`canProceedToConfirmation=false`, does not persist validation status, does not
create folders or backups, and does not change user files.

Current implementation note: Organize `Saved plans` now shows a visible
`Validation preview` section for selected saved drafts. The UI calls the
read-only `previewApplyPlanValidation` API, displays summary counts, caveats,
friendly item-level validation/conflict labels, and `Future confirmation
blocked`. It does not persist validation state, expose Apply, create folders,
or change files.

Current planning note: backup/restore/result-log design now lives in
`docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`. Validation preview can
continue to report `backup_required`, but future confirmation stays blocked
until copy-backup-first recovery, restore maps, and per-file result logs are
implemented and proven.

Current implementation note: DB-only run/result/restore metadata tables now
exist for future ApplyPlan work. Validation preview remains response-only and
still returns `canProceedToConfirmation=false`; the new result/restore records
do not execute backup, restore, Apply, or any file-changing workflow.

Current implementation note: a fixture-only backup prototype now verifies
copy-backup-first behavior with temporary test files and records safe
result/restore metadata. It remains outside validation UI, does not persist
Apply readiness, and does not change user files.

Current implementation note: a fixture-only restore prototype now verifies
restore-copy behavior from recorded backup references to temporary fixture
targets and records safe result/restore metadata. It remains outside validation
UI, does not persist Apply or Restore readiness, and does not change user files.

Current implementation note: a fixture-only backup + restore integration proof
now verifies the private backup and restore prototypes together with temporary
test files only. It remains outside validation UI, does not persist Apply or
Restore readiness, and does not change user files.

Current implementation note: Organize `Saved plans` now also shows read-only
`Recovery history` metadata for existing DB-only run logs, result logs, and
restore-map records. This does not change validation semantics:
`canProceedToConfirmation=false` remains the safe boundary, Apply is not ready
yet, and Restore is not ready yet.

Current planning note: dry-run Apply design now lives in
`docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`. Dry-run must depend on validation
preview output before classifying future item behavior, and it must keep
`canProceedToApply=false` and `canProceedToConfirmation=false` until later
confirmation, backup/restore execution, result logging, and proof exist.

Current implementation note: `preview_apply_plan_dry_run` now exists and calls
the read-only validation preview before classifying saved draft items. Dry-run
does not replace validation, does not persist validation or dry-run state, does
not create result/restore metadata, and still keeps `canProceedToApply=false`
and `canProceedToConfirmation=false`.

## 1. Purpose

Validation/conflict preview is the safety gate between a saved draft preview
plan and any future confirmation workflow.

Saved plans currently let users review what SimSuite suggested and why. They do
not prove that the source files still exist, that destinations are still valid,
that roots are configured, that there are no destination conflicts, or that
backup/restore requirements are satisfied.

Future validation must answer these questions before any later Apply work can
be considered:

- is the saved draft still based on current Library facts?
- is each source still present and under an allowed root?
- is each destination under a configured Mods or Tray root?
- would any destination conflict with an existing file or unsupported folder?
- are blocked, review-only, duplicate-review, or weak-evidence items excluded?
- is backup/restore design available before confirmation?

Validation is not Apply. It is a preview of readiness problems and blockers.

## 2. Current State

Current saved draft plans can be created and reviewed from Organize.

- `generate_sorting_preview_plan` creates preview-only `StagingPlan` data.
- `build_apply_plan_from_staging_plan` saves a backend-owned draft from that
  preview data.
- `apply_plans`, `apply_plan_items`, `apply_plan_item_signals`, and
  `apply_plan_item_blockers` store draft preview records.
- `apply_plan_items` already has nullable `validation_status` and
  `conflict_status` columns for future use.
- Organize can list, inspect, and cancel saved draft records.

Missing today:

- `preview_apply_plan_validation` now provides a read-only validation/conflict
  response for saved draft records.
- no backup/restore result model for ApplyPlan.
- no result log.
- no real Apply.
- Organize `Saved plans` now has a visible validation preview review section
  that calls `preview_apply_plan_validation`.

No future Apply can proceed until validation/conflict preview exists, is proven,
and is followed by confirmation, backup/restore, recoverable errors, result
logs, and desktop proof.

## 3. Existing Systems Reused

Future validation must reuse existing SimSuite evidence before adding new
logic.

| System | What it provides today | How validation should use it | Do not duplicate |
| --- | --- | --- | --- |
| Saved ApplyPlan drafts | Saved plan, item, signal, blocker, caveat, source-scope snapshots | Source of reviewed suggestions and saved blockers | Do not build from frontend-only item arrays. |
| `StagingPlan` / sorting preview | Preview-only source item data with evidence levels, buckets, caveats, source signals, and `wouldTouchFiles=false` | Keep saved drafts tied to their preview origin | Do not create a second organization preview model. |
| Library file identity | `files.id`, current indexed path, filename, source root, size/date, hash when indexed | Compare saved snapshots against current indexed state | Do not create new unreviewed file lists. |
| Scanner/indexed roots | Configured Mods/Tray paths, source roots, scan-time folder metadata | Validate allowed roots and stale source context | Do not infer roots from raw strings when settings and indexed roots exist. |
| Library folder metadata | Indexed real folder tree and direct folder files | Check destination-root and folder context for preview purposes | Do not rebuild folder truth in UI. |
| Duplicate detector | Exact duplicate proof and review-only comparison rows | Block duplicate-review and cleanup-like items from future Apply | Do not reimplement duplicate truth. |
| Review queue | Manual review membership and reasons | Preserve review-only blockers and user-facing reasons | Do not create private review queues. |
| Updates/watch | Source/watch state and no-source caveats | Add caveats only; never treat update state as replacement proof | Do not invent update-source truth. |
| Inbox | Intake/downloaded/imported batch state | Preserve origin context when drafts come from intake work | Do not treat raw intake batches as saved plans. |
| Snapshot/restore prior art | Existing snapshot and restore code paths | Inform backup/restore requirements only | Do not treat existing restore helpers as a complete Apply contract. |
| Apply Safety Contract | Required preview, confirmation, backup/restore, path validation, conflicts, result logs, and proof | Gate all future validation and confirmation decisions | Do not expose direct file actions outside the contract. |
| Existing Systems Integration Contract | Anti-duplication and evidence-reuse rules | Prevent parallel parsers, duplicate truth, folder truth, and UI-owned file actions | Do not add new metadata paths without explaining why. |

## 4. What Validation Can Check Later

Future validation may check:

- source file still exists in the Library index.
- source path still matches the saved snapshot, or is marked stale when it
  changed.
- source path is under an allowed configured root or known app-local intake
  root.
- destination root is configured.
- destination path is under the allowed Mods or Tray root.
- destination path does not use path traversal.
- destination does not already exist.
- destination parent folder exists, or future folder creation would need to be
  explicitly previewed.
- item is not already blocked.
- item is not review-only.
- item is not weak heuristic-only.
- item is not duplicate cleanup or replacement work.
- item has enough evidence for a future confirmation review.
- backup/restore design exists before any future confirmation.
- conflict status is known and clear.

All checks must be explainable from existing indexed evidence, saved item
snapshots, settings, or explicitly documented validation logic.

## 5. What Validation Must Not Do

Validation must not:

- move files.
- copy files.
- create folders.
- delete files.
- rename files.
- clean up folders.
- quarantine files.
- replace files.
- auto-sort files.
- mark a file as safe.
- claim a mod is broken.
- claim dependency or missing mesh truth.
- call AI to decide readiness.
- silently turn saved drafts into Apply-ready records.
- call legacy staging, downloads, move-engine, restore, cleanup, reject, or
  file-changing commands.

Validation output must use wording such as `No files changed`,
`Validation preview`, `Conflict preview`, `Not ready to apply`, `Blocked`, and
`Review-only`.

## 6. Validation Status Model

Final validation status names for the future preview layer:

| Status | Meaning |
| --- | --- |
| `not_validated` | No validation preview has been run for this plan or item. |
| `valid_preview_only` | The item has no current validation blocker, but it is still preview-only and cannot proceed to confirmation until the rest of the Apply Safety Contract exists. |
| `blocked` | Existing plan/item blockers prevent future confirmation. |
| `stale_source` | The saved source snapshot no longer matches current indexed Library facts. |
| `missing_source` | The source file is missing from the current index or cannot be resolved by the chosen future strategy. |
| `missing_destination_root` | The Mods or Tray destination root is not configured or unavailable. |
| `unsafe_destination` | The destination path is outside the allowed root, uses traversal, or cannot be proven safe under path rules. |
| `destination_exists` | The destination path already exists and future Apply must not overwrite it. |
| `unsupported_cross_root` | The source/destination root combination is not supported by the future Apply scope. |
| `review_only_blocked` | The item is review-only, weak-evidence, or manually blocked from confirmation. |
| `duplicate_review_blocked` | The item is related to duplicate review or cleanup-style work that cannot be applied by organization Apply. |
| `backup_required` | Backup/restore requirements are not satisfied, so future confirmation must stay blocked. |
| `error` | Validation could not complete and must surface the error as a blocker. |

Plan-level validation should roll item statuses into a summary. If any item is
blocked, stale, missing, conflicted, review-only, or backup-blocked, the plan
must stay not ready for confirmation.

## 7. Conflict Status Model

Final conflict status names for the future preview layer:

| Status | Meaning |
| --- | --- |
| `not_checked` | No destination conflict check has run. |
| `none` | No conflict was found by the preview check. This still does not authorize Apply. |
| `destination_exists` | A file or folder already exists at the destination. |
| `same_name_conflict` | Another item would produce the same destination name. |
| `case_conflict` | Destination differs only by case on a case-insensitive filesystem. |
| `folder_missing` | Destination parent folder is missing and folder creation is not yet previewed. |
| `permission_unknown` | SimSuite cannot prove the destination is writable without a future confirmed action. |
| `source_missing` | The source is missing during conflict checks. |
| `path_too_long` | The destination may exceed supported platform path limits. |
| `cross_root_blocked` | The destination would cross an unsupported root boundary. |
| `unsupported` | Conflict checking cannot support this item type or action. |

Conflict preview must never create folders or touch destination files.

## 8. Proposed Validation Result Model

These model shapes are design-only. They should not be implemented until the
read-only validation preview command sprint.

```ts
type ApplyPlanValidationPreview = {
  planId: number;
  status: "not_validated" | "valid_preview_only" | "blocked";
  canProceedToConfirmation: false;
  checkedAt: string;
  summary: {
    totalItems: number;
    blockedItems: number;
    reviewOnlyItems: number;
    conflictItems: number;
    staleItems: number;
    missingSourceItems: number;
    destinationConflictItems: number;
    backupBlockedItems: number;
  };
  caveats: string[];
  items: ApplyPlanValidationItem[];
};

type ApplyPlanValidationItem = {
  itemId: number;
  fileId: number | null;
  fileName: string;
  validationStatus: string;
  conflictStatus: string;
  blocked: boolean;
  reviewOnly: boolean;
  canApplyLater: false;
  reasons: string[];
  requiredNextSteps: string[];
};
```

`canProceedToConfirmation` must remain `false` until a later sprint implements
and proves validation, confirmation, backup/restore, result logs, and desktop
proof. The first read-only command should not mark plans as ready.

## 9. Future Command Design

Implemented first command:

### `preview_apply_plan_validation`

- Read-only response first; implemented in
  `src-tauri/src/core/apply_plan_validation.rs`.
- Accepts a saved draft plan id.
- Loads saved ApplyPlan data.
- Compares saved item snapshots with current Library/settings evidence.
- Returns validation/conflict status, counts, reasons, caveats, and required
  next steps.
- Keeps `canProceedToConfirmation=false`.
- Does not write files.
- Does not create folders.
- Does not create backups.
- Does not persist result logs.
- Does not call Apply, move-engine apply paths, cleanup, reject, restore, or
  staging/downloads mutation commands.

### `refresh_apply_plan_validation`

Future optional command after the read-only preview UX is proven.

- May persist `validation_status` and `conflict_status` to existing
  `apply_plan_items` columns.
- Must remain DB-only plus read-only filesystem/index checks.
- Must not create Apply-ready records.
- Must not set executor states.

Updated implementation sequence:

1. Keep `preview_apply_plan_validation` response-only and covered by backend
   and TypeScript tests.
2. Keep Organize validation preview UI as review-only with no Apply button.
3. Consider persisted validation statuses only after UX and semantics are
   stable.

## 10. UI Recommendation

Implemented first UI:

Organize `Saved plans` now shows validation as a review layer, not as an Apply
workflow. The selected saved draft detail view includes `Validation preview`,
`Check saved plan`, summary counts, caveats, item-level validation/conflict
labels, reasons, required next steps, `No files changed`, and `Future
confirmation blocked`.

Recommended UI copy:

- `Needs validation`
- `Validation preview`
- `Conflict preview`
- `No files changed`
- `Not ready to apply`
- `Backup required before applying`
- `Review-only`
- `Blocked`

The UI should show:

- plan-level validation summary.
- counts for blocked, review-only, stale, missing-source, and conflict items.
- item-level validation and conflict reasons.
- source snapshot compared with current indexed state.
- full paths collapsed behind `Technical details`.
- no Apply button.

If a disabled future note is needed, it should say:

`Apply is not ready yet. Future Apply requires validation, confirmation,
backup/restore, conflict handling, result logs, tests, and proof.`

The UI should keep the current SimSuite direction: flat, compact,
desktop-oriented, squared, accessible, and preview-first.

## 11. Testing Strategy

Future implementation tests must prove:

- validation never calls mutating commands.
- blocked items remain blocked.
- review-only items remain blocked.
- weak heuristic-only items cannot proceed.
- duplicate-review or cleanup-style items cannot proceed.
- missing source is detected from indexed state or the documented future
  filesystem strategy.
- destination conflict is detected without creating files.
- path traversal is blocked.
- destination outside Mods/Tray root is blocked.
- missing destination root blocks validation.
- unsupported cross-root moves are blocked.
- backup requirements keep confirmation blocked.
- output keeps `canProceedToConfirmation=false` until later safety work exists.
- UI shows no Apply, move, cleanup, delete, quarantine, replacement, or
  auto-sort controls.
- desktop proof verifies validation preview and `No files changed` copy.

Current design-sprint guard test should only assert that this document exists
and keeps the no-file-change boundary explicit.

## 12. Recommended Next Implementation Phase

Next sprint:

Validation preview UX polish or backup/restore/result-log design.

Potential UX polish scope:

- tighten validation result grouping.
- add filtering by blocked, review-only, conflict, stale, and missing-source
  states.
- keep full paths collapsed by default.
- keep `canProceedToConfirmation=false`.
- no Apply, file movement, folder creation, backup creation, result logs, or
  restore entries.

## 13. Decision Record

Decisions:

- The read-only validation command exists.
- Organize now displays validation/conflict preview results for saved drafts.
- The UI remains review-only and does not expose Apply.
- No migration in this design sprint.
- No TypeScript/Rust runtime enum placeholders in this design sprint.
- Use existing nullable `validation_status` and `conflict_status` columns later
  if persisted validation is needed.
- First future implementation should return a read-only preview response before
  persisting validation results.
- Validation/conflict preview must keep all future outputs blocked from
  confirmation until the full Apply Safety Contract is implemented and proven.

What remains blocked:

- real Apply.
- file movement.
- file copying.
- folder creation.
- cleanup.
- delete.
- quarantine.
- replacement.
- auto-sort execution.
- AI decisions.
- result logs.
- restore entries.
