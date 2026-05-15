# Saved Plan UI Review v1 Report

Date: 2026-05-15

Branch: `codex/saved-plan-ui-review-v1`

## What was audited

- The existing Organize planning UI and tests.
- The `PendingPlansPreview` batch-handoff component.
- ApplyPlan API/types/mock behavior in `src/lib/api.ts` and `src/lib/types.ts`.
- The backend-owned builder command contract from
  `build_apply_plan_from_staging_plan`.
- Desktop proof coverage in `scripts/desktop/desktop-library-proof.mjs`.
- Apply Safety Contract, Existing Systems Integration Contract, navigation,
  trust, backend map, implementation status, and handoff docs.

## Current saved-plan UI gap

Before this sprint, Organize could generate preview-only sorting plans, and the
backend could build and persist draft ApplyPlan records. The user interface
could not save a generated preview, list saved draft plans, inspect saved plan
evidence, or cancel a draft record.

Real Apply still did not exist, and this sprint keeps that boundary.

## UI decision

Organize now has three focused workspace tabs:

- `Create plan` for generating a new preview suggestion.
- `Saved plans` for saved draft preview organization plans.
- `Pending batches` for imported/downloaded batch handoff notes.

This keeps saved organization drafts separate from Inbox-style intake batch
data, so imported/downloaded batches do not look like saved organization plans.

## Existing systems reused

- `generate_sorting_preview_plan` remains the source of preview suggestions.
- `build_apply_plan_from_staging_plan` is the only UI save path for generated
  preview plans.
- `list_saved_apply_plans`, `get_apply_plan`, and `delete_draft_apply_plan`
  power list, details, and draft cancellation.
- `StagingPlan` remains the preview source model.
- Existing ApplyPlan persistence stores the draft records.
- Organize remains the route owner for planning work.
- The Apply Safety Contract and Existing Systems Integration Contract remain
  the safety and reuse rules.

## New data or logic added

- New frontend component: `SavedPlansReview`.
- New Organize state for the latest preview request, save status, selected
  saved draft, and saved-plan refreshes.
- New saved-plan UI flows: save draft, list drafts, open details, and cancel
  draft records.
- New tests for save/list/detail/cancel behavior and trust-boundary copy.
- Updated desktop proof for the visible saved-plan workflow.

No backend command, database migration, new route, or new top-level page was
added.

## What changed

UI:

- Organize tabs now read `Create plan`, `Saved plans`, and `Pending batches`.
- `Create plan` shows `Save preview plan` only after a preview exists.
- Saving uses `api.buildApplyPlanFromStagingPlan` with the latest generation
  request, not frontend item arrays.
- `Saved plans` shows compact summaries without dumping item paths.
- Saved plan details show caveats, counts, source scope, evidence/confidence,
  source signals, blockers, shortened paths, and collapsed technical details.
- `Cancel draft` uses inline confirmation and explains that files are not
  touched.

Tests/proof:

- Organize tests cover the saved-plan section, save through builder, summaries,
  details, cancel, empty/error states, and forbidden controls.
- Desktop proof now verifies the saved-plan flow and captures
  `organize-saved-plan-ui-review-v1.png`.

Docs:

- Current-state docs now record that saved-plan review exists in Organize and
  remains preview-only.

## What this means for the user

Users can generate a preview organization plan, save it as a draft, return to it
later, review why SimSuite suggested each item, and cancel the draft record.

No files are moved, copied, deleted, replaced, cleaned up, or changed.

## Trust / safety boundary

This is not real Apply.

- No Apply button was added.
- No file movement, copy, delete, cleanup, quarantine, replacement, auto-sort
  execution, update replacement, or AI decision was added.
- The UI does not expose legacy mutating backend commands.
- Saved plans remain draft preview records with `No files changed` safety copy.
- Future Apply still requires validation, confirmation, backup/restore,
  conflict handling, result logs, and proof.

## Saved plan behavior

- `Save preview plan` saves the latest generated preview through the
  backend-owned builder path.
- `Saved plans` lists draft preview summaries only.
- `Plan details` fetches full saved data and shows caveats, evidence,
  blockers, and technical paths behind collapsed details.
- `Cancel draft` soft-cancels the saved draft record through the existing
  DB-only API and does not touch files.

## Current UI exposure

The visible app exposes no enabled Apply, move-files, cleanup, delete,
quarantine, replacement, auto-sort, or safe-to-delete controls from Organize.

## Files changed

- `src/screens/OrganizeScreen.tsx`: adds tabs and save-preview-plan behavior.
- `src/screens/organize/SavedPlansReview.tsx`: adds saved draft review UI.
- `src/screens/organize/PendingPlansPreview.tsx`: clarifies Pending batches.
- `src/screens/OrganizeScreen.test.tsx`: covers saved-plan UI behavior.
- `src/screens/StagingScreen.test.tsx`: updates direct route expectations.
- `src/trustBoundaryCopy.test.ts`: guards Organize save path and forbidden
  file-changing wording.
- `scripts/desktop/desktop-library-proof.mjs`: verifies the saved-plan flow and
  screenshot.
- Current-state docs and this report record the delivery.

## Tests

- `npm run test:unit -- src/screens/OrganizeScreen.test.tsx src/screens/StagingScreen.test.tsx src/trustBoundaryCopy.test.ts`:
  passed.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `111` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

Rust validation was skipped because no Rust/backend code changed.

## Desktop/runtime proof

- `npm run desktop:proof:fixtures`: passed.
- `npm run desktop:smoke:fixtures`: passed.

Proof screenshot:

- `output/desktop/library-proof/2026-05-15T17-40-09-265Z/organize-saved-plan-ui-review-v1.png`

An intermediate proof pass hit an existing Updates bridge timing race. The
proof now waits for the focused file context before asserting that bridge, and
the final proof/smoke checks passed.

## What could not be verified

- Real Apply was not verified because it does not exist and remains out of
  scope.
- Result logs, restore entries, validation/conflict checks, and real file
  movement remain future work.
- Rust-specific validation was not run because this sprint only changed
  frontend, tests, proof, and docs.

## Linear updates

Linear updates are intended for the focused saved-plan UI issue and related
M4 issues (`VEL-31`, `VEL-18`, `VEL-19`, and `VEL-20`) after validation.

## Recommended next sprint

Validation/conflict preview design for saved draft plans, still with no real
Apply.

## Docs updated

- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated worktree changes

Known unrelated dirty files were left alone and must remain unstaged unless they
contain this sprint's new top-note hunks:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

Pending final commit for this sprint: `Show saved preview plans in Organize`.

## Final honest verdict

Verified: Saved Plan UI Review v1 is working for the tested paths.
