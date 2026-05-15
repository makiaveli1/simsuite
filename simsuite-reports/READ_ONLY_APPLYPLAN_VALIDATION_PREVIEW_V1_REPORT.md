# Read-only ApplyPlan Validation Preview v1 Report

Date: 2026-05-15

## What was audited

- Validation design in `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`.
- Saved ApplyPlan persistence and item/signal/blocker records.
- Current `apply_plan_items.validation_status` and `conflict_status` columns.
- Library file identity/current path data in the `files` table.
- Library settings for configured Mods and Tray roots.
- Tauri command registration and TypeScript API/mock boundaries.
- Trust-boundary tests and docs.
- Linear/GitHub delivery expectations.

## Validation preview behavior

`preview_apply_plan_validation` is a backend-owned, read-only command. It loads a
saved draft ApplyPlan, compares each saved item snapshot with current Library
rows and configured roots, and returns a validation/conflict preview.

Implemented v1 checks:

- saved blockers.
- review-only items.
- duplicate-review blockers.
- missing `file_id` or missing current Library row.
- stale saved source path versus current indexed path.
- missing destination root configuration.
- unsafe destination path outside configured Mods/Tray root.
- unsupported cross-root movement.
- destination path already exists.
- backup/restore requirement caveat.

The command always returns `canProceedToConfirmation=false`, and every item
returns `canApplyLater=false`.

## Existing systems reused

- Saved ApplyPlan records from `apply_plans`.
- Saved item snapshots from `apply_plan_items`.
- Saved blocker and source-signal rows.
- Library file identity/current paths from indexed `files` rows.
- Configured Mods/Tray roots from existing Library settings.
- Apply Safety Contract wording and status boundaries.
- Existing Systems Integration Contract evidence-reuse rules.

## New data or logic added

- Rust validation preview models.
- TypeScript validation preview types.
- `src-tauri/src/core/apply_plan_validation.rs`.
- Tauri command wrapper and command registration for
  `preview_apply_plan_validation`.
- TypeScript API/mock support through `api.previewApplyPlanValidation`.
- Rust and TypeScript tests for response-only validation behavior.

No SQLite migration, result-log table, restore table, validation persistence,
or visible UI was added.

## What changed

- Backend can now produce a validation/conflict preview for saved draft plans.
- Frontend API code can call the new preview command and mock it in unit tests.
- Docs now say the first read-only validation preview command exists.

## What this means for the user

Users still cannot Apply changes. SimSuite can now explain, through a backend
API, why a saved draft plan is blocked or needs review before any future Apply
work exists. No files are moved or changed.

## Trust / safety boundary

No Apply was added. No files are moved, copied, deleted, cleaned up,
quarantined, replaced, or auto-sorted. The command does not create folders,
create backups, write result logs, persist validation state, or call mutating
backend commands. Future confirmation remains blocked.

## Validation statuses implemented

- `valid_preview_only`
- `blocked`
- `stale_source`
- `missing_source`
- `missing_destination_root`
- `unsafe_destination`
- `destination_exists`
- `unsupported_cross_root`
- `review_only_blocked`
- `duplicate_review_blocked`
- `error`

`backup_required` remains represented as a plan caveat and summary count in v1.

## Conflict statuses implemented

- `not_checked`
- `none`
- `destination_exists`
- `permission_unknown`
- `source_missing`
- `cross_root_blocked`
- `unsupported`

Other designed conflict statuses remain future until a UI and broader proof
justify them.

## Current UI exposure

No visible UI changed in this sprint. Organize saved plans remain review-only
and do not expose Apply or file-changing controls.

## Tests

Focused checks already run during implementation:

- `cargo test apply_plan_validation --quiet`: passed.
- `npm run test:unit -- --run src/lib/api.test.ts`: passed.

Full validation:

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `114` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `cargo fmt`: completed.
- `cargo check`: passed with existing warning noise.
- `cargo test`: passed (`291` passed, `2` ignored).
- `cargo build --release`: passed with existing warning noise.
- `npm run test:rust`: passed (`291` passed, `2` ignored).

## Desktop/runtime proof

Desktop proof/smoke is skipped for this sprint because no visible route behavior
changed. If a later UI sprint surfaces validation previews, desktop proof should
cover Organize saved-plan validation states and no file-changing controls.

## What could not be verified

- No visible validation UI exists yet.
- No real Apply, result-log, or restore workflow exists.
- Live user-library validation behavior was not exercised in this sprint.

## Linear updates

Pending final delivery. Relevant issues should include the validation/conflict
preview issue plus `VEL-18`, `VEL-19`, `VEL-20`, and `VEL-31` where available.

## Recommended next sprint

Add validation preview UI to Organize `Saved plans`, still with no real Apply.

## Docs updated

- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`
- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
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

Pending final validation.

## Final honest verdict

Pending final validation.
