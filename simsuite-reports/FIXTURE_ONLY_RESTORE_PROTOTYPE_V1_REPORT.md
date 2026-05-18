# Fixture-Only Restore Prototype v1 Report

Date: 2026-05-18

## What was audited

- Backup / restore / result-log design.
- Fixture-only backup prototype.
- DB-only ApplyPlan run/result/restore metadata helpers.
- Snapshot, restore, move-engine, downloads apply/reject, and cleanup paths as
  prior art only.
- Existing command registration and trust-boundary tests.

## Prototype decision

This sprint adds a backend-only fixture restore prototype. It copies from a
recorded backup reference into a temporary fixture restore target, verifies size
and SHA-256 hash, records safe result-log and restore-map metadata, and keeps
the prototype out of Tauri commands, TypeScript API, and visible UI.

It does not add real Apply, real Restore, user-file backup execution, user-file
restore execution, file movement, user-file copying, folder creation outside
temp fixtures, deletion, cleanup, quarantine, replacement, auto-sort execution,
or AI decisions.

Temporary test files only.
Restore is not ready yet.

## Existing systems reused

- Fixture-only backup prototype path and hash guard patterns.
- Saved ApplyPlan records.
- Saved ApplyPlan items.
- Existing result/restore DB foundation tables.
- Existing DB-only result-log and restore-entry helper functions.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Backup / Restore / Result Log Design v1.

## New data or logic added

- New private fixture restore request/result structs.
- Restore-map scope validation against one run, plan, item, and result.
- Temp-directory path boundary checks for backup and restore target paths.
- Fixture restore copy and SHA-256 verification.
- Safe DB result-log recording with `pending_log` or `failed_before_change`.
- Safe restore-map recording with `design_only`.
- Rust tests proving temp fixture restore behavior and command non-exposure.
- Trust-boundary doc guard.

## What changed

- Backend gained a private fixture-only restore helper inside
  `apply_plan_backup_prototype`.
- Tests now prove copy-backup-first restore mechanics against temporary files.
- Docs now record that this is prototype proof only, not user backup/restore.

## What this means for the user

Users still cannot Apply or Restore changes. This work proves restore mechanics
only on temporary test files, so future recovery work can be safer before any
user-file workflow exists.

## Trust / safety boundary

No Apply was added. No real Restore was added. No user files are moved, copied,
created, deleted, cleaned up, quarantined, replaced, or auto-sorted. The only
restore copy in this sprint is limited to temporary test fixtures under a
supplied fixture root. No user files changed. Apply is not ready yet. Restore is
not ready yet.

## Fixture restore behavior

- Requires `fixture_mode=true`.
- Requires an existing restore-map row for the same run, plan, item, and result.
- Requires backup and restore target paths to stay inside a supplied temp
  fixture root.
- Requires the restore target parent directory to already exist.
- Refuses existing restore targets instead of overwriting.
- Copies a temp backup file into a temp restore target.
- Verifies restored size and SHA-256 hash against the backup.
- Leaves the backup file unchanged.

## Result/restore records

- Successful fixture restores record `pending_log` result rows.
- Successful fixture restores record `design_only` restore-map rows.
- Missing fixture backups record `failed_before_change` result rows and no new
  restore-map row.
- No `applied`, `restored`, `moved`, `copied`, or restore-complete status is
  recorded.

## Current UI exposure

No visible UI changed. There is no Apply, Restore, Backup, result-log, move,
delete, cleanup, quarantine, or replacement control.

## Tests

- `cargo test --manifest-path src-tauri/Cargo.toml apply_plan_backup_prototype -- --nocapture` passed: 21 passed.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 28 files, 124 tests.
- `npm run build` passed with the existing Vite chunk-size warning.
- `cd src-tauri && cargo fmt` completed.
- `cd src-tauri && cargo check` passed with existing Rust warning noise.
- `cd src-tauri && cargo test` passed: 324 passed, 0 failed, 2 ignored.
- `cd src-tauri && cargo build --release` passed with existing Rust warning noise.
- `npm run test:rust` passed: 324 passed, 0 failed, 2 ignored.

## Desktop/runtime proof

Desktop proof and smoke were skipped because this sprint made no visible route
or UI behavior changes.

## What could not be verified

Real Apply, user-file backup execution, user-file restore execution,
confirmation, partial Apply failure handling, and visible recovery UX remain
intentionally unimplemented.

## Linear updates

To be completed after validation and commit.

## Recommended next sprint

Fixture-only backup + restore integration proof or result/restore review UI. Do
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

Verified: Fixture-only Restore Prototype v1 is working for the tested paths.
