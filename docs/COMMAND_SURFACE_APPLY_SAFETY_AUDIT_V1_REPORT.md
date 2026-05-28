# Command Surface Apply Safety Audit V1 Report

Date: 2026-05-26
Branch: `codex/organize-dry-run-preview-ui-v1`
Scope: Tauri command registration, frontend API exposure, and file-changing safety gates before any visible Organize Apply or Restore work.

## Executive summary

SimSuite currently keeps the user-facing Organize path in a read-only preview/dry-run state, but the Tauri backend still registers legacy file-changing commands. That means the current UI copy and hidden buttons are useful product guardrails, not sufficient backend enforcement.

**Core finding:** UI gating is not a backend safety boundary. Every file-changing or result-forging command must be blocked, narrowed, or converted to an executor-only path before visible Apply/Restore can ship.

The audit found command registration parity is healthy: the app has a large command surface, but the registered commands and command definitions line up. The risk is not missing registration; the risk is that high-impact commands are still callable from the webview invoke layer if frontend code reaches them.

## Risk classes

| Class | Meaning | Current disposition |
| --- | --- | --- |
| Read-only | Query/list/preview/status commands | Generally acceptable, with normal input validation. |
| DB metadata mutation | Changes labels, settings, audit decisions, watch sources, or plan records | Acceptable only with validation, undo/history where relevant, and clear UI confirmation for bulk changes. |
| Config/path mutation | Changes roots or behavior that later drives file operations | Needs canonical path validation and overlap checks. |
| Filesystem write/move/delete | Moves, copies, restores, rejects, cleans up, or extracts files | Must stay disabled or executor-only until the Apply Safety Contract exists. |
| Network/browser/process side effect | Opens paths/URLs or checks sources | Needs allowlisting, rate limits, and privacy-aware UX. |
| Result/restore log mutation | Writes Apply/Restore history | Must be backend-derived, not client-forged. |

## Critical public backend risks

These commands are especially sensitive because they can move, copy, reject, restore, replace, clean up, or otherwise mutate user/app files. Some have existing `approved` booleans, rollback behavior, or current UI blocking, but those are not enough for the future Apply contract.

| Command | Operation | Existing posture | Required disposition before visible Apply/Restore |
| --- | --- | --- | --- |
| `apply_preview_organization` | Moves existing library files from a generated preview. | Legacy apply path; current Organize UI should not call it. | Disable/remove from public UI flow or wrap behind ApplyPlan executor only. |
| `commit_staging_area` | Moves one staged download area into the library. | Legacy staging commit; current Staging UI is preview-only. | Executor-only; require plan/run id, validation, backup, and confirmation token. |
| `commit_all_staging_areas` | Batch commits staged download areas. | Legacy batch commit. | Executor-only plus batch limits and per-item result isolation. |
| `apply_download_item` | Moves one download item into library. | Downloads visible file actions are blocked. | Executor-only; revalidate item state and filesystem roots at execution. |
| `apply_download_items` | Batch applies download items. | Downloads visible file actions are blocked. | Executor-only; batch limits, result isolation, rollback/restore map. |
| `apply_guided_download_item` | Guided install; can move/replace installed files. | Has preflight/rollback concepts and `approved`, but still public. | Require immutable plan hash, backend-issued confirmation token, backup/restore entries, and result log. |
| `apply_special_review_fix` | Special repair path; can move/replace files. | Public command with mutating behavior. | Same gate as guided apply. |
| `apply_review_plan_action` | Mixed action command: open URL, download trusted files, install dependency, split files, repair. | Too much risk variety inside one command. | Split by risk class; all mutating/download branches require confirmation token and allowlisted source policy. |
| `restore_snapshot` | Moves files back from a previous snapshot. | Has `approved` and some conflict/hash checks. | Require restore preview, snapshot provenance, restore confirmation token, and per-file restore result log. |
| `undo_applied_item` | Moves applied files back to origin. | No command-level confirmation token. | Add restore preview + confirmation token; canonicalize source/destination at execution. |
| `reject_download_item` | Moves staged files to rejected area and mutates DB rows. | Downloads visible file actions are blocked. | Confirm, canonicalize staging/reject roots, avoid DB deletion before successful move, result log. |
| `reject_download_items` | Batch reject. | Downloads visible file actions are blocked. | Same as reject plus batch limit and partial-failure reporting. |
| `restore_rejected_item` | Moves rejected files back into staging. | Lower risk than user-library Apply, but still mutating. | Canonical root checks, confirmation, and rescan before marking ready. |
| `cleanup_staging_areas` | Deletes app-data staging folders. | Public command accepting path-like inputs. | Stop accepting raw paths; accept staging IDs only, canonicalize target, reject symlink/out-of-root escapes. |

## Sensitive metadata and config commands

These do not directly move user files in the current flow, but they can change future routing, tracking, or history. They should not be treated as harmless.

| Command | Risk | Required gate |
| --- | --- | --- |
| `save_library_paths` | Root paths drive scans and future file movement. | Canonicalize, require existing dirs, block overlapping roots, block drive/home roots unless explicitly confirmed. |
| `save_app_behavior_settings` | Can enable background behavior, ignore patterns, and watch cadence. | Validate intervals/patterns; show confirmation for background/automatic behavior. |
| `save_creator_learning` | Can alter preferred routing. | Preferred paths must be under allowed roots; confirm locked/bulk choices. |
| `apply_creator_audit` | Batch metadata mutation. | Preview diff, confirmation, audit/event log. |
| `save_category_override` | Alters category/routing metadata. | Keep taxonomy validation; add event log/undo where useful. |
| `apply_category_audit` | Batch metadata mutation. | Preview diff, confirmation, audit/event log. |
| `save_watch_source_for_file` / `save_watch_sources_for_files` | Stores external URLs and future update checks. | HTTPS/no-credential validation, trusted-host labels, batch limits. |
| `refresh_watch_source_for_file` / `refresh_watched_sources` | External network checks and DB metadata updates. | Rate limits, timeouts, privacy copy, source allowlist where applicable. |
| `reveal_file_in_folder` | Opens OS file manager for a supplied path. | Restrict to known records or allowlisted roots; avoid arbitrary path opening. |

