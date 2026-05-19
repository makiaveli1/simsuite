# Fixture Backup + Restore Integration Proof v1 Report

Date: 2026-05-19

## What was audited

- Fixture-only backup prototype.
- Fixture-only restore prototype.
- DB-only ApplyPlan run/result/restore metadata foundation.
- Restore-map scope validation for one run, plan, item, and result.
- Existing command registration and source guards.
- Backup / Restore / Result Log Design v1.
- Apply Safety Contract and Existing Systems Integration Contract.

## Integration decision

This sprint adds a private backend-only integration proof around the existing
fixture backup and restore helpers. The proof backs up a temporary fixture
source file, records safe result/restore metadata, restores from the recorded
backup reference into a temporary fixture target, verifies the restored copy,
and checks that unsafe statuses are never recorded.

No Tauri command, TypeScript API, visible UI, real Apply, or real Restore was
added.

## Existing systems reused

- Existing fixture-only backup prototype.
- Existing fixture-only restore prototype.
- Existing SHA-256 helper and fixture path-boundary checks.
- Saved ApplyPlan records and items.
- DB-only ApplyPlan run/result/restore metadata helpers.
- Existing safe statuses: `pending_log`, `failed_before_change`, and
  `design_only`.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Backup / Restore / Result Log Design v1.

## New data or logic added

- Test-local integration helper for the full fixture backup -> restore chain.
- Rust integration tests for backup verification, restore verification, DB
  metadata scope, overwrite refusal, mismatch rejection, and unsafe-status
  guards.
- Current-state docs and sprint report.

## What changed

- Backend tests now prove the fixture backup and restore prototypes work
  together as one recovery chain.
- Docs now record that the integrated chain is temporary-fixture proof only.

## What this means for the user

Users still cannot Apply or Restore changes. This work proves the full recovery
chain with fake temporary files only, so future recovery work can be safer
before any user-file workflow exists.

## Trust / safety boundary

No Apply was added. No real Restore was added. No user files are moved, copied,
created, deleted, cleaned up, quarantined, replaced, or auto-sorted. The only
backup and restore copies in this sprint are limited to temporary test fixtures
under a supplied fixture root. No user files changed. Apply is not ready yet.
Restore is not ready yet.

## Integration behavior

- Creates a temporary fixture source file.
- Runs the existing fixture backup helper.
- Verifies source and backup bytes, size, and SHA-256 hash.
- Verifies the backup result row and restore-map reference.
- Runs the existing fixture restore helper from that recorded backup reference.
- Verifies backup and restored bytes, size, and SHA-256 hash.
- Verifies restore result and restore-map metadata.

## Result/restore records

- Backup attempts record safe `pending_log` result rows.
- Backup attempts record safe `design_only` restore-map rows.
- Restore attempts record safe `pending_log` result rows.
- Restore attempts record safe `design_only` restore-map rows.
- Safe pre-copy failures continue to use `failed_before_change`.
- No `applied`, `restored`, `moved`, `copied`, `restore_complete`, or
  `apply_complete` status is recorded.

## Scope and path guards

- Restore remains scoped to one ApplyPlan run, plan, item, and result.
- Mismatched run, plan, item, or result references are rejected.
- Source, backup root, backup path, and restore target paths must stay inside
  the supplied fixture root.
- Restore target overwrite is refused.
- `fixture_mode=false` is rejected.

## Current UI exposure

No visible UI changed. There is no Apply, Restore, Backup, result-log, move,
delete, cleanup, quarantine, or replacement control.

## Tests

- `cargo test --manifest-path src-tauri/Cargo.toml apply_plan_backup_prototype -- --nocapture` passed: 24 passed.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 28 files, 124 tests.
- `npm run build` passed with the existing Vite chunk-size warning.
- `cd src-tauri && cargo fmt` completed.
- `cd src-tauri && cargo test apply_plan_backup_prototype -- --nocapture` passed: 24 passed.
- `cd src-tauri && cargo check` passed with existing Rust warning noise.
- `cd src-tauri && cargo test` passed: 327 passed, 0 failed, 2 ignored.
- `cd src-tauri && cargo build --release` passed with existing Rust warning noise.
- `npm run test:rust` passed: 327 passed, 0 failed, 2 ignored.

## Desktop/runtime proof

Desktop proof and smoke are skipped unless visible UI changes unexpectedly.
This sprint is backend/test-only.

## What could not be verified

Real Apply, user-file backup execution, user-file restore execution,
confirmation, partial Apply failure handling, and visible recovery UX remain
intentionally unimplemented.

## Linear updates

Pending final delivery updates.

## Recommended next sprint

Result/restore review UI or another fixture-only recovery proof. Do not start
real Apply yet.

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

Known unrelated dirty files were present before this sprint and must stay
outside the implementation:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

Pending final commit and PR details.

## Final honest verdict

Verified: Fixture-only Backup + Restore Integration Proof v1 is working for
the tested paths.
