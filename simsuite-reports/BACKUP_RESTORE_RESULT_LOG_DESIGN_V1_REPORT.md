# Backup / Restore / Result Log Design v1 Report

Date: 2026-05-16

## What was audited

- Apply Safety Contract requirements for backup, restore, recoverable errors,
  and per-file result logs.
- ApplyPlan persistence audit and current draft-plan schema.
- Validation/conflict preview design and read-only validation UI state.
- Existing snapshot manager and `snapshots` / `snapshot_items` tables.
- Move-engine rollback, restore, preflight, hash, and DB update helpers.
- Downloads/Inbox reject, restore, guided apply, and special apply prior art.
- Existing docs, trust boundaries, backend map, and navigation ownership.
- Linear issues `VEL-18`, `VEL-19`, `VEL-20`, and `VEL-31`.

## Backup / restore / result-log gap

Saved draft plans and validation preview exist, but there is still no future
Apply recovery contract in runtime.

Missing before future Apply:

- result-log tables.
- restore-entry tables.
- Apply run table.
- backup execution.
- restore execution.
- confirmation workflow.
- per-file execution result logging.
- fixture proof for backup and restore behavior.

## Existing systems reused

- Saved ApplyPlan records.
- ApplyPlan item snapshots.
- ApplyPlan item blockers and source signals.
- Read-only validation preview output.
- Library file identity and current indexed paths.
- Scanner roots/settings.
- Existing snapshot manager and snapshot tables as prior art.
- Move-engine preflight, rollback, restore, hash, and DB update helpers as prior
  art only.
- Apply Safety Contract.
- Existing Systems Integration Contract.

## New data or logic added

Docs/test only:

- new backup/restore/result-log source-of-truth design.
- new sprint report.
- lightweight trust-boundary doc guard.
- current-state notes in existing repo memory docs.

No migration, command, API wrapper, UI, backup execution, restore execution,
result-log persistence, file movement, folder creation, cleanup, delete,
quarantine, replacement, auto-sort execution, or AI decision was added.

## Backup strategy decision

Recommended future v1: copy-backup-first plus an explicit restore map and
per-file result log.

Reason: journal-only restore is too weak for user-owned Sims folders. A copied
backup can be verified before a future move, and the restore map/result log can
explain exactly what changed, what failed, and what can be restored.

Hybrid backup can be revisited later after fixture proof.

## Restore map design

Future restore entries must record:

- apply run id.
- apply plan id.
- apply plan item id.
- result id.
- original source path.
- destination path at execution.
- backup path when present.
- file hash and size before operation when available.
- operation kind.
- result status.
- restore status.
- error code/message.
- timestamps.

Restore must be scoped to one Apply run and must not touch unrelated user
files.

## Result log design

Future per-file result logs must record:

- run id.
- apply plan item id.
- operation attempted.
- source and destination path at execution.
- backup path when relevant.
- status: `pending`, `skipped`, `applied`, `failed`, `restored`, or
  `restore_failed`.
- error code/message.
- user-readable summary.
- timestamps.

The result log must distinguish blocked/skipped items from files that actually
changed.

## Future schema recommendation

Design-only future tables:

- `apply_plan_runs`: one future confirmed execution attempt and summary.
- `apply_plan_results`: one per-file operation/result row.
- `apply_plan_restore_entries`: one restore map row per changed file or backup
  reference.

These tables were not implemented in this sprint.

## What changed

- Added `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`.
- Added this report.
- Added a trust-boundary guard for the new design doc.
- Added current-state references in planning, trust, backend-map, navigation,
  status, and handoff docs.

## What this means for the user

SimSuite is defining recovery and logs before it ever adds file-changing Apply.
Users still cannot Apply changes, but future work now has a clearer contract
for explaining what happened and how recovery should work.

## Trust / safety boundary

No Apply was added. No files were moved, copied, created, deleted, cleaned up,
quarantined, replaced, or auto-sorted. No folders were created. No backups or
restore entries were created. No AI decisions were added.

## Current UI exposure

No UI behavior changed. Organize remains preview/review-only. There are no
enabled Apply, move, delete, cleanup, quarantine, restore, undo, or confirmation
controls.

## Tests

Validation commands for this sprint:

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `119` tests).
- `npm run build`: passed with the existing Vite chunk-size warning.

Rust validation is skipped because this sprint changes docs and a TypeScript
doc guard only.

## Desktop/runtime proof

Desktop proof and smoke are skipped because this sprint does not change visible
route behavior.

## What could not be verified

Real Apply, backup execution, restore execution, result-log persistence,
confirmation, and fixture recovery behavior remain intentionally unimplemented.

## Linear updates

- `VEL-18`: updated as the primary backup/restore design issue.
- `VEL-19`: updated as related dry-run/apply split context.
- `VEL-20`: updated as confirmed Apply prototype blocker context.
- `VEL-31`: updated as ApplyPlan persistence context.

## Recommended next sprint

Result/restore schema foundation or a fixture-only backup prototype design.
Do not start real Apply yet.

## Docs updated

- `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated worktree changes

Known unrelated dirty files were present before this sprint and must remain
unstaged unless they contain sprint-relevant hunks:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing handoff/status hunks unrelated to this sprint

## Commit

- Design commit: `c2497ca` -
  `Design backup restore result log contract`.
- Delivery commit: recorded in the final handoff after this report update.
- Branch: `codex/backup-restore-result-log-design-v1`.
- Draft PR: `https://github.com/makiaveli1/simsuite/pull/20`.

## Final honest verdict

Verified: Backup / Restore / Result Log Design v1 is complete and ready for the
next safe implementation step.
