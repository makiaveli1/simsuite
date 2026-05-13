# Staging Preview Plan Foundation v1 Report

Date: 2026-05-13

Branch: `codex/staging-preview-plan-foundation-v1`

## Pre-Implementation Audit

### 1. Current Staging data map

- `get_staging_areas` reads app-local folders under `downloads_inbox`.
- The returned data is folder-level: `itemId`, staged subdirectory path/name, file count, total bytes, and creation time.
- Staged folders come from the Downloads watcher archive/intake staging area.
- Current Staging data does not provide per-file Library IDs.
- Current Staging data does not provide per-file current paths beyond staged folder paths.
- Current Staging data does not provide suggested destination paths.
- Current Staging data can support a folder-level preview/review plan today.
- Current Staging data cannot honestly support per-file move suggestions yet.

### 2. Existing file-action risk map

| Path | Classification | Notes |
| --- | --- | --- |
| `get_staging_areas` | read-only | Lists app-local staged folders and counts. |
| `get_staging_preview_plan` | read-only | New v1 command; returns preview-only plan data and `wouldTouchFiles=false`. |
| `cleanup_staging_areas` | cleanup / mutating | Deletes app-local staged folders under `downloads_inbox`; remains unexposed by current Staging UI. |
| `commit_staging_area` | apply / mutating | Can call move-engine apply paths for a ReadyNow standard download item; remains unexposed by current Staging UI. |
| `commit_all_staging_areas` | apply / mutating | Iterates staged areas and can call commit behavior; remains unexposed by current Staging UI. |
| Downloads reject/restore paths | moves/copies files | Existing Downloads workflow, not exposed through Staging Preview Plan v1. |
| Organize apply paths | moves files | Existing apply-style backend surface, not touched by this sprint. |

### 3. StagingPlan contract decision

`StagingPlan` v1 is preview-only and uses the shared Rust/TypeScript shape:

- `id`
- `createdAt`
- `source`
- `status`
- `title`
- `summary`
- `itemCount`
- `wouldTouchFiles`
- `caveats`
- `items`

`StagingPlanItem` v1 includes:

- `id`
- `fileId`
- `fileName`
- `currentPath`
- `suggestedDestinationPath`
- `actionKind`
- `evidenceLevel`
- `reason`
- `caveats`
- `wouldTouchFiles`

Safety decisions:

- `wouldTouchFiles` is always `false`.
- v1 returns `blocked` when no staged content exists.
- v1 returns `preview_only` for current folder-level staged data.
- `ready_for_review` exists in the type for future use but is not used for v1 folder-level data.
- v1 does not fake per-file suggestions. Folder-level staged subdirectories become `suggest_review` items with `fileId=null` and `suggestedDestinationPath=null`.

### 4. User experience impact plan

Users will see Staging as a preview area for plans, not a place that changes files. The app can start showing plan items with reasons and caveats, but nothing is moved or deleted.

## What Was Audited

- Staging route and tests.
- Staging backend commands.
- Downloads watcher staging data.
- Tauri command registration.
- TypeScript API and mock fallback.
- Trust-boundary docs.
- Navigation workflow docs.
- Linear issues `VEL-12`, `VEL-13`, and `VEL-14`.

## Current Staging Data Map

Staging can currently show app-local staged folders and their counts. It cannot yet provide per-file organization suggestions, file IDs, or destination paths.

## File-Action Risk Map

The new preview command is read-only. Existing mutating Staging commands still exist in the backend, but the visible Staging screen does not call or expose them.

## StagingPlan Contract

The contract is preview-only. It can represent future suggested organization plans, but v1 only uses honest folder-level review items from existing staging data.

## What Changed

- Added Rust and TypeScript `StagingPlan` / `StagingPlanItem` types.
- Added read-only backend command `get_staging_preview_plan`.
- Added API and mock support.
- Added a minimal Staging preview plan panel.
- Added focused backend/frontend/trust tests.
- Updated trust, navigation, backend map, handoff, and status docs.

