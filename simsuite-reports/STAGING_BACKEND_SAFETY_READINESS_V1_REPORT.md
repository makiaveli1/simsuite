# Staging Backend Safety Readiness v1 Report

Date: 2026-05-13

Branch: `codex/staging-backend-safety-readiness-v1`

## Pre-Implementation Audit

### Worktree State

Initial worktree check showed pre-existing unrelated changes:

- `.cocoindex_code/cocoindex.db/mdb/data.mdb`
- `.cocoindex_code/target_sqlite.db`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

This sprint must leave unrelated Home, global CSS, generated CocoIndex, and older status/handoff hunks alone.

### 1. Current Staging Inventory

| Area | Current status | Notes |
| --- | --- | --- |
| Frontend route | real and wired | `StagingScreen` is lazy-loaded from `App.tsx`. |
| Navigation | real and wired | Staging is visible in Seasoned and Creator modes through `experienceMode.ts`. |
| Backend read command | real and wired | `get_staging_areas` reads app-local `downloads_inbox` staging folders. |
| Backend cleanup command | real and wired | `cleanup_staging_areas` deletes selected paths under the app-local staging root. |
| Backend commit commands | real and wired | `commit_staging_area` and `commit_all_staging_areas` call move-engine apply paths for ReadyNow standard download items. |
| Database tables | partial | Staging itself is folder-based under app data; committed download items are tracked through Downloads/Library tables. |
| Preview plan model | partial | Downloads/Organize have preview/apply models; Staging route mostly lists staged folders and exposes commit/reject actions. |
| Apply/move behavior | real and wired | Staging commit can apply preview moves for active download file IDs. |
| Delete/cleanup behavior | real and wired | Staging reject deletes app-local staged extracted folders. |
| Tests/proof | partial | Existing Staging test expected cleanup API to be called after confirmation. Desktop proof/smoke cover broader runtime lanes, not Staging readiness specifically. |

Classification: Staging is real and visible for some modes, but it currently exposes file-changing actions. It is not ready to be the safety bridge for future Auto Sorting until the visible route is made preview-only or guarded.

### 2. File-Action Inventory

| Operation | Location | Classification | Notes |
| --- | --- | --- | --- |
| List staging areas | `get_staging_areas`, `list_staging_areas` | read-only | Reads app-local staging folders and counts files/bytes. |
| Commit one staging area | `commit_staging_area` | moves/copies/renames real files through move engine | Requires ReadyNow standard download item and calls `apply_preview_moves_for_files`. |
| Commit all staging areas | `commit_all_staging_areas` | moves/copies/renames real files through move engine | Iterates staged areas and applies commit sync. |
| Reject staged folder | `cleanup_staging_areas` | deletes app-local staged extracted files | Restricted to app-local staging root, but still file-changing. |
| Downloads reject/restore | Downloads watcher helpers | moves app-managed download items | Existing Downloads workflow, not a new Staging feature. |
| Move engine preview/apply | `move_engine` | preview or file-changing depending on command | Existing backend has snapshot/rollback concepts, but Staging UI must not expose apply before readiness is complete. |
| Open folder/reveal helpers | command helpers | opens Explorer only | Non-mutating when used only to reveal real paths. |

### 3. Safety Gap Analysis

| Requirement | Current Staging status |
| --- | --- |
| Evidence model | Partial; Staging shows folder/file counts, not per-file suggested action evidence. |
| Preview | Partial; Staging lists staged folders but does not present a structured per-file plan. |
| User confirmation | Partial; reject had confirmation, commit actions did not show the full future safety requirements. |
| Backup/restore | Partial in move engine, not clearly surfaced or enforced as a Staging readiness contract. |
| Undo/recovery | Partial in move engine/Downloads, not a Staging route guarantee. |
| Conflict handling | Not clearly surfaced in Staging. |
| Path safety | Backend cleanup restricts to staging root; future move/apply still needs explicit path validation contract. |
| Source/destination validation | Existing apply path may validate through move engine, but Staging does not explain it as a preview plan. |
| Same-drive/cross-drive behavior | Future work for Staging readiness. |
| Permission failure handling | Backend can return errors, but route currently presents limited readiness context. |
| Duplicate destination handling | Future structured plan requirement. |
| Logs/audit trail | Future per-file result log requirement. |
| Tests | Current test covered cleanup confirmation, not preview-only safety. |
| Desktop proof | No Staging-specific proof step yet. |

