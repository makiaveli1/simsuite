# Read-only Dry-run Apply Command v1 Report

Date: 2026-05-24

Branch: `codex/read-only-dry-run-apply-command-v1`

## What was audited

- Dry-run Apply design source of truth.
- Saved ApplyPlan persistence and item snapshot models.
- Read-only validation preview command and tests.
- DB-only result/restore metadata helpers and commands.
- Fixture-only backup/restore prototype boundaries.
- Tauri command registration.
- TypeScript API/mock patterns.
- Trust-boundary guard tests.

## Built / Changed

Implemented the first backend/API-only `preview_apply_plan_dry_run` command.
The command loads a saved draft ApplyPlan, reuses the existing read-only
validation preview, and classifies each saved item as blocked, skipped, needing
review, needing destination review, needing backup, or only a candidate after
future safety gates.

The command always returns `canProceedToApply=false`,
`canProceedToConfirmation=false`, and item `canApply=false`.

No UI was added.

## What this means for the user

Users still cannot Apply or Restore changes. SimSuite can now produce a
read-only dry-run preview through the backend/API, but it still cannot change
files.

## Trust / safety boundary

- No Apply.
- No Restore.
- No confirmation.
- No file movement.
- No file copying.
- No folder creation.
- No backup execution.
- No restore execution.
- No result-log writes from dry-run.
- No restore-entry writes from dry-run.
- No fixture helper exposure.
- No AI decision.
- `canProceedToApply=false`.
- `canProceedToConfirmation=false`.

## Existing systems reused

- Saved ApplyPlan records.
- Saved ApplyPlan item snapshots.
- Saved item blockers and signals.
- Existing `preview_apply_plan_validation` output.
- Result/restore metadata schema as required-safety-step context only.
- Library identity and configured root evidence through validation preview.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Trust-boundary guard test suite.

## New data or logic added

- Rust dry-run request/response models.
- `src-tauri/src/core/apply_plan_dry_run.rs` read-only classifier.
- Tauri command wrapper and registration for `preview_apply_plan_dry_run`.
- TypeScript dry-run types and `previewApplyPlanDryRun` API wrapper.
- Mock dry-run support for tests and non-Tauri development.
- Rust and TypeScript safety tests.

No schema migration was added. Dry-run v1 has no DB persistence.

## Dry-run behavior

- Blocked saved-plan items stay blocked.
- Review-only items require manual review.
- Missing or stale sources are skipped.
- Destination conflicts require destination review.
- Items without a current validation blocker still require backup, restore-map,
  result-log, confirmation, and executor proof.
- The command never marks a plan or item as Apply-ready.

## Current UI exposure

No visible UI changed. Organize still has no Apply, Restore, Backup,
confirmation, move, delete, cleanup, quarantine, replacement, or auto-sort
controls for this command.

## Tests

Validation completed:

- `cd src-tauri && cargo test apply_plan_dry_run -- --nocapture` - passed.
- `npx tsc --noEmit` - passed.
- `npm run test:unit` - passed, 28 files / 131 tests.
- `npm run build` - passed, with the existing Vite chunk-size warning.
- `cd src-tauri && cargo fmt` - passed.
- `cd src-tauri && cargo check` - passed, with existing warning noise in older
  modules.
- `cd src-tauri && cargo test` - passed, 333 passed / 2 ignored.
- `cd src-tauri && cargo build --release` - passed, with existing warning noise
  in older modules.
- `npm run test:rust` - passed, 333 passed / 2 ignored.

## Visual / runtime proof

Desktop proof was skipped because this sprint adds no visible route behavior.

## Linear / PR

Linear updated:

- `VEL-19` primary dry-run/apply split issue.
- Related comments added to `VEL-18`, `VEL-20`, `VEL-31`, `VEL-32`,
  `VEL-33`, and `VEL-34`.

Branch pushed:

- `codex/read-only-dry-run-apply-command-v1`

Draft PR:

- https://github.com/makiaveli1/simsuite/pull/27

## What could not be verified yet

No desktop UI behavior was verified because none changed.

## Recommended next sprint

Build a read-only dry-run UI inside Organize saved plan details, or design the
confirmation layer. Do not start real Apply.

## Docs updated

- `docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`
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

## Unrelated worktree changes

Known unrelated dirty files were present before this sprint and were left
unstaged unless a top current-state note was updated:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- older unrelated lower hunks in `SESSION_HANDOFF.md`
- older unrelated lower hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

- Implementation commit: `78005da` -
  `Implement read-only dry-run Apply preview`.
- Delivery-report update commit is recorded in Git history after this report
  update.

## Final honest verdict

Verified: Read-only Dry-run Apply Command v1 is working for the tested paths.
