# Suggested Plan Generator v1 Report

Date: 2026-05-13

Branch: `codex/suggested-plan-generator-v1`

## Pre-Implementation Audit

### 1. Current Model Fit

Current `StagingPlan` and `StagingPlanItem` are preview-only and already carry `wouldTouchFiles=false`, but the item model is too thin for per-file Auto Sorting suggestions.

Fields to add for v1:

- `leave_in_place` action kind, so the generator can explicitly recommend no move when evidence is weak or the current folder is already clear.
- `sourceSignals[]`, so each suggestion can say which evidence was used.
- `blockedReasons[]`, so stronger suggestions can explain why they were blocked.
- `bucket`, so the UI can group suggestions without parsing copy.
- `confidenceLabel`, so user-facing confidence can stay separate from backend evidence enums.
- `currentRoot`, so the plan can distinguish Mods, Tray, Downloads, Inbox, and unknown roots.

These fields are small serializable contract additions and do not require a database migration.

### 2. Scope Decision

The supported v1 scopes are:

- selected Library file IDs.
- a bounded Library folder scope for Mods or Tray.

The generator will not support whole-Library plans. Folder scope is capped by a backend limit. Staged batch scope is not implemented in this sprint because current Staging data is folder-level and cannot honestly provide per-file organization suggestions yet.

### 3. Evidence-To-Plan Mapping

The generator will implement the rules from `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`:

- exact duplicates -> `suggest_review`, bucket `needs_review`.
- parser warnings -> `suggest_review`, bucket `needs_review`.
- inspection/safety warnings -> `suggest_review`, bucket `needs_review`.
- review queue membership -> `suggest_review`, bucket `needs_review`.
- weak or unknown metadata -> `leave_in_place` or `suggest_review`.
- `.ts4script` / Script Mods with no blockers -> bucket `script_mods` with script placement caveat.
- Tray source or Tray extension -> bucket `tray`, review-safe and no cross-root move.
- strong CAS metadata -> bucket `cas`.
- strong Build/Buy metadata -> bucket `build_buy`.
- strong Gameplay metadata -> bucket `gameplay`.
- strong Presets & Sliders metadata -> bucket `presets_sliders`.
- strong Overrides & Defaults metadata -> bucket `overrides_defaults`.
- filename-only clues -> review-only support, never a strong move.
- same folder, same pack, and family hints -> grouping/review context only, no dependency claim.
- no clear benefit -> `leave_in_place`.

Suggested destination paths are preview strings only. They are included only when the configured target root exists and the suggestion is not blocked.

### 4. User Impact Plan

Users will be able to request a preview-only organization plan for selected Library files or a bounded folder scope. SimSuite will explain suggestions and caveats, but it will not move, delete, quarantine, replace, or auto-sort anything.

## What Was Audited

- `StagingPlan` Rust and TypeScript models.
- Existing Staging preview command and UI contract.
- Library file metadata, duplicate proof, parser warnings, safety notes, review queue, folder scope, and configured root settings.
- Legacy Organize and move-engine paths, to make sure the new generator does not route through Apply.
- Linear issues `VEL-16` and `VEL-17`.

## Generator Scope

- Supported now: selected Library file IDs.
- Supported now: bounded Mods/Tray Library folder scope.
- Not supported now: whole-Library plans.
- Not supported now: staged batch per-file plans, because current Staging data is still folder-level.

## Model Changes

- Added `leave_in_place` as a plan item action kind.
- Added `sourceSignals[]`, `blockedReasons[]`, `bucket`, `confidenceLabel`, and `currentRoot` to `StagingPlanItem`.
- Added `GenerateSortingPreviewPlanRequest` with `selected_files` and `library_folder` scopes.
- Kept `wouldTouchFiles=false` on every generated plan and item.
- No schema migration was needed.

## Rule Mapping

- Exact duplicates route to `suggest_review` in `needs_review`.
- Parser warnings, inspection warnings, and review queue membership route to `suggest_review`.
- Weak filename-only clues route to review-only suggestions, not move suggestions.
- Weak or unknown metadata routes to `leave_in_place` or review.
- `.ts4script` and Script Mods can preview the `script_mods` bucket with script placement caveats.
- Tray source or Tray extension routes to a Tray review plan, not a cross-root move.
- Strong indexed kind metadata can preview CAS, Build/Buy, Gameplay, Presets & Sliders, or Overrides & Defaults buckets.
- Missing configured target roots block destination paths.
- Suggested destination paths are preview strings only.

## What Changed

