# Dry-run Apply Design v1 Report

Date: 2026-05-24

## What was audited

- Saved draft ApplyPlan records and saved-plan review context.
- Validation preview design and UI.
- Recovery history UI and DB-only result/restore metadata.
- Fixture-only backup, restore, and integration proof reports.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Trust-boundary copy guard.
- Linear issue context for dry-run/apply split.

## Dry-run gap

SimSuite can save, review, validate, and show recovery metadata for draft
plans. It still cannot explain, in one dry-run response, which items a future
Apply would skip, which would remain blocked, which would require backup,
which would require confirmation, and why no file-changing action is allowed.

## Existing systems reused

- Saved ApplyPlan records.
- Saved ApplyPlan item snapshots.
- Saved ApplyPlan item signals and blockers.
- Validation preview output.
- Result/restore schema foundation.
- Recovery history UI.
- Fixture-only backup/restore proof as private test prior art.
- Library identity/current paths and scanner roots through validation.
- Apply Safety Contract.
- Existing Systems Integration Contract.

## New data or logic added

- New source-of-truth dry-run design document.
- New sprint report.
- Lightweight trust-boundary doc guard.
- Current-state notes in existing planning and memory docs.

No runtime command, API wrapper, migration, UI, confirmation workflow, Apply,
Restore, backup execution, restore execution, file operation, or fixture helper
exposure was added.

## Dry-run design

Dry-run Apply is a read-only rehearsal. It should explain what SimSuite would
try later, what it would skip, and what safety gates are still missing.

Future status names include `not_run`, `blocked`, `would_skip`,
`would_require_review`, `would_require_backup`,
`would_require_confirmation`, `would_require_destination_review`,
`candidate_after_future_safety_gates`, and `error`.

The recommended future command is `preview_apply_plan_dry_run`. It should be
read-only, response-only first, and must always return
`canProceedToApply=false` and `canProceedToConfirmation=false` in v1.

## What this means for the user

Users still cannot Apply or Restore changes. This design defines how a future
dry-run preview should explain possible later actions and blockers before any
file-changing feature exists.

## Trust / safety boundary

No Apply was added. No Restore was added. No confirmation workflow was added.
No files were moved, copied, created, deleted, cleaned up, quarantined,
replaced, or auto-sorted. No fixture helper was exposed. No AI decision was
added. No files changed. Apply is not ready yet. Restore is not ready yet.

## Future command recommendation

Implement `preview_apply_plan_dry_run` next as backend/API-only and read-only:

- input: `{ planId: number }`.
- output: `ApplyPlanDryRunPreview`.
- no DB writes in v1.
- no backup creation.
- no restore entry creation.
- no result-log creation.
- no folder creation.
- no Apply.
- no confirmation token.

## Current UI exposure

No visible Apply, Restore, Backup, move, delete, cleanup, quarantine,
replacement, or auto-sort execution control was added.

## Tests

- `npx tsc --noEmit`
  - passed.
- `npm run test:unit`
  - passed, 28 files and 129 tests.
- `npm run build`
  - passed. The existing Vite chunk-size warning remains.
- Rust validation was skipped because this sprint touched docs and a frontend
  test guard only.

## Desktop/runtime proof

Skipped because this sprint added no visible route behavior.

## Linear updates

- Used `VEL-19 - [M4-2] Dry-run/apply split` as the primary issue.
- Commented on `VEL-18`, `VEL-20`, `VEL-31`, `VEL-32`, `VEL-33`, and
  `VEL-34` as related context.
- Did not close real Apply or Restore issues.

## What could not be verified

Real Apply, real Restore, confirmation, user-file backup/restore execution,
and the future dry-run command remain intentionally unimplemented. This sprint
verified only the docs/test-guard design layer.

## Recommended next sprint

Implement the read-only `preview_apply_plan_dry_run` command v1. Still no real
Apply.

## Docs updated

- `docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`
- `simsuite-reports/DRY_RUN_APPLY_DESIGN_V1_REPORT.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `src/trustBoundaryCopy.test.ts`

## Unrelated worktree changes

Known unrelated dirty files were present before this sprint and must stay
outside the implementation:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

Pending.

## Final honest verdict

Verified: Dry-run Apply Design v1 is complete and ready for the next safe
implementation step.
