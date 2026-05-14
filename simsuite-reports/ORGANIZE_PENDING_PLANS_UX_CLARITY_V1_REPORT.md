# Organize Pending Plans UX Clarity v1 Report

Date: 2026-05-14

Branch: `codex/organize-pending-plans-ux-clarity-v1`

## Pre-Implementation Audit

### 1. Pending Data Meaning Audit

Worktree at start:

- Starting branch: `codex/plan-preview-organize-consolidation-v1`.
- Created sprint branch: `codex/organize-pending-plans-ux-clarity-v1`.
- Known unrelated dirty files were already present: `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, plus existing `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` hunks.

Data findings:

| Source | What it returns | Meaning | UX risk |
| --- | --- | --- | --- |
| `get_staging_areas` | Folder groups under app-local `downloads_inbox`, including item IDs, subfolder names, file counts, sizes, and paths. | Imported/downloaded batches waiting for review. | Raw IDs and folder names can look like broken plan names. |
| `get_staging_preview_plan` | A preview-only `StagingPlan` built from those folder groups. | Folder-level readiness data, not a saved organization plan. | Repeated `suggest_review` rows make technical folders look like user decisions. |
| `generate_sorting_preview_plan` | Generated organization suggestions for bounded Library scopes. | Real preview plan generator output. | Should stay under `Create plan`, not be confused with app-local batches. |

The visible timestamp-like numbers are internal folder/item IDs or fixture-style folder names from app-local pending batch data. The large counts are produced by folder counting in the current pending batch data. This sprint does not prove whether a count is stale or wrong; it changes the UI to treat those counts as review data rather than plan truth.

Current pending data does not represent saved organization plans. It represents app-local imported/downloaded batches and compatibility data. That data can be useful, but it belongs in a compact summary with next steps, not as primary plan titles.

### 2. UX Decision

Pending plans should not list raw internal staging folders as if they are organization plans.

V1 behavior:

- Keep `Create plan` as the place to generate preview-only organization suggestions.
- Make `Pending plans` explain that generated plans are not saved yet.
- Summarize app-local imported/downloaded batches as `Pending batch 1`, `Pending batch 2`, and so on.
- Hide raw technical IDs and folder names unless the user opens technical details.
- Cap visible pending batch rows and provide a show-more control.
- Point users to `Open Inbox` for imported/downloaded batch review and `Create preview plan` for organization suggestions.
- Keep `No files changed` visible.

### 3. User Impact Plan

Users will no longer see random folder IDs as plan names. Pending plans will explain whether there are real saved plans or only imported/downloaded batches waiting for review. The user will know what to do next, and no files are changed.

## What Was Audited

- Pending Plans data and UI.
- Organize `Create plan` / `Pending plans` tabs.
- Direct internal Plan Preview route.
- Read-only backend data shape for `get_staging_areas` and `get_staging_preview_plan`.
- Current trust and navigation docs.
- Desktop proof coverage.
- Linear issue `VEL-11`.

## Pending Data Meaning

Current pending data is folder-level app-local batch data, not saved organization plan data. It can show that imported/downloaded content exists and how much data is present, but it does not provide per-file organization suggestions or apply-ready destinations.

## UX Decision

Pending Plans now treats app-local batch data as a compact review summary. Raw internal IDs and folder names are not primary UI labels. They are available only behind technical details.

## What Changed

- Reworked `PendingPlansPreview` around meaning, next steps, and safety.
- Added friendly `Pending batch N` rows.
- Added summary stats, large-count caution copy, and row capping.
- Hid raw internal folder IDs and folder names by default.
- Added technical-detail reveal for users who need internal IDs.
- Clarified Create Plan vs Pending Plans copy.
- Updated proof to verify raw IDs are not primary labels and technical details are collapsed by default.

## What This Means For The User

Users can understand Pending Plans without reading internal app folders. They see whether there are pending imported/downloaded batches, what they mean, and what to do next. No files are changed.

## Trust / Safety Boundary

No unsafe automation was added. SimSuite still does not apply, move, delete, clean up, quarantine, replace, or auto-sort files from this workflow. AI is not deciding file actions. Pending Plans remains preview-only.

## Create Plan vs Pending Plans

- `Create plan`: generate a new preview-only organization suggestion from a bounded Library scope.
- `Pending plans`: review saved or pending preview work. Since generated organization plans are not saved yet, this currently summarizes imported/downloaded batches when they exist.

## Direct Plan Preview Route Behavior

The direct internal Plan Preview route remains safe. It still points users back to Organize and now uses the same clearer Pending Plans summary.

## Files Changed

- `src/screens/organize/PendingPlansPreview.tsx`: meaning-first Pending Plans UI.
- `src/screens/OrganizeScreen.tsx`: clearer tab copy and Create Plan handoff.
- `src/screens/OrganizeScreen.test.tsx`: Pending Plans UX coverage.
- `src/screens/StagingScreen.test.tsx`: direct route safety and hidden technical details coverage.
- `src/styles/globals.css`: compact Pending Plans summary, batch rows, and technical-detail styling.
- `scripts/desktop/desktop-library-proof.mjs`: runtime proof checks for readable Pending Plans UI.
- Current-state docs and this report.

## Tests

- `npm run test:unit -- --run src/screens/OrganizeScreen.test.tsx src/screens/StagingScreen.test.tsx src/trustBoundaryCopy.test.ts`: passed (`3` files, `11` tests).
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`27` files, `100` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

## Desktop / Runtime Proof

- Browser rendered QA against `http://127.0.0.1:1420/#organize`: passed.
  - Pending plans showed friendly `Pending batch 1` labeling.
  - Raw timestamp-like IDs were not visible in primary labels.
  - Technical details were collapsed by default.
  - No enabled file-changing controls were present.
  - The Organize shell scrolled at a smaller desktop viewport.
- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
  - Output folder: `output/desktop/library-proof/2026-05-14T22-49-49-900Z/`.
  - Screenshot: `output/desktop/library-proof/2026-05-14T22-49-49-900Z/organize-pending-plans-ux-clarity-v1.png`.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## What Could Not Be Verified

- This sprint did not prove whether any unusually large pending batch count is stale or incorrect. The UI now treats the count as review data and warns when it looks large.
- No real user Mods, Tray, or Downloads files were used.
- No backend changes were made, so separate Rust commands were not run; desktop proof/smoke built the release Tauri app and showed the existing Rust warning noise.

## Linear Updates

- Commented on `VEL-11` with the UX clarification summary and validation results.
- Left `VEL-11` open because this sprint clarified the folded Pending Plans experience, but future cleanup may still move imported/downloaded batch review more naturally into Inbox or add saved plan persistence.

## Recommended Next Sprint

Inbox batch review clarity, because imported/downloaded batches naturally belong in the intake workflow. Apply Safety Contract design should still happen before any real Apply or file movement work starts.

## Docs Updated

- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/ORGANIZE_PENDING_PLANS_UX_CLARITY_V1_REPORT.md`

## Unrelated Worktree Changes

Unrelated `.cocoindex_code/*`, Home, global CSS Home hunks, and pre-existing status/handoff hunks were left alone.

## Commit

Pending commit.

## Final Honest Verdict

Verified: Organize Pending Plans UX Clarity v1 is working for the tested paths.
