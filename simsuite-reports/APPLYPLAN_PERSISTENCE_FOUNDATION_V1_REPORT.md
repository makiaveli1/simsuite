# ApplyPlan Persistence Foundation v1 Report

Date: 2026-05-15

Branch: `codex/applyplan-persistence-foundation-v1`

## What was audited

- `StagingPlan` / `StagingPlanItem` Rust and TypeScript models.
- Sorting preview plan output from `generate_sorting_preview_plan`.
- SQLite migration and `ensure_schema` conventions.
- Existing command registration in `src-tauri/src/commands/mod.rs` and `src-tauri/src/lib.rs`.
- Existing TypeScript API and mock conventions in `src/lib/api.ts`.
- Trust, Apply Safety Contract, Existing Systems Integration Contract, navigation, and backend map docs.
- Current visible UI exposure. No visible route changes were made.

## Current persistence gap

Before this sprint, generated organization preview plans existed only as API responses and mock data. `StagingPlan` remained preview-only with `wouldTouchFiles=false`, and the ApplyPlan persistence audit was design-only.

There was no runtime table, model, command, or TypeScript API for saving a reviewed preview plan as a draft ApplyPlan record.

## Persistence foundation decision

This sprint implements a DB-only/read-only foundation for draft preview ApplyPlan records.

Implemented:

- SQLite tables for saved preview plans, items, source signals, and blockers.
- `ensure_schema` repair for existing databases.
- Rust persistence models.
- DB-only Tauri commands to save, list, read, and soft-cancel draft preview plans.
- TypeScript API types, wrappers, and mock in-memory persistence.
- Focused Rust and TypeScript tests.

Intentionally not implemented:

- real Apply.
- file movement, copying, deletion, cleanup, quarantine, replacement, or auto-sort execution.
- backup/restore execution or result-log execution tables.
- visible Apply UI or saved-plan UI.
- AI decisions.

## Existing systems reused

- `StagingPlan` and `StagingPlanItem` remain the source preview model.
- `generate_sorting_preview_plan` can provide preview-only organization plan data.
- Library file ids and indexed paths are stored as snapshots or soft references for future validation.
- Existing evidence levels, buckets, confidence labels, caveats, source signals, and blocked reasons are preserved.
- Apply Safety Contract rules gate all future file-changing work.
- Existing Systems Integration Contract rules require future builders to reuse scanner, Library, duplicate, review, update, Inbox, and Organize evidence before adding new logic.

## New data or logic added

- New migration `0003_applyplan_persistence_foundation.sql`.
- New tables: `apply_plans`, `apply_plan_items`, `apply_plan_item_signals`, `apply_plan_item_blockers`.
- New Rust module `core::apply_plan_persistence`.
- New Tauri commands:
  - `save_apply_plan_preview`
  - `list_saved_apply_plans`
  - `get_apply_plan`
  - `delete_draft_apply_plan`
- New TypeScript API methods:
  - `saveApplyPlanPreview`
  - `listSavedApplyPlans`
  - `getApplyPlan`
  - `deleteDraftApplyPlan`

All new runtime logic is database-only. It does not touch the filesystem.

## What changed

Backend/database:

- Added ApplyPlan persistence tables and indexes.
- Added schema repair through `ensure_schema`.
- Added persistence functions that reject any source plan or source item with `would_touch_files=true`.
- Added soft cancellation for draft preview plans.

TypeScript:

- Added persisted ApplyPlan types.
- Added API wrappers and mock persistence.
- Added tests for save/list/get/cancel behavior.

Docs:

- Updated current-state docs to record that persistence foundation now exists and remains preview-only.

## What this means for the user

Users still cannot Apply changes. No files are moved or changed.

This work gives SimSuite a safer place to remember reviewed preview-plan data later. Future work can build from saved draft plans instead of trying to jump straight from a preview screen to file movement.

## Trust / safety boundary

This is persistence only.

- No files changed.
- No Apply button was added.
- No move, copy, delete, cleanup, quarantine, replacement, auto-sort execution, or AI decision was added.
- Saved records are not proof that anything is safe to move.
- `applyable_items` is always `0` in this v1 foundation.
- Future Apply still requires confirmation, backup/restore, path validation, conflict handling, recoverable errors, result logs, and proof.

## Future tables implemented

| Table | Purpose |
| --- | --- |
| `apply_plans` | One saved draft/preview plan record with title, status, counts, caveats, source scope, and safety flags. |
| `apply_plan_items` | One saved item snapshot per preview item, including source path, destination preview string, evidence level, bucket, blocked/review-only state, and validation placeholders. |
| `apply_plan_item_signals` | Normalized source-signal/evidence snapshots for each item. |
| `apply_plan_item_blockers` | Normalized blocked, review-only, validation, and source-plan reasons for each item. |

`file_id` is intentionally a soft reference. Saved plan snapshots may outlive or be compared against current Library rows later.

## Future tables not implemented

- `apply_plan_results`
- `apply_plan_restore_entries`

Those remain future work for result-log and backup/restore sprints. They should not land until the next safety phase needs them.

## Commands added

| Command | Classification | Behavior |
| --- | --- | --- |
| `save_apply_plan_preview` | DB-only state-changing | Saves a preview-only `StagingPlan` snapshot as a draft/blocked preview ApplyPlan record. Rejects any source data that would touch files. |
| `list_saved_apply_plans` | Read-only | Returns summaries only, hides cancelled plans by default, and does not dump item paths. |
| `get_apply_plan` | Read-only | Returns the full saved plan, including items, signals, blockers, caveats, and source scope. |
| `delete_draft_apply_plan` | DB-only state-changing | Soft-cancels draft/preview records by setting status to `cancelled`. It does not delete files. |

## Current UI exposure

No visible UI was changed in this sprint.

Inbox, Organize, Pending Plans, and the direct Plan Preview route remain preview/review-only. No enabled Apply, move, cleanup, delete, quarantine, replacement, or auto-sort control was added.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `107` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `cargo fmt`: completed.
- `cargo check`: passed with existing warning noise.
- `cargo test`: passed (`277` passed, `2` ignored).
- `cargo build --release`: passed with existing warning noise.
- `npm run test:rust`: passed (`277` passed, `2` ignored).

## Desktop/runtime proof

Desktop proof and smoke were skipped because no visible route behavior changed.

## What could not be verified

- No desktop/runtime proof was run because this was backend/API/docs work with no visible route behavior changes.
- Linear and GitHub delivery details are recorded after commit/push/PR.

## Linear updates

Pending until commit/push/PR are complete.

Expected updates:

- create or update a focused ApplyPlan persistence issue if needed.
- comment on `VEL-31`, `VEL-18`, `VEL-19`, and `VEL-20` with validation, branch, commit, and PR details.

## Recommended next sprint

Read-only ApplyPlan builder from `StagingPlan` or saved-plan UI review, still with no real Apply.

Do not build real Apply until the Apply Safety Contract phases for backup/restore, path validation, conflict handling, confirmation, result logs, and proof are complete.

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

Known unrelated dirty files were left alone and must remain unstaged unless they contain this sprint's new top-note hunks:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

Pending.

## Final honest verdict

Verified: Read-only ApplyPlan Persistence Foundation v1 is working for the tested paths.
