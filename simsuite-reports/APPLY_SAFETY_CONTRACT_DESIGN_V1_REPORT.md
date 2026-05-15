# Apply Safety Contract Design v1 Report

Date: 2026-05-15

Branch: `codex/apply-safety-contract-design-v1`

## 1. Existing File-Changing Command Inventory

The current backend already contains file-changing and state-changing paths.
This sprint does not expose or call them. They are documented so future work can
gate them behind a real safety contract.

| Name | Location | What it can do | Current UI exposure | Current tests/proof | Safety gaps | Future status |
| --- | --- | --- | --- | --- | --- | --- |
| `apply_preview_organization` | `src-tauri/src/commands/mod.rs` | Calls `move_engine::apply_preview_moves` for legacy organization preview moves. | Not called by current Organize UI. | Existing backend tests cover some move/rollback behavior. | Legacy model is not the new `StagingPlan`; visible safety contract is missing. | Keep unexposed until replaced by ApplyPlan flow. |
| `apply_download_item` | `src-tauri/src/commands/mod.rs` | Applies active Inbox download files through move-engine paths. | Current visible Inbox has `INBOX_FILE_ACTIONS_BLOCKED=true`. | Existing desktop proof verifies no enabled file-changing controls. | Backend command remains registered; future UI must not call it directly. | Future candidate only after ApplyPlan and backup/restore contract. |
| `apply_download_items` | `src-tauri/src/commands/mod.rs` | Batch version of Inbox apply path. | Blocked from visible Inbox workflow. | Existing guard tests cover current visible UI. | Batch partial failure/result log contract is not user-facing enough. | Future candidate only after per-file result logging and confirmation. |
| `apply_guided_download_item` | `src-tauri/src/commands/mod.rs` / `move_engine` | Can install guided/special mod batches, move incoming files, back up replaced files, and write snapshots. | Blocked from visible Inbox workflow. | Backend tests cover guided MCCC fixture paths. | Useful recovery pieces exist, but current product copy and safety contract are not sufficient for broad Apply. | Keep internal until guided Apply has the same safety contract. |
| `apply_special_review_fix` | `src-tauri/src/commands/mod.rs` / `move_engine` | Can repair a special review item by moving/replacing files. | Not exposed by current visible workflow. | Some move-engine fixture coverage exists. | Repair language and replacement scope need a stricter user-facing contract. | Keep blocked. |
| `apply_review_plan_action` mutating branches | `src-tauri/src/commands/mod.rs` | Can call special repair, guided install, trusted download import, or split batch actions. | Current visible Inbox handler returns before calling it. | Guard tests cover current Inbox block. | Mixed action kinds include file/network/state changes and need separate confirmation and result semantics. | Keep blocked; split into explicit future contracts. |
| `undo_applied_item` | `src-tauri/src/commands/mod.rs` | Moves previously applied files back to source-origin paths and updates DB rows. | Not exposed by current visible Inbox. | Existing backend behavior is not current UI proof. | Not enough for general restore: path validation, result logs, and same-run restore scoping need design. | Future restore candidate after ApplyPlan persistence. |
| `restore_snapshot` | `src-tauri/src/commands/mod.rs` / `move_engine` | Rolls back snapshot items and can move backup files. | Not exposed by current Organize UI. | Move-engine rollback tests exist. | Snapshot scope is not a complete visible Apply contract. | Useful recovery building block. |
| `commit_staging_area` / `commit_all_staging_areas` | `src-tauri/src/commands/mod.rs` | Applies ready app-local staging/download items via move-engine paths. | Not called by current Plan Preview or Organize UI. | Command registration tests ensure preview commands are before mutating commands. | Commit wording and behavior are too technical and file-changing. | Keep internal/unexposed; future replacement should be ApplyPlan-based. |
| `cleanup_staging_areas` | `src-tauri/src/commands/mod.rs` / `downloads_watcher` | Deletes app-local staging folders/files under the staging root. | Not exposed by current Plan Preview UI. | Limited backend safety checks. | Cleanup/delete semantics require product decision, confirmation, and recovery. | Not allowed for visible v1 Apply. |
| `reject_download_item` / `reject_download_items` | `src-tauri/src/commands/mod.rs` / `downloads_watcher` | Moves downloaded staging files to `SimSuite_Rejected` and deletes related DB rows. | Hidden/blocked from current visible Inbox. | Existing backend path exists; current UI proof checks no enabled controls. | This is a file-moving workflow and should not be casual review UI. | Keep blocked until separate safety contract. |
| `restore_rejected_item` | `src-tauri/src/commands/mod.rs` / `downloads_watcher` | Moves rejected files back into app-local staging and recreates file rows. | Not part of current visible Inbox workflow. | Backend path exists. | Needs same path validation/result-log rules. | Future recovery candidate only. |
| `move_engine::move_single_file` | `src-tauri/src/core/move_engine/mod.rs` | Uses `fs::rename`, falls back to copy/remove. | Backend helper only. | Covered indirectly by move-engine tests. | Must never be called from preview-only flows. | Keep behind future ApplyPlan executor. |