### 4. User Experience Impact Plan

This sprint should make Staging honest and safe to expose. Users may still open Staging to inspect app-local staged folders, but they should not see enabled commit, reject, delete, quarantine, or apply controls from this screen. The page should say it is preview/readiness only until SimSuite has the full safety pieces needed for real file-changing workflows: preview, confirmation, backup/restore, recoverable errors, and proof.

No user files should be moved, deleted, disabled, quarantined, or replaced by this sprint.

## What Was Audited

- Staging frontend route: `src/screens/StagingScreen.tsx`.
- Staging route registration: `src/App.tsx`.
- Staging navigation exposure: `src/components/layout/Sidebar.tsx` and `src/lib/experienceMode.ts`.
- Staging API wrappers and types: `src/lib/api.ts`, `src/lib/types.ts`.
- Staging test coverage: `src/screens/StagingScreen.test.tsx`.
- Trust-sensitive copy guard: `src/trustBoundaryCopy.test.ts`.
- Field Guide Staging copy: `src/components/FieldGuide.tsx`.
- Backend Staging commands: `src-tauri/src/commands/mod.rs`.
- Downloads staging folder helpers: `src-tauri/src/core/downloads_watcher/mod.rs`.
- Existing organization preview and move paths: `src-tauri/src/core/rule_engine/mod.rs`, `src-tauri/src/core/move_engine/mod.rs`.
- Trust and backend docs: `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`, `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`.

## Current Staging Status

Staging is real and visible in Seasoned and Creator modes. Before this sprint it loaded app-local staged folder data and exposed commit/reject controls. Those controls were wired to backend commands that can change files.

After this sprint, the visible Staging route is preview/readiness only. It still reads staged folder metadata, file counts, and byte totals, but it does not expose enabled file-changing controls and does not call commit, commit-all, or cleanup APIs.

## File-Changing Action Inventory

| Operation | Current exposure | Effect |
| --- | --- | --- |
| `get_staging_areas` | still used by Staging | Read-only listing of app-local staged folders. |
| `cleanup_staging_areas` | backend command remains, no longer called by current Staging route | Deletes app-local staged extracted folders under `downloads_inbox`. |
| `commit_staging_area` | backend command remains, no longer called by current Staging route | Can apply move-engine paths for one ReadyNow standard download item. |
| `commit_all_staging_areas` | backend command remains, no longer called by current Staging route | Can apply move-engine paths for multiple staging areas. |
| `apply_preview_organization` and related move-engine commands | existing non-Staging flows | File-changing when approved; future Staging exposure needs a full safety contract. |
| Downloads reject/restore helpers | existing Downloads workflow | Moves app-managed staged/rejected content; not changed in this sprint. |

## Safety Gaps

Staging is not ready for real file movement yet because it still needs:

- a structured per-file preview plan,
- evidence and caveats for each suggested action,
- explicit user confirmation at the point of action,
- backup or restore support surfaced as part of the workflow,
- source and destination path validation,
- duplicate destination handling,
- permission and partial-failure recovery,
- per-file result logs,
- focused backend/frontend tests,
- desktop proof for the eventual file-changing path.

## What Changed

- Reworked `StagingScreen` so it only reads staged folder metadata and shows preview/readiness copy.
- Removed visible route calls to `api.commitStagingArea`, `api.commitAllStagingAreas`, and `api.cleanupStagingAreas`.
- Replaced the old Staging reject/cleanup test with a preview-only guard test.
- Added Staging to the trust-boundary copy guard and added a static guard against reintroducing commit/reject API calls in the current Staging surface.
- Updated Field Guide copy so Staging is described as a preview/readiness checkpoint, not a batch action surface.
- Added Staging-specific file-change readiness rules to the trust-boundary doc.
- Updated the backend systems map with current Staging command boundaries.