## What This Means For The User

Staging can now show a preview plan area with reasons and caveats. It still does not move, delete, clean up, replace, or auto-sort files.

## Trust / Safety Boundary

No files are moved, deleted, disabled, quarantined, replaced, or auto-sorted. The new plan is preview-only. Auto Sorting is still future work. AI is not deciding file actions. Mutating Staging commands remain unexposed by the current visible Staging route.

## Backend Command Behavior

`get_staging_preview_plan` reads existing staging folders and returns a preview-only plan. It does not call commit, cleanup, move-engine apply, shell, delete, or write paths.

## Frontend / API Behavior

The Staging screen now requests the preview plan and renders plan summary, caveats, review-only items, and blocked/empty states. It keeps the existing staged area summary and disabled readiness controls.

## Linear Updates

Linear issues inspected:

- `VEL-12` - Define StagingPlan model.
- `VEL-13` - Add preview-only staging plan command.
- `VEL-14` - Staging plan UI.

Final Linear comments were added after validation. The issues were left open because the available workflow does not need to close them from this sprint, and future UI/model follow-up may still build on the same workstream.

## Files Changed

- `src-tauri/src/models.rs` - added shared preview-only StagingPlan Rust models.
- `src-tauri/src/core/downloads_watcher/mod.rs` - added read-only plan builder and backend tests.
- `src-tauri/src/commands/mod.rs` - added `get_staging_preview_plan` and read-only guard test.
- `src-tauri/src/lib.rs` - registered the new Tauri command and updated registration drift coverage.
- `src/lib/types.ts` - added TypeScript StagingPlan contract.
- `src/lib/api.ts` - added API and mock support.
- `src/screens/StagingScreen.tsx` - added a minimal preview plan panel without file-changing controls.
- `src/screens/StagingScreen.test.tsx` - covered preview, blocked, caveat, and no-mutating-call behavior.
- `scripts/desktop/desktop-library-proof.mjs` - added Staging preview proof checks.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md` - documented StagingPlan v1 boundary.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md` - added M2 current-state note.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md` - recorded the read-only Staging preview command.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` - added current sprint notes.

## Tests

- `cargo fmt`: passed.
- `cargo test staging_preview -- --nocapture`: passed (`3` focused backend tests).
- `npm run test:unit -- StagingScreen trustBoundaryCopy`: passed (`2` files / `4` focused tests).
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`23` files / `90` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `cargo check`: passed with existing Rust warning noise.
- `cargo test`: passed (`255` passed / `2` ignored).
- `cargo build --release`: passed with existing Rust warning noise.
- `npm run test:rust`: passed (`255` passed / `2` ignored).

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
  - output folder: `output/desktop/library-proof/2026-05-13T14-07-26-741Z`
  - Staging screenshot: `output/desktop/library-proof/2026-05-13T14-07-26-741Z/08-staging-preview-plan.png`
  - proof confirmed Staging loaded, preview plan copy appeared, “No files changed” appeared, and no enabled file-changing Staging buttons were found.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## What Could Not Be Verified

- StagingPlan v1 does not verify per-file organization suggestions because current staging data is folder-level.
- Future apply/move behavior was intentionally not verified because no apply, move, cleanup, delete, quarantine, replacement, or auto-sort workflow was added.
- Linear issues were commented but not closed.

## Recommended Next Sprint

Auto Sorting rules audit, if this sprint lands green. That should still produce preview-only suggested plans and no file movement.

## Docs Updated

- `simsuite-reports/STAGING_PREVIEW_PLAN_FOUNDATION_V1_REPORT.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated Worktree Changes

Known unrelated dirty files remain outside this sprint unless explicitly staged as sprint-specific hunks:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated status/handoff hunks

## Commit

- `a0a72f7` - Add preview-only Staging plan command

## Final Honest Verdict

Verified: Staging Preview Plan Foundation v1 is working for the tested paths.
