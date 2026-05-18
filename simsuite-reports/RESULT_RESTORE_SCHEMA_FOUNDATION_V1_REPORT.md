# DB-Only Result / Restore Schema Foundation v1 Report

Date: 2026-05-18

## What was audited

- Backup/restore/result-log design in
  `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`.
- ApplyPlan persistence tables, helpers, commands, and TypeScript API/mock
  shape.
- Validation preview command and Organize saved-plan review boundary.
- SQLite migration and `ensure_schema` repair style.
- Existing snapshot, restore, move-engine, downloads apply/reject, and cleanup
  paths as prior art only.
- Existing docs, trust-boundary guards, command-registration tests, and API
  tests.

## Foundation decision

This sprint implements the DB-only schema foundation for future ApplyPlan run
metadata, per-file result logs, and restore-map references.

It intentionally does not implement real Apply, backup execution, restore
execution, file movement, file copying, folder creation, deletion, cleanup,
quarantine, replacement, auto-sort execution, AI decisions, or visible Apply /
Restore UI.

## Existing systems reused

- Existing saved ApplyPlan records in `apply_plans`.
- Existing saved ApplyPlan items in `apply_plan_items`.
- Existing saved blockers/signals as the evidence and review context.
- Existing validation preview as the current readiness gate.
- Existing SQLite migration and `ensure_schema` repair style.
- Existing Tauri command wrapper pattern.
- Existing TypeScript API/mock pattern in `src/lib/api.ts`.
- Apply Safety Contract.
- Existing Systems Integration Contract.

## New data or logic added

- New migration `0004_applyplan_result_restore_foundation.sql`.
- New tables: `apply_plan_runs`, `apply_plan_results`, and
  `apply_plan_restore_entries`.
- New Rust models for run logs, result logs, and restore entries.
- New DB-only helper module for creating/listing/getting log foundation rows.
- New DB-only Tauri commands for run/result/restore metadata.
- New TypeScript types, API wrappers, and mock persistence.
- New Rust and TypeScript tests for safe statuses and forbidden execution
  states.

## What changed

- Database schema now has future run/result/restore metadata tables.
- Backend schema repair can add those tables and indexes to existing databases.
- Backend command surface can create and read DB-only log rows.
- Frontend API/mocks can exercise the same DB-only contracts.
- Docs now record that this is a foundation only, not file execution.

## What this means for the user

Users still cannot Apply or Restore changes. This work creates the local
database foundation that future recovery and result-log features will need
before any file-changing feature can be considered.

## Trust / safety boundary

No Apply was added. No files are moved, copied, created, deleted, cleaned up,
quarantined, replaced, restored, or auto-sorted. No folders are created. No
backup execution or restore execution exists. No AI decisions were added.

## Tables added

- `apply_plan_runs`: one future run metadata record for a saved ApplyPlan.
- `apply_plan_results`: future per-file operation/result-log rows.
- `apply_plan_restore_entries`: future restore-map references scoped to one
  run/result/item.

## Commands added

All new commands are DB-only:

- `create_apply_plan_run_log`
- `list_apply_plan_run_logs`
- `get_apply_plan_run_log`
- `record_apply_plan_result_log`
- `list_apply_plan_result_logs`
- `record_apply_plan_restore_entry`
- `list_apply_plan_restore_entries`

The commands reject v1-forbidden execution statuses such as `applying`,
`applied`, `restored`, and related completion states.

This foundation does not execute Apply, does not execute backup, and does not execute restore.

## Current UI exposure

No visible UI changed. There is no Apply button, Restore button, result-log tab,
or file-changing control.

## Tests

- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 28 files, 122 tests.
- `npm run build` passed, with the existing Vite chunk-size warning.
- `cd src-tauri && cargo fmt` completed.
- `cd src-tauri && cargo check` passed, with existing Rust warning noise.
- `cd src-tauri && cargo test` passed: 303 passed, 0 failed, 2 ignored.
- `cd src-tauri && cargo build --release` passed, with existing Rust warning
  noise.
- `npm run test:rust` passed: 303 passed, 0 failed, 2 ignored.

## Desktop/runtime proof

Skipped. This sprint adds DB-only schema, backend helpers, command wrappers, API
types, mocks, tests, and docs only. No visible route behavior changed.

## What could not be verified

Real Apply, backup execution, restore execution, confirmation, and fixture
recovery behavior remain intentionally unimplemented.

Desktop proof was not run because there were no visible UI changes.

## Linear updates

To be completed after commit/push so the issue comments can include final
delivery metadata.

## Recommended next sprint

Fixture-only backup prototype design or DB-only result/restore review UI. Do
not start real Apply yet.

## Docs updated

- `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`
- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated worktree changes

Known unrelated dirty files were present before this sprint and were left
outside the implementation:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

To be completed after validation.

## Final honest verdict

Verified: DB-only Result / Restore Schema Foundation v1 is working for the
tested paths.