Database-only or mostly database-state paths also exist: `ignore_download_item`,
`ignore_download_items`, `snooze_download_item`, creator/category audit apply
commands, settings saves, and watch-source saves. These do not move user files
directly, but they still need clear product copy when visible because they can
change app state.

Existing recovery pieces include `snapshot_manager::create_snapshot`,
`snapshot_manager::list_snapshots`, and `move_engine::restore_snapshot`. They are
useful, but not enough by themselves to make Apply safe to expose.

## 2. Current Preview-Only Surfaces

- **Inbox**: visible workflow is intake/review-only and blocks file-changing
  controls through `INBOX_FILE_ACTIONS_BLOCKED=true`.
- **Organize**: generates preview-only `StagingPlan` data through
  `generate_sorting_preview_plan`; it does not call legacy apply/snapshot APIs.
- **Pending Plans**: summarizes app-local pending batch data and points detailed
  batch review back to Inbox.
- **Plan Preview / internal Staging**: direct route remains compatibility-only,
  preview/readiness-only, and points users to Organize and Inbox.
- **Library safe action preflight**: review-first surface; it must not imply safe
  delete or safe replace.
- **Duplicates**: comparison-only; deterministic duplicate proof does not become
  cleanup permission.
- **Updates**: review/check workflow only; no automatic replacement.

## 3. Apply Safety Contract Definition

Before SimSuite can apply a plan, it must show exactly what will change, require
explicit confirmation, create or verify a backup/restore path, validate all
source and destination paths, handle conflicts, record per-file results, and
fail safely.

`StagingPlan` remains preview-only. A future `ApplyPlan` must be separate and
must block review-only, heuristic-only, conflicted, unsupported, and invalid-path
items.

## 4. User Impact Plan

Users will not get an Apply button yet. This sprint defines the safety rules so
that when Apply is built later, it can protect their files and clearly explain
every change.

### What this means for the user

Nothing changes files. SimSuite can still generate and show preview plans, but
future Apply work now has a written checklist before it can touch a real Mods,
Tray, Downloads, or app-local intake file.

### Trust / safety boundary

No Apply workflow was implemented. No files were moved, deleted, cleaned up,
quarantined, replaced, auto-sorted, or changed by AI. Existing backend mutating
commands remain internal and unexposed by the current visible Inbox, Organize,
and Plan Preview workflows.

## What Changed

- Added `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`.
- Added this sprint report.
- Updated trust/navigation/backend/status docs with current-state notes.
- No Rust, schema, API, or visible route behavior changes were made.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `104` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

## Desktop / Runtime Proof

Skipped. This sprint changed docs and a trust-boundary test only; it did not
change visible route behavior, backend commands, schemas, or runtime UI.

## What Could Not Be Verified Yet

- No future ApplyPlan persistence or migration exists yet.
- Existing recovery primitives were audited, not redesigned or executed.
- No real file-changing path was run.

## Linear Updates

Completed after validation:

- Commented on `VEL-18` with backup/restore contract coverage (`2257955b-7cb6-4303-8930-0cd6e41a4c29`).
- Commented on `VEL-19` with dry-run/apply split coverage (`513f3dd0-f63b-4f4f-8a92-4ea4889b4e93`).
- Left `VEL-20` open and blocked.

## Recommended Next Sprint

ApplyPlan persistence audit or ApplyPlan builder design. Do not build real Apply
or move files yet.

## Unrelated Worktree Changes

Known unrelated dirty files must remain unstaged:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- unrelated hunks in `src/styles/globals.css`
- unrelated pre-existing hunks in `SESSION_HANDOFF.md`
- unrelated pre-existing hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Commit hash is recorded in the final Codex report after commit.

## Final Honest Verdict

Verified: Apply Safety Contract Design v1 is complete and ready for the next
planning/implementation step.
