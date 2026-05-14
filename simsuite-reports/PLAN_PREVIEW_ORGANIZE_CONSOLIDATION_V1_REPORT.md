# Plan Preview / Organize Consolidation v1 Report

Date: 2026-05-14

Branch: `codex/plan-preview-organize-consolidation-v1`

## Pre-Implementation Audit

### 1. Current Route / Navigation State

Worktree at start:

- Starting branch: `codex/rename-staging-plan-preview-v1`.
- Created sprint branch: `codex/plan-preview-organize-consolidation-v1`.
- Known unrelated dirty files were already present: `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, plus existing `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` hunks.

Route and navigation findings:

| Area | Finding | Decision |
| --- | --- | --- |
| Sidebar | `Plan Preview` was visible as the internal `staging` route in Seasoned and Creator modes. | Hide it from normal top-level navigation. |
| Direct route | `#staging` loaded the safe Plan Preview route. | Keep it routable for compatibility and explain that the workflow now lives in Organize. |
| Organize | Existing route generated preview-only organization plans through `generate_sorting_preview_plan`. | Keep this as the main planning workspace. |
| Plan Preview data | Direct route used `get_staging_areas` and `get_staging_preview_plan`, both read-only. | Reuse this data inside Organize as `Pending plans`. |
| Safety controls | Visible Plan Preview and Organize routes did not expose enabled Apply, move, cleanup, delete, quarantine, or commit controls. | Preserve this boundary. |

### 2. Consolidation Decision

- Organize remains the top-level planning workspace.
- Plan Preview becomes a section/tab inside Organize under `Pending plans`.
- The direct internal `staging` route remains safe and routable for now.
- The sidebar no longer needs to show Plan Preview as a top-level item.
- No file-changing workflow is added.

### 3. User Impact Plan

Users will go to Organize to create and review plans. Plan Preview will be inside Organize instead of feeling like a separate page. Nothing changes files.

## What Was Audited

- Existing Organize route, generated-plan UI, and tests.
- Direct Plan Preview route, route copy, and tests.
- Sidebar/mode visibility model.
- Existing read-only Plan Preview APIs and mock data.
- Desktop proof route checks and screenshots.
- Current navigation, trust, backend map, status, and handoff docs.
- Linear issue `VEL-11`.

## Consolidation Decision

Plan Preview is now folded into Organize as a visible `Pending plans` tab. The internal direct route remains available for compatibility, but it points users back to Organize and stays preview-only.

## What Changed

- Added a reusable read-only `PendingPlansPreview` component for pending plan data.
- Updated `OrganizeScreen` to use a compact two-tab planning workspace:
  - `Create plan`
  - `Pending plans`
- Reused `get_staging_areas` and `get_staging_preview_plan` in the Organize `Pending plans` tab.
- Removed the top-level Plan Preview exposure from Seasoned and Creator sidebar profiles.
- Kept the direct Plan Preview route safe and added `Open Organize` copy/action.
- Updated desktop proof to check the sidebar, Organize tabs, generated plan output, pending plans, and the direct route.
- Added focused tests for navigation visibility, pending plans, direct route safety, and trust-boundary copy.

## What This Means For The User

Users now have one clearer place to plan organization work. They open Organize, create preview plans, and review pending plans there. The separate Plan Preview sidebar item is gone from the normal navigation. No files are changed by this work.

## Trust / Safety Boundary

No unsafe automation was added. SimSuite still does not apply, move, delete, clean up, quarantine, replace, or auto-sort files from this workflow. Plan Preview and Organize remain preview-only. The internal mutating staging commands still exist in the backend, but they are not exposed or called by the visible route.

## Organize Behavior

Organize now has a two-tab planning workspace:

- `Create plan`: keeps the existing generated preview plan workflow.
- `Pending plans`: shows the old Plan Preview data inside Organize, including pending plan folders, plan summary, caveats, item counts, empty/loading/error states, and `No files changed` copy.

## Direct Plan Preview Route Behavior

The direct internal `#staging` route still works. It explains that the main Plan Preview workflow now lives in Organize, includes an `Open Organize` action, and still shows preview-only pending plan data.

## Sidebar Behavior

Plan Preview is hidden from Casual, Seasoned, and Creator top-level navigation. Internal route labels remain available for compatibility and direct route rendering.

## Files Changed

- `src/screens/OrganizeScreen.tsx`: adds tabbed Create Plan / Pending Plans workspace while preserving the generated preview workflow.
- `src/screens/organize/PendingPlansPreview.tsx`: new reusable read-only pending-plan section.
- `src/screens/StagingScreen.tsx`: direct route now points users toward Organize while staying preview-only.
- `src/lib/experienceMode.ts` and `src/lib/experienceMode.test.ts`: remove Plan Preview from normal mode navigation and test that it stays hidden.
- `src/screens/OrganizeScreen.test.tsx` and `src/screens/StagingScreen.test.tsx`: cover the consolidated UI and direct route safety.
- `src/styles/globals.css`: adds focused tab/pending-plan layout styles and path wrapping for the reused preview rows.
- `scripts/desktop/desktop-library-proof.mjs`: updates runtime proof for the consolidated Organize flow.
- Current-state docs and this report.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit -- --run src/screens/OrganizeScreen.test.tsx src/screens/StagingScreen.test.tsx src/lib/experienceMode.test.ts src/trustBoundaryCopy.test.ts`: passed (`4` files, `11` tests).
- `npm run test:unit`: passed (`27` files, `99` tests).
- `npm run build`: passed. The existing Vite chunk-size warning remains.
- Separate Rust validation was not run because no Rust files were changed. Desktop proof/smoke still built the release Tauri app and showed existing Rust warning noise only.

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
- Proof output: `output/desktop/library-proof/2026-05-14T12-03-27-658Z`.
- Captured screenshot: `output/desktop/library-proof/2026-05-14T12-03-27-658Z/organize-plan-preview-consolidated-v1.png`.
- Direct route screenshot: `output/desktop/library-proof/2026-05-14T12-03-27-658Z/plan-preview-direct-route-safe-v1.png`.
- Proof verified sidebar no longer shows Plan Preview as a normal top-level item, Organize opens, generated preview still works, `Pending plans` loads read-only preview data, `No files changed` appears, and no enabled file-changing controls appear.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## What Could Not Be Verified

- Real-user libraries were not scanned.
- No Apply/file movement behavior was verified because it intentionally was not built.
- Linear status movement was not performed; a comment was added instead so broader consolidation workflow state can remain under project control.

## Linear Updates

Linear issue `VEL-11` was inspected and updated with a comment summarizing the v1 consolidation and validation results.

## Recommended Next Sprint

Apply Safety Contract design. This should define preview, confirmation, backup/restore, path validation, destination conflict handling, recoverable errors, and per-file result logs before any real Apply or file movement work starts.

## Docs Updated

- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/PLAN_PREVIEW_ORGANIZE_CONSOLIDATION_V1_REPORT.md`

## Unrelated Worktree Changes

- Left unrelated `.cocoindex_code/*` generated files alone.
- Left unrelated `src/screens/HomeScreen.tsx` changes alone.
- Preserved pre-existing unrelated `src/styles/globals.css` Home hunks and will stage only this sprint's CSS hunks.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` had pre-existing unrelated hunks; only this sprint's top-section notes should be staged.

## Commit

- `355fc84` - `Fold Plan Preview into Organize`
- Follow-up docs record commit is reported in the final Codex response.

## Final Honest Verdict

Verified: Plan Preview / Organize consolidation v1 is working for the tested paths.
