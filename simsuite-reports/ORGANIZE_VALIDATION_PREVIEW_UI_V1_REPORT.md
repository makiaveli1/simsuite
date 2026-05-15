# Organize Validation Preview UI v1 Report

Date: 2026-05-16

## Current UI gap

Saved draft plans can be opened from Organize `Saved plans`, and the backend
already exposes the read-only `preview_apply_plan_validation` command through
`api.previewApplyPlanValidation`.

Before this sprint, the UI could show saved plan details, evidence, blockers,
caveats, and safe draft cancellation, but it did not show validation/conflict
preview results. Apply remains unavailable.

## UI decision

Saved plan details now include a `Validation preview` section below the plan
summary and caveats.

The section:

- explains that the check is read-only.
- shows `No files changed`.
- starts in `Needs validation`.
- uses `Check saved plan` to call `previewApplyPlanValidation`.
- shows summary counts, caveats, item-level reasons, and required next steps.
- shows `Future confirmation blocked`.
- does not show an Apply button or file-changing action.

Full technical details stay collapsed.

## User impact

Users can check a saved draft plan and see what would block it before any
future Apply work exists. Nothing is moved or changed.

## What was audited

- Organize saved-plan UI and detail selection.
- Existing `SavedPlansReview` state and cancel-draft flow.
- TypeScript validation preview types and `api.previewApplyPlanValidation`.
- Mock validation behavior in `src/lib/api.ts`.
- Desktop proof script coverage for Organize saved plans.
- Trust-boundary copy guard.
- Planning, trust, backend-map, status, and handoff docs.
- Linear/GitHub delivery expectations.

## UI behavior

When a saved draft is selected, Organize shows a `Validation preview` panel.

Before the check runs, it shows `Needs validation` and a safe `Check saved plan`
button. Clicking the button calls the backend-owned validation API:
`api.previewApplyPlanValidation({ planId })`.

On success, the UI shows:

- plan-level validation state.
- summary counts for blocked, review-only, conflict, stale, missing-source,
  destination-conflict, and backup-required items.
- validation caveats.
- item-level validation and conflict labels.
- item-level reasons and required next steps.
- `Future confirmation blocked`.
- `No files changed`.

On error, it shows a safe error state and keeps the no-file-change boundary
visible.

## Existing systems reused

- `previewApplyPlanValidation`.
- Saved ApplyPlan records.
- Saved ApplyPlan item snapshots.
- Saved source signals and blockers.
- Existing Organize saved-plan UI.
- Existing ApplyPlan persistence/API types.
- Apply Safety Contract.
- Existing Systems Integration Contract.
- Trust-boundary copy guard.

## New data or logic added

- React UI state for validation preview loading, success, and error states.
- Friendly validation/conflict label mapping.
- Validation summary and item result rendering.
- Scoped Organize/Saved Plans CSS.
- Unit tests for validation preview UI behavior.
- Desktop proof updates for the new visible flow.

No backend command, migration, validation persistence, result log, restore
entry, Apply button, or file-changing workflow was added.

## What changed

- `SavedPlansReview` now renders validation preview inside saved plan details.
- Organize tests cover validation preview loading, status labels, caveats,
  safety copy, blocked confirmation, and safe errors.
- Desktop proof now opens Organize, saves a preview plan, checks the saved
  plan, verifies validation summary and no-file-change copy, and captures
  `organize-validation-preview-ui-v1.png`.
- Docs now say validation preview UI exists in Organize `Saved plans`.

## What this means for the user

Users can see why a saved draft plan is blocked, stale, conflicted,
review-only, or not ready for future confirmation. They still cannot Apply the
plan, and SimSuite does not change files.

## Trust / safety boundary

No Apply was added. The UI does not move files. No files are moved, copied,
created, deleted, cleaned up, quarantined, replaced, or auto-sorted. No AI
decisions were added. The UI calls the read-only validation preview command and
respects `canProceedToConfirmation=false`.

## Validation preview states shown

Plan-level labels:

- `Needs validation`
- `Blocked`
- `No current blocker found, but still preview-only`

Item validation labels:

- `No current blocker found`
- `Blocked`
- `Stale source`
- `Missing source`
- `Missing destination root`
- `Unsafe destination`
- `Destination exists`
- `Unsupported root move`
- `Review-only`
- `Duplicate review`
- `Backup required`
- `Could not validate`

Conflict labels:

- `Not checked`
- `No conflict found`
- `Destination exists`
- `Same name conflict`
- `Case conflict`
- `Folder missing`
- `Permission unknown`
- `Source missing`
- `Path too long`
- `Cross-root blocked`
- `Unsupported`

## Current UI exposure

Organize still does not expose enabled Apply, move, cleanup, delete,
quarantine, replacement, auto-sort, or confirmation controls. The visible
action is `Check saved plan`, which is a read-only validation preview.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `118` tests).
- `npm run build`: passed with the existing Vite chunk-size warning.
- `npm run desktop:proof:fixtures`: passed.
- `npm run desktop:smoke:fixtures`: passed.

Rust validation was not run as a separate command because no Rust/backend files
were changed. The desktop proof and smoke commands rebuilt the Tauri release
binary and showed the repo's existing Rust warning noise.

## Desktop/runtime proof

Desktop proof captured:

- Organize opens.
- Create Plan still works.
- Save preview plan still works.
- Saved plans list appears.
- Saved plan detail opens.
- Validation preview runs through `Check saved plan`.
- Validation summary appears.
- `No files changed` appears.
- no enabled file-changing controls appear.
- screenshot
  `output/desktop/library-proof/2026-05-15T23-48-54-014Z/organize-validation-preview-ui-v1.png`.

## What could not be verified

Real Apply, validation persistence, result logs, restore entries, and
confirmation remain intentionally unimplemented.

## Linear updates

Pending final Linear delivery comment after commit/PR.

## Recommended next sprint

Validation preview UX polish, then backup/restore/result-log design or a
dedicated validation UX hardening pass. Do not start real Apply yet.

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
- `src/styles/globals.css` unrelated Home/global CSS hunks
- pre-existing handoff/status hunks unrelated to this sprint

## Commit

Pending final commit.

## Final honest verdict

Verified: Organize Validation Preview UI v1 is working for the tested paths.