## What this means for the user

Staging is clearer and safer right now. Users can inspect staged folders and see counts, but this screen will not move, delete, reject, quarantine, replace, or auto-sort files. It now says plainly that Staging is preview-only until SimSuite has the safety pieces needed for real changes.

## Trust / safety boundary

No files are moved, deleted, disabled, quarantined, replaced, or auto-sorted by this sprint. Staging is preview/readiness work only. Auto Sorting remains future work. Future file-changing workflows need preview, confirmation, backup/restore, recoverable errors, and proof before they can be exposed. AI is not deciding file actions.

## Auto Sorting Readiness

Auto Sorting can start next only as a suggested plan workflow. It should produce a preview-only plan with evidence, caveats, and no file movement. Real apply actions must wait for the Level 4 safety contract: confirmation, backup/restore, path validation, recoverable errors, per-file result logs, and desktop proof.

## Backend / Data Notes

- No schema or migration was added.
- No Tauri command contract changed.
- Mutating backend Staging commands still exist for future workflows, but the current Staging route no longer exposes them.
- The read-only `get_staging_areas` command remains the only Staging command used by the current route.

## Files Changed

- `src/screens/StagingScreen.tsx`: made current Staging preview-only and removed route calls to mutating Staging APIs.
- `src/screens/StagingScreen.test.tsx`: replaced cleanup confirmation test with preview-only route guard.
- `src/trustBoundaryCopy.test.ts`: added Staging to trust-sensitive copy coverage and added a static Staging API guard.
- `src/components/FieldGuide.tsx`: updated Staging guidance to preview/readiness wording.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: added Staging-specific readiness rules.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: documented Staging command boundaries and current UI exposure.
- `SESSION_HANDOFF.md`: added sprint handoff notes.
- `docs/IMPLEMENTATION_STATUS.md`: added sprint status notes.
- `simsuite-reports/STAGING_BACKEND_SAFETY_READINESS_V1_REPORT.md`: this report.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit -- StagingScreen trustBoundaryCopy`: passed (`2` files, `3` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- `npm run test:unit`: passed (`23` files, `89` tests).

Rust-specific validation was not run separately because this sprint did not modify Rust code. The desktop proof/smoke commands did rebuild the Rust release app and showed the existing unused-code warnings.

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
- Latest proof folder: `output/desktop/library-proof/2026-05-13T00-17-48-883Z`.
- `npm run desktop:smoke:fixtures`: passed against the release app.
- Existing Rust unused-code warnings remain unchanged.
- Existing Tauri callback reload warnings still appear in browser log inspection; no severe runtime errors were reported by the proof summary.

## What Could Not Be Verified

- No real file-changing Staging workflow was verified because this sprint intentionally does not expose one.
- Backend mutating Staging commands were audited but not redesigned.
- No real user Mods, Tray, Downloads, or staged files were used.
- The current desktop proof does not include a dedicated Staging screenshot step; Staging route behavior is covered by unit/static tests and broader desktop proof/smoke.

## Recommended Next Sprint

Auto Sorting Suggested Plan v1, preview-only and no file movement. That sprint should build a structured suggested plan with evidence and caveats, but still avoid applying moves until Staging has the Level 4 safety contract.

## Docs Updated

- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/STAGING_BACKEND_SAFETY_READINESS_V1_REPORT.md`

## Unrelated Worktree Changes

Pre-existing unrelated changes were left alone:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing status/handoff hunks outside this sprint's top notes

## Commit

- `06423c8` - Audit Staging safety and readiness

## Final Honest Verdict

Verified: Staging backend safety/readiness v1 is working for the tested paths.

This does not mean Staging is ready for real file moves. It means the currently exposed route is preview/readiness only, and the missing safety work is documented before future Auto Sorting or file-changing workflows.
