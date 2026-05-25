# Organize Dry-run Preview UI v1 Report

Date: 2026-05-25

Branch: `codex/organize-dry-run-preview-ui-v1`

## What was audited

- Organize saved-plan details.
- Existing validation preview UI.
- Existing Recovery history UI.
- `previewApplyPlanDryRun` TypeScript API and mock support.
- Dry-run Apply design and command report.
- Desktop proof script for Organize saved-plan flows.
- Trust-boundary copy guard.

## Built / Changed

Added a read-only `Dry-run preview` section inside Organize saved plan details.
The section calls the existing `previewApplyPlanDryRun` API from a
user-triggered preview control and displays dry-run classifications, summary
counts, caveats, required future safety steps, and item details.

The UI keeps validation preview and Recovery history separate.

## What this means for the user

Users can now view a read-only dry-run preview inside Organize saved plan
details. It explains how a saved draft would be classified for a future Apply
flow, but users still cannot Apply or Restore changes and no files are changed.

## Trust / safety boundary

- No Apply.
- No Restore.
- No confirmation.
- No file movement.
- No file copying.
- No folder creation.
- No backup execution.
- No restore execution.
- No result-log writes from dry-run UI.
- No restore-entry writes from dry-run UI.
- No fixture helper exposure.
- No AI decision.
- `canProceedToApply=false`.
- `canProceedToConfirmation=false`.
- item `canApply=false`.

## Existing systems reused

- Saved ApplyPlan records.
- Organize saved-plan details.
- Existing validation preview UI and API.
- Existing Recovery history context.
- Existing `previewApplyPlanDryRun` API wrapper and mock behavior.
- Existing path-shortening and `Technical details` patterns.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Trust-boundary guard tests.

## New data or logic added

- Dry-run-only React state in saved plan details.
- Read-only dry-run summary and grouped item rendering.
- Plan-switch clearing so stale dry-run results do not remain visible.
- Organize UI tests for dry-run loading, success, error, no-write, and
  plan-switch behavior.
- Desktop proof coverage for the visible dry-run preview section.
- Current-state documentation and this report.

No backend code, schema migration, Tauri command, fixture helper, or new
runtime file-operation path was added.

## Recovery / dry-run behavior

The UI displays:

- `Dry-run preview`.
- `No files changed`.
- `Apply is not ready yet`.
- `Future confirmation blocked`.
- grouped item states including skipped, blocked, review-needed,
  destination-review, backup-required, candidate-after-future-safety-gates, and
  error states.
- required future steps such as validation proof, backup, restore map, result
  log, explicit confirmation, and Apply executor proof.

Dry-run does not create Recovery history records.

## Current UI exposure

Organize still exposes no enabled Apply, Restore, Backup, confirmation, move,
delete, cleanup, quarantine, replacement, auto-sort, or file-changing controls.

## Tests

Validation completed:

- `npx tsc --noEmit` - passed.
- `npx vitest run src/screens/OrganizeScreen.test.tsx` - passed, 18 tests.
- `npm run test:unit` - passed, 28 files / 135 tests.
- `npm run build` - passed, with the existing Vite chunk-size warning.
- `npm run desktop:proof:fixtures` - passed; this command also rebuilt the
  Tauri release bundle and emitted existing Rust warning noise.
- `npm run desktop:smoke:fixtures` - passed; this command also rebuilt the
  Tauri release bundle and emitted existing Rust warning noise.

## Desktop/runtime proof

Desktop proof was updated and passed. It checked Organize, saved plan details,
validation preview, Recovery history, the new Dry-run preview section, dry-run
success copy, and forbidden enabled file-changing controls.

Screenshot captured:

- `C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\output\desktop\library-proof\2026-05-25T01-09-56-451Z\organize-dry-run-preview-ui-v1.png`

## What could not be verified yet

No separate manual desktop pass was run outside the automated desktop proof and
smoke scripts.

## Linear updates

- `VEL-19` updated as the primary dry-run/apply split issue.
- Related comments added to `VEL-18`, `VEL-20`, `VEL-31`, `VEL-32`,
  `VEL-33`, and `VEL-34`.
- No real Apply or Restore issues were closed.

## Recommended next sprint

Dry-run UI polish, confirmation design, or another read-only safety gate. Do
not start real Apply or real Restore.

## Docs updated

- `docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated worktree changes

Known unrelated dirty files were present before this sprint and must stay
unstaged unless their top current-state notes are updated:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- older unrelated lower hunks in `SESSION_HANDOFF.md`
- older unrelated lower hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Pending.

## Final honest verdict

Verified: Organize Dry-run Preview UI v1 is working for the tested paths.
