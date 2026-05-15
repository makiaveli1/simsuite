# Validation & Conflict Preview Design v1 Report

Date: 2026-05-15

Branch: `codex/validation-conflict-preview-design-v1`

## What was audited

- Saved-plan UI and current Organize ownership.
- Saved ApplyPlan persistence fields, including existing nullable
  `validation_status` and `conflict_status` item columns.
- ApplyPlan builder and persistence contracts.
- Library file identity, scanner/settings root ownership, and folder metadata
  responsibilities.
- Existing move-engine preflight and snapshot/restore prior art.
- Trust-boundary, Apply Safety Contract, Existing Systems Integration
  Contract, navigation, backend map, handoff, and implementation status docs.

## Current validation gap

Saved draft preview plans can be created, listed, opened, and cancelled.
However, they are not validated for any future Apply workflow.

Missing today:

- no validation preview command.
- no conflict preview command.
- no backup/restore ApplyPlan result model.
- no result log.
- no real Apply.

The existing `apply_plan_items.validation_status` and
`apply_plan_items.conflict_status` fields are ready for future persisted
validation state, but this sprint does not use them at runtime.

## Existing systems reused

Future validation should reuse:

- saved ApplyPlan draft records and item snapshots.
- saved item source signals and blockers.
- Library file ids, current indexed paths, file names, size/date, and hashes
  where available.
- scanner-owned source roots and folder metadata.
- configured Mods, Tray, and Downloads paths.
- duplicate detector truth and review-only comparison state.
- review queue membership and reasons.
- update/watch caveats.
- Inbox intake context when relevant.
- snapshot/restore prior art only as a design input.
- Apply Safety Contract and Existing Systems Integration Contract.

## New data or logic added

This sprint adds docs and one test guard only.

No runtime model, command, migration, API wrapper, visible route behavior, file
operation, folder operation, Apply workflow, AI decision, result log, or restore
entry was added.

## Validation design decision

Validation/conflict preview should be the future bridge between saved draft
plans and any later confirmation design.

Final validation status names:

- `not_validated`
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
- `backup_required`
- `error`

Validation output must keep `canProceedToConfirmation=false` until validation,
confirmation, backup/restore, conflict handling, result logs, tests, and proof
exist.

## Conflict design decision

Final conflict status names:

- `not_checked`
- `none`
- `destination_exists`
- `same_name_conflict`
- `case_conflict`
- `folder_missing`
- `permission_unknown`
- `source_missing`
- `path_too_long`
- `cross_root_blocked`
- `unsupported`

Conflict preview must not create folders, write files, or touch destination
paths.

## What changed

- Added `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`.
- Added this implementation report.
- Updated current-state docs to point to the new validation/conflict preview
  design.
- Added a lightweight trust-boundary doc guard for the new design document.

## What this means for the user

Users still cannot Apply changes.

This work defines how SimSuite will later warn them when a saved draft plan has
missing files, stale paths, unsafe destinations, destination conflicts,
review-only items, or missing backup/restore requirements before any future
file-changing workflow can be considered.

## Trust / safety boundary

This is not real Apply.

- No Apply button was added.
- No file movement, copy, folder creation, delete, cleanup, quarantine,
  replacement, auto-sort execution, update replacement, or AI decision was
  added.
- No existing mutating backend command was exposed.
- No result log or restore entry table was added.
- No visible UI behavior changed.
- The future validation recommendation still keeps `No files changed` and
  `canProceedToConfirmation=false`.

## Future command recommendation

Recommended next sprint:

`preview_apply_plan_validation`

Classification:

- read-only validation preview response first.
- no DB write in v1 unless separately justified.
- no folder creation.
- no backup creation.
- no file movement.
- no Apply.

`refresh_apply_plan_validation` should wait until the read-only command and UI
meaning are proven.

## Current UI exposure

Organize still exposes Create plan, Saved plans, and Pending batches as
preview/review workflows only. Saved plans can be saved, inspected, and
cancelled as draft records, but the visible UI still does not expose enabled
Apply, move, cleanup, delete, quarantine, replacement, or auto-sort controls.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `112` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

Rust validation was skipped because this sprint did not touch Rust/backend
files. Desktop proof/smoke were skipped because no visible route behavior
changed.

## Desktop/runtime proof

Desktop proof was skipped. This sprint is docs plus a test guard only and does
not change visible route behavior.

## What could not be verified

- Real Apply was not verified because it does not exist and remains out of
  scope.
- Validation/conflict command behavior was not verified because this sprint
  intentionally does not implement that command.
- Result logs and restore entries remain future work.

## Linear updates

Pending at report update time. Relevant issues include `VEL-18`, `VEL-19`,
`VEL-20`, and `VEL-31`, plus any focused validation/conflict-preview issue
found by search.

## Recommended next sprint

Implement the read-only `preview_apply_plan_validation` command v1.

Still no real Apply.

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

Known unrelated dirty files remain outside this sprint:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

Pending at report creation time.

## Final honest verdict

Verified: Validation & Conflict Preview Design v1 is complete and ready for the
next safe implementation step.
