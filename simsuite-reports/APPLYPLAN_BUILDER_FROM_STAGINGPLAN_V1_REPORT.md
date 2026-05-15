# ApplyPlan Builder From StagingPlan v1 Report

Date: 2026-05-15

Branch: `codex/applyplan-builder-from-stagingplan-v1`

## What was audited

- The DB-only ApplyPlan persistence foundation in
  `src-tauri/src/core/apply_plan_persistence.rs`.
- `StagingPlan` and `StagingPlanItem` models.
- The sorting preview generator in
  `src-tauri/src/core/rule_engine/sorting_plan.rs`.
- Tauri command wrappers and registration.
- TypeScript API and mock persistence.
- Apply Safety Contract, Existing Systems Integration Contract, navigation,
  trust, backend map, implementation status, and handoff docs.

## Current builder gap

Before this sprint, `generate_sorting_preview_plan` could create a preview-only
`StagingPlan`, and `save_apply_plan_preview` could persist a provided source
plan. There was no backend-owned command that generated the preview plan and
saved the draft ApplyPlan in one controlled path.

That gap mattered because future UI could be tempted to pass arbitrary
frontend-shaped item arrays. The safer path is for the backend to build from
SimSuite's existing sorting preview evidence.

## Builder decision

This sprint adds a backend-owned builder command:

- `build_apply_plan_from_staging_plan`

The command accepts a preview-generation request, calls the existing sorting
preview generator, then saves the result through the existing ApplyPlan
persistence foundation.

It returns the saved draft plan summary. Full saved plan details remain
available through `get_apply_plan`.

## Existing systems reused

- `generate_sorting_preview_plan` remains the source of organization preview
  evidence.
- `StagingPlan` and `StagingPlanItem` remain the preview source model.
- Library file ids, current paths, current roots, evidence levels, buckets,
  confidence labels, source signals, blocked reasons, and caveats are preserved
  through the saved draft record.
- The ApplyPlan persistence foundation stores the snapshot.
- The Apply Safety Contract and Existing Systems Integration Contract remain
  the gate for any future file-changing work.

## New data or logic added

- New Rust request model: `BuildApplyPlanFromStagingPlanRequest`.
- New backend builder function:
  `apply_plan_persistence::build_apply_plan_from_staging_plan`.
- New Tauri command: `build_apply_plan_from_staging_plan`.
- New TypeScript API wrapper: `api.buildApplyPlanFromStagingPlan`.
- New mock builder path for unit tests.
- Focused Rust and TypeScript tests for backend-owned draft-plan building.

No SQLite migration or new table was added.

## What changed

Backend/API:

- The backend can now generate a sorting preview plan and save it as a draft
  ApplyPlan snapshot in one controlled path.
- The builder forces the existing v1 safety posture: `wouldTouchFiles=false`
  and `applyableItems=0`.
- The builder does not accept arbitrary frontend item arrays.

TypeScript:

- Added the matching request type and API wrapper.
- Added mock behavior that generates a preview plan, persists it in the mock
  ApplyPlan store, and returns a saved draft summary.

Docs:

- Updated current-state docs to record that the builder exists and remains
  preview/persistence-only.

## What this means for the user

Users still cannot Apply changes, and no files are changed.

This makes saved draft plans more trustworthy for future work because SimSuite
can build them from its own backend evidence instead of trusting arbitrary UI
data.

## Trust / safety boundary

This is not real Apply.

- No files changed.
- No Apply button was added.
- No move, copy, delete, cleanup, quarantine, replacement, auto-sort execution,
  update replacement, or AI decision was added.
- The command does not call move-engine apply paths, staging commit/cleanup,
  Downloads apply/reject, restore, shell, delete, or quarantine paths.
- Saved draft plans are still not proof that files are safe to move.
- Future Apply still requires validation, confirmation, backup/restore,
  conflict handling, recoverable errors, result logs, and proof.

## Commands added

| Command | Classification | Behavior |
| --- | --- | --- |
| `build_apply_plan_from_staging_plan` | Backend-owned DB-only builder | Generates a read-only sorting preview plan, persists it as a draft ApplyPlan snapshot, and returns the saved summary. |

Existing commands remain unchanged:

- `save_apply_plan_preview`
- `list_saved_apply_plans`
- `get_apply_plan`
- `delete_draft_apply_plan`

## Current UI exposure

No visible UI was changed in this sprint. No enabled file-changing controls
were added.

## Tests

- `cargo test builder_generates_and_saves_draft_apply_plan_from_sorting_preview`:
  passed.
- `cargo test apply_plan_builder_command_stays_backend_owned_and_db_only`:
  passed.
- `cargo test apply_plan_persistence_commands_are_registered_with_tauri`:
  passed.
- `npm run test:unit -- src/lib/api.test.ts`: passed.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `108` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `cargo fmt`: completed.
- `cargo check`: passed with existing warning noise.
- `cargo test`: passed (`279` passed, `2` ignored).
- `cargo build --release`: passed with existing warning noise.
- `npm run test:rust`: passed (`279` passed, `2` ignored).

## Desktop/runtime proof

Desktop proof and smoke are expected to be skipped because this sprint makes no
visible route behavior change.

## What could not be verified

- Desktop/runtime proof was not run because no visible route behavior changed.
- Linear natural-language search could not be completed because the connector
  returned a runtime `tool not found` error. Known related issues were updated
  directly instead.

## Linear updates

- `VEL-31`: commented with implementation summary, validation, branch, commit,
  report, and PR.
- `VEL-18`: commented as related backup/restore work; result/restore execution
  remains future work.
- `VEL-19`: commented as related dry-run/apply split work; builder remains
  preview/persistence-only.
- `VEL-20`: commented that confirmed Apply remains blocked by validation,
  confirmation, backup/restore, result logs, and proof.
- No duplicate issue was created.

## Recommended next sprint

Saved-plan UI review in Organize, still with no real Apply.

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

- Implementation commit: `c6b0726`
- Branch pushed: `codex/applyplan-builder-from-stagingplan-v1`
- Draft PR: `https://github.com/makiaveli1/simsuite/pull/15`

## Final honest verdict

Verified: Read-only ApplyPlan Builder from StagingPlan v1 is working for the
tested paths.