## ApplyPlan-specific command disposition

The newer ApplyPlan path is correctly preview-first. These commands are safe only if they remain preview/result-foundation helpers and do not become client-controlled execution switches.

| Command | Current role | Disposition |
| --- | --- | --- |
| `build_apply_plan_from_staging_plan` | Backend-owned draft plan creation from staging preview. | Keep preview-only; bind to current scan/settings/provenance hashes before future execution. |
| `save_apply_plan_preview` | Persists preview-only ApplyPlan records. | Keep preview-only; reject any client claim that makes a plan executable. |
| `preview_apply_plan_validation` | Read-only validation preview. | Keep read-only; execution must re-run equivalent checks. |
| `preview_apply_plan_dry_run` | Read-only dry-run classification. | Keep read-only; never create result logs or restore entries. |
| `delete_draft_apply_plan` | Cancels draft/preview/blocked plans. | Low risk; confirm only if plan has runs/results later. |
| `create_apply_plan_run_log` | Creates run log metadata. | Executor-only; frontend should not forge run state. |
| `record_apply_plan_result_log` | Records item result metadata. | Executor-only; derive from actual backend operation transaction. |
| `record_apply_plan_restore_entry` | Records restore mapping metadata. | Executor-only; derive from backup/restore map creation. |

## Required Apply Safety Contract

No real user-file Apply or Restore should be exposed until this contract exists at the backend boundary:

1. **Persisted ApplyPlan identity** — executor accepts a plan/run ID, not raw frontend file paths.
2. **Immutable plan hash** — confirmation must bind to the exact operations shown to the user.
3. **Fresh validation** — backend re-runs validation immediately before execution.
4. **Dry-run parity** — executable classification must match the reviewed dry-run output or force re-confirmation.
5. **Backend-issued confirmation token** — UI confirmation produces a short-lived token scoped to plan hash, operation class, and destination roots.
6. **Canonical root checks** — source, destination, backup, and restore targets must be canonicalized at execution time. This audit intentionally calls these `canonical root checks` because they are a backend execution requirement, not copy text.
7. **No symlink escape** — reject symlink/out-of-root paths for source, staging, backup, restore, and cleanup operations.
8. **Backup first** — every mutating operation touching user files must create or verify backup/restore material before moving/replacing.
9. **Per-file result log** — result rows are written by the executor after actual operation attempts, not by frontend calls.
10. **Restore map** — restore entries are derived from backend-observed source/destination/backup paths.
11. **Partial failure isolation** — batch runs record per-item success/failure and do not silently continue into unsafe states.
12. **No delete/quarantine/replace expansion** — first visible Apply should move/copy only the narrowest confirmed operation set.

## Path safety rules

- Prefer record IDs over raw paths in commands.
- Canonicalize all configured roots on save.
- Canonicalize every source and destination again at execution time.
- Reject roots that overlap each other unless the overlap is explicitly supported and documented.
- Reject drive root, home/user root, OneDrive root, and app-data root as library/download roots unless the user explicitly confirms a high-risk choice.
- Reject symlinks where they could escape allowed roots.
- Refuse cleanup/delete operations unless the target is under a canonical app-owned root and identified by DB record ID.
- Treat validation preview as advisory only; the executor must enforce the same or stricter rules.

## Immediate sprint recommendations

1. Keep Organize Apply and Restore invisible/disabled.
2. Add backend tests before any command is moved from preview-only to mutating.
3. Convert legacy direct apply/commit/restore commands to one of:
   - removed from current frontend wrappers,
   - feature-flagged off,
   - or explicitly documented as legacy and not used by Organize.
4. Build the next UI sprint as **Confirmation Design V1** only: show exact operations, blockers, backup expectations, restore expectations, and a disabled/fake final confirmation state.
5. Only after that, build a fixture-only executor path that cannot touch user files.

## Release-hardening notes

Before any external release, also audit:

- `tauri.conf.json` security posture, especially development conveniences such as devtools and CSP.
- Tauri capabilities and whether command exposure can be narrowed.
- Large command modules that mix read-only, metadata, filesystem, network, and process side effects.
- README claims that imply Apply/Restore are user-ready before they really are.

## Backend Command Gating V1 update

As of the current branch state, the audit's immediate backend-gating recommendation has been implemented at the Tauri command boundary.

Current disposition:

- safe preview/review commands remain callable;
- preview/draft DB writes remain callable for saved ApplyPlan review;
- `create_apply_plan_run_log`, `record_apply_plan_result_log`, and `record_apply_plan_restore_entry` fail closed as future executor-only commands;
- legacy file-changing commands such as staging commit/cleanup, legacy Organize apply, Downloads apply/reject/restore, special repair/install, and snapshot restore fail closed before doing work.

This still does not enable Apply, Restore, backup execution, result-log writes, restore-entry writes, file movement, file copying, folder creation, deletion, cleanup, quarantine, replacement, or AI decisions. The gate only prevents current webview invocation from bypassing preview-only product flows.

## Sprint status

This report is documentation plus Backend Command Gating V1 guardrail work. It does not enable Apply, Restore, backup execution, file movement, file copying, folder creation, deletion, cleanup, quarantine, replacement, or AI decisions.
