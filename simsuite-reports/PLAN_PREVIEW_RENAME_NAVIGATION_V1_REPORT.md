# Plan Preview Rename / Navigation Wording v1 Report

Date: 2026-05-14

Branch: `codex/rename-staging-plan-preview-v1`

## Pre-Implementation Audit

### 1. Current Staging Wording Map

Worktree at start:

- Branch before sprint branch: `codex/organize-plan-review-ui-v1`.
- Created sprint branch: `codex/rename-staging-plan-preview-v1`.
- Known unrelated dirty files were already present: `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, plus existing `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` hunks.

Current wording inventory:

| Area | Finding | Classification | Decision |
| --- | --- | --- | --- |
| Sidebar navigation | `staging` route showed visible label `Staging`. | User-facing UI copy | Rename visible label to `Plan Preview`; keep route id `staging`. |
| `StagingScreen` heading/body | Title and safety copy said `Staging`, `Staging is preview-only`, `No staged content`, and `staging area`. | User-facing UI copy | Rename to `Plan Preview`, `Pending plans`, `Loading plan preview`, and keep no-files-changed safety copy. |
| `StagingScreen` area card | Non-numeric item label used `Uncommitted`. | User-facing UI copy | Rename to `Pending plan`; avoid commit-style language. |
| Staging plan mock/API fallback | Mock title/caveats said `Staging preview plan` and `Current Staging data`. | User-facing fallback/mock data | Rename visible mock text to `Plan Preview`; keep internal ids and source values. |
| Organize link | Existing Organize route linked to `Open Staging`. | User-facing UI copy | Rename to `Open Plan Preview`. |
| Field Guide | Current guide text still described Staging and old apply/move style Organize behavior. | Current user-facing help copy | Reword toward Plan Preview and preview-only Organize language. |
| `uiLanguage` | `screenLabel("staging")` returned `Staging`; helper copy described commits/staged items. | User-facing label/helper copy | Rename label to `Plan Preview` and helper copy to pending-plan review language. |
| Downloads copy | A few current strings said `staging area` or `staged files`. | User-facing adjacent copy | Reword to Downloads queue/download files without changing workflow behavior. |
| Tests/proof | Unit tests and desktop proof waited for `Staging` copy. | Test/proof expectation | Update to Plan Preview wording and keep safety assertions. |
| Backend/API names | `StagingPlan`, `StagingArea`, `get_staging_areas`, `get_staging_preview_plan`, mutating staging commands. | Internal code/API | Leave stable for now; document that user-facing wording changed. |
| Historical reports/docs | Older reports and long roadmap text mention Staging. | Historical source material | Leave unless the document is a current-state doc updated by this sprint. |

### 2. Navigation Decision

- Organize remains the planning workspace for generated organization plans.
- Plan Preview / Pending Plans is the user-facing name for the old Staging concept.
- The direct `staging` route remains internally named and routable, but its visible label and copy should say `Plan Preview`.
- Plan Preview should eventually fold into Organize as a preview/plans tab.
- This sprint is a naming/navigation simplification step, not a full route merge.

### 3. User Impact Plan

Users will see clearer wording. Instead of a technical `Staging` page, SimSuite describes this area as `Plan Preview` or `Pending Plans`, meaning a place to review proposed plans before anything happens.

## What Was Audited

- `src/components/layout/Sidebar.tsx`, `src/lib/uiLanguage.ts`, and user-mode route labels.
- Direct Plan Preview route implementation in `src/screens/StagingScreen.tsx` and `src/screens/StagingScreen.test.tsx`.
- Organize relationship in `src/screens/OrganizeScreen.tsx` and existing Organize tests.
- Adjacent current user-facing copy in `src/components/FieldGuide.tsx`, `src/screens/DownloadsScreen.tsx`, and mock fallback data in `src/lib/api.ts`.
- Current docs: navigation plan, trust boundaries, backend systems map, session handoff, and implementation status.
- Desktop proof selector expectations in `scripts/desktop/desktop-library-proof.mjs`.
- Linear issue `VEL-11`.

## Naming Decision

- Use `Plan Preview` as the main user-facing label for the old Staging route/concept.
- Use `Pending plans` for empty/list style language.
- Keep `Preview plan` for the plan surface.
- Keep internal code/API names stable for now: `StagingScreen`, `StagingPlan`, `StagingArea`, `get_staging_areas`, and `get_staging_preview_plan`.

## What Changed

- Sidebar visible label for the internal `staging` route is now `Plan Preview`.
- The direct Plan Preview route now uses `Plan Preview`, `Pending plans`, `Loading plan preview`, and preview-only safety copy instead of technical Staging language.
- The non-numeric area label now says `Pending plan` instead of `Uncommitted`.
- Organize now links to `Open Plan Preview`.
- Field Guide and helper copy now describe Organize and Plan Preview as preview-only, not old apply/move-style workflows.
- Adjacent Downloads copy no longer says `staging area` or `staged files` in the touched user-facing strings.
- Mock fallback plan text now says `Plan Preview`.
- Desktop proof waits for Plan Preview copy and captures `plan-preview-rename-v1.png`.
- Current-state docs now record that Plan Preview is the user-facing label while internal staging names remain.

## What This Means For The User

Users see a clearer name. Instead of a technical `Staging` page, SimSuite now calls the area `Plan Preview`, which better explains that it is for reviewing proposed plans before anything changes. No files are changed by this sprint.

## Trust / Safety Boundary

No unsafe automation was added. SimSuite still does not apply, move, delete, clean up, quarantine, replace, or auto-sort files from this work. The route remains preview-only, and internal mutating staging commands remain unexposed by the visible UI.

## Organize vs Plan Preview

- Organize remains the planning workspace where generated organization plans are created and reviewed.
- Plan Preview is the user-facing name for the pending/proposed-plan view backed by internal staging data.
- This sprint did not fold the Plan Preview route into Organize.
- Future consolidation should still be preview-only until the Apply Safety Contract exists.

## Internal Code Decision

Internal staging names stayed stable. This avoids a broad codebase rename while improving user-facing clarity. The report and docs explicitly note that `StagingScreen`, `StagingPlan`, `get_staging_areas`, and `get_staging_preview_plan` remain implementation names for now.

## Files Changed

- `src/components/layout/Sidebar.tsx`: visible nav label changed to `Plan Preview`; footer safety copy softened.
- `src/lib/uiLanguage.ts` and `src/lib/uiLanguage.test.ts`: user-facing label/helper language for the internal staging route.
- `src/screens/StagingScreen.tsx` and `src/screens/StagingScreen.test.tsx`: direct route copy/tests changed to Plan Preview/Pending Plans.
- `src/screens/OrganizeScreen.tsx`: link changed to `Open Plan Preview`.
- `src/components/FieldGuide.tsx`: current help copy aligned with preview-only Organize/Plan Preview behavior.
- `src/lib/api.ts`: mock fallback plan wording changed to Plan Preview.
- `src/screens/DownloadsScreen.tsx`: touched adjacent staging wording changed to Downloads/download-files wording.
- `src/trustBoundaryCopy.test.ts`: preview-only copy guard updated for Plan Preview.
- `scripts/desktop/desktop-library-proof.mjs`: desktop proof expectations and screenshot name updated.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`, `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`, `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`, `SESSION_HANDOFF.md`, `docs/IMPLEMENTATION_STATUS.md`: current-state documentation updates.
- `simsuite-reports/PLAN_PREVIEW_RENAME_NAVIGATION_V1_REPORT.md`: this sprint report.