- Added `src-tauri/src/core/rule_engine/sorting_plan.rs`.
- Registered read-only Tauri command `generate_sorting_preview_plan`.
- Added API/mock support through `api.generateSortingPreviewPlan()`.
- Added backend and frontend/API tests for the generator contract.
- Updated trust, navigation, rules, backend map, handoff, status, and this report.

## What This Means For The User

Users can get a cautious preview plan for selected Library files or a bounded folder scope once the UI calls this command. SimSuite can explain suggested buckets, review reasons, and caveats. Nothing is moved, deleted, cleaned up, quarantined, replaced, or auto-sorted.

## Trust / Safety Boundary

This is still preview-only. The generator does not call Apply, move-engine apply paths, Staging commit/cleanup commands, shell operations, AI, delete, or quarantine behavior. Exact duplicate proof is review information, not a cleanup instruction. Filename, version, folder, pack, and family hints remain review-only unless stronger evidence exists.

## Backend Command Behavior

`generate_sorting_preview_plan` reads existing Library metadata and settings, then returns a `StagingPlan`.

It is bounded:

- selected IDs are deduplicated and capped.
- folder scope is limited, with a default cap of 100 and a hard cap of 250.
- folder scope supports Mods/Tray only.

It is read-only:

- no filesystem writes.
- no directory creation.
- no move/delete/quarantine/cleanup/commit calls.
- no thumbnail parsing.
- no AI.

## Frontend / API Behavior

- `src/lib/types.ts` exposes the new request and item fields.
- `src/lib/api.ts` exposes `generateSortingPreviewPlan(request)`.
- Mock API returns a preview-only plan with `wouldTouchFiles=false`.
- No visible Staging or Organize route behavior was changed in this sprint.

## Linear Updates

- Added a completion comment to `VEL-16` with the implemented scope and validation results.
- Added a follow-up comment to `VEL-17` noting that Organize plan review UI remains next.
- Did not create duplicate issues.
- Did not move issue status because the available direct Linear tools exposed comments but not status updates.

## Files Changed

- `src-tauri/src/models.rs`: added generator request and plan item fields.
- `src-tauri/src/core/rule_engine/sorting_plan.rs`: new read-only rule mapper and generator.
- `src-tauri/src/core/rule_engine/mod.rs`: exports the sorting plan module.
- `src-tauri/src/commands/mod.rs`: registers the command wrapper and read-only guard test.
- `src-tauri/src/lib.rs`: registers the Tauri command.
- `src-tauri/src/core/downloads_watcher/mod.rs`: fills new fields for existing staging preview items.
- `src/lib/types.ts`: adds TypeScript request and plan item fields.
- `src/lib/api.ts`: adds API and mock support.
- `src/lib/api.test.ts`: verifies mock preview-only plan fields.
- `src/screens/StagingScreen.test.tsx`: updates existing plan fixture shape.
- Planning/status docs and this report.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed, `24` files / `91` tests.
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `cargo fmt`: passed.
- `cargo check`: passed with existing Rust warning noise.
- `cargo test sorting_plan -- --nocapture`: passed, `12` generator tests.
- `cargo test`: passed, `268` passed / `2` ignored.
- `cargo build --release`: passed with existing Rust warning noise.
- `npm run test:rust`: passed, `268` passed / `2` ignored.

Note: an initial attempt to run `npm run test:unit -- --runInBand` failed because Vitest does not support that Jest option. The required `npm run test:unit` command passed afterward.

## Desktop / Runtime Proof

Skipped. This sprint added backend/API/types and docs only, with no visible route behavior change and no Organize plan review UI. Desktop proof should run when the next UI sprint exposes generated plans.

## What Could Not Be Verified

- Real user Library sorting quality was not verified.
- Generated plans are not visible in the app yet because Organize plan review UI was not built.
- Whole-Library planning was intentionally not implemented.
- Staged batch per-file plans were not implemented because current Staging data is folder-level.
- Linear status transitions could not be completed through the available direct tools.

## Recommended Next Sprint

Organize Plan Review UI v1: show generated plans in Organize, still preview-only, with reasons, caveats, buckets, source signals, blocked reasons, and no enabled Apply/move/delete/cleanup/quarantine controls.

## Docs Updated

- `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/SUGGESTED_PLAN_GENERATOR_V1_REPORT.md`

## Unrelated Worktree Changes

Left alone: `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, and pre-existing unrelated status/handoff hunks outside this sprint.

## Commit

Commit hash is reported in the final Codex response after the sprint commit is created.

## Final Honest Verdict

Verified: Suggested Plan Generator v1 is working for the tested paths.