## Tests

- `npx vitest run src/screens/StagingScreen.test.tsx src/screens/OrganizeScreen.test.tsx src/lib/uiLanguage.test.ts src/trustBoundaryCopy.test.ts`: passed (`4` files, `10` tests).
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`26` files, `97` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- Separate Rust commands were not run because no Rust files changed. Desktop proof/smoke built the release Tauri app and showed existing Rust warning noise only.

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
- Proof summary: `output/desktop/library-proof/latest-summary.json`.
- Proof folder: `output/desktop/library-proof/2026-05-14T01-00-33-794Z`.
- Plan Preview screenshot: `output/desktop/library-proof/2026-05-14T01-00-33-794Z/plan-preview-rename-v1.png`.
- Proof verified Organize still opens/generates a preview, Plan Preview loads, preview-only/no-files-changed copy appears, and no enabled file-changing controls are present.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## What Could Not Be Verified

- Real-user libraries were not scanned.
- Full Plan Preview / Organize route consolidation was not implemented or verified.
- No Apply/file movement behavior was verified because it intentionally was not built.
- Linear issue state was not changed; only a comment was added to the relevant consolidation issue.

## Linear Updates

- Commented on `VEL-11` with the Plan Preview rename summary, validation results, and remaining consolidation boundary.
- Comment id: `fefd2b4e-cd84-4d7f-963f-ff65ef466ad7`.
- Did not create duplicate issues.
- Did not close `VEL-11` because the full Plan Preview / Organize consolidation is still future work.

## Recommended Next Sprint

Plan Preview / Organize consolidation follow-up, still preview-only, to decide whether the direct Plan Preview route becomes an Organize tab/subpage. Apply Safety Contract design remains required before any real file-changing workflow.

## Docs Updated

- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/PLAN_PREVIEW_RENAME_NAVIGATION_V1_REPORT.md`

## Unrelated Worktree Changes

- Left unrelated `.cocoindex_code/*` generated files alone.
- Left unrelated `src/screens/HomeScreen.tsx` changes alone.
- Left pre-existing unrelated `src/styles/globals.css` hunks alone.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` had pre-existing unrelated hunks; only this sprint’s top-section notes should be staged.

## Commit

- `bf06285` - `Rename Staging UI to Plan Preview`

## Final Honest Verdict

Verified: Plan Preview rename/navigation wording v1 is working for the tested paths.
