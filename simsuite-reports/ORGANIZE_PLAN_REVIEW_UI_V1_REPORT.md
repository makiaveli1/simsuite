# Organize Plan Review UI v1 Report

Date: 2026-05-13

Branch: `codex/organize-plan-review-ui-v1`

## Pre-Implementation Audit

### 1. Existing Organize Page Audit

Current purpose:

- The existing `OrganizeScreen` is the top-level planning route for organization workflows.
- It currently mixes a legacy organization preview, rule preset selection, sample rows, snapshots, and apply/restore controls.

Current data source:

- `api.listRulePresets()`
- `api.previewOrganization(...)`
- `api.listSnapshots(...)`
- `api.applyPreviewOrganization(...)`
- `api.restoreSnapshot(...)`

Current UI sections:

- summary rail with safe/review/aligned counts.
- preset selector.
- issue summary.
- sample preview list.
- snapshot/restore panel.
- preview inspector.

Current actions:

- refresh legacy preview.
- choose preset.
- open Review.
- show all/sample preview rows.
- apply legacy organization preview.
- restore snapshot.

Legacy apply/move risk:

- The current page exposes enabled “Move ready files” / “Apply safe moves” controls when `safeCount` is nonzero.
- It calls `api.applyPreviewOrganization(...)`, which maps to `apply_preview_organization`.
- It uses copy such as “safe moves”, “Ready to move”, and “Safe to move”.
- This is outside the trust boundary for the new preview-only Auto Sorting work.

What will be preserved:

- The route owner remains `OrganizeScreen`.
- The page stays the organization planning workspace.
- The Review navigation button is preserved.
- Existing app route/sidebar behavior is preserved.

What will be disabled, hidden, or reworded:

- Legacy apply/restore controls are removed from the visible route for this sprint.
- Legacy “safe move” and “ready to move” copy is replaced with preview-only wording.
- The page stops calling legacy preview/apply/snapshot APIs.

What will be replaced by generated plan review:

- The legacy OrganizationPreview surface will be replaced by `generateSortingPreviewPlan(...)` output rendered as a preview-only `StagingPlan`.

### 2. Organize vs Staging Decision

- Organize owns the planning workspace.
- Staging remains the preview/readiness layer for app-local staged folders.
- This sprint adds generated plan review to Organize.
- This sprint does not delete Staging.
- This sprint does not fully fold Staging into Organize.
- The direct Staging route remains safe and preview-only.

### 3. UI Scope Decision

Smallest useful v1 scope:

- Use the existing Organize page.
- Add one “Generate preview” area.
- Use bounded Library folder scope controls: Mods/Tray, folder path, recursive toggle, and capped limit.
- Render one generated plan result surface grouped by bucket.
- Do not add Apply, file movement, cleanup, delete, quarantine, or AI.

Selected-file scope is supported by the backend but not exposed in this UI because Organize does not currently receive a selected Library file context.

### 4. User Impact Plan

Users can open Organize and generate a safe preview plan. SimSuite shows suggested buckets, reasons, caveats, blocked reasons, and source signals, but no files are changed.

## What Was Audited

- Existing `OrganizeScreen` route ownership, data calls, visible actions, and legacy apply/snapshot controls.
- Staging relationship and direct route safety.
- `generateSortingPreviewPlan(...)` API shape, `StagingPlan` fields, buckets, evidence labels, source signals, blocked reasons, and `wouldTouchFiles=false` behavior.
- Legacy Organize and Staging action risks, including apply, commit, cleanup, and move-style paths.
- Frontend tests, trust-boundary copy tests, desktop proof, and Linear issue `VEL-17`.

## Existing Organize Page Decision

- Replaced the visible legacy Organize surface with preview-only generated plan review.
- Preserved the existing `OrganizeScreen` route and route ownership.
- Removed visible legacy preset, snapshot, apply, and restore controls from the route.
- Kept Review/Library/Staging navigation available without creating a new route.

## Organize vs Staging

- Organize now owns the visible generated-plan review workspace.
- Staging remains a separate direct route for staged-folder preview/readiness.
- This sprint did not fold the full Staging route into Organize.
- Direct Staging remains preview-only and safe.

## What Changed

- `OrganizeScreen` now calls `api.generateSortingPreviewPlan(...)` for bounded Mods/Tray folder scopes.
- Added bounded scope controls for source root, folder path, recursive mode, and preview limit.
- Rendered generated plans grouped by bucket with item reasons, caveats, source signals, blocked reasons, confidence/evidence labels, current paths, and suggested destination preview strings.
- Added “No files changed” and preview-only safety copy.
- Added focused Organize route tests and extended trust-boundary copy coverage.
- Updated desktop proof to open Organize, generate a preview plan, assert review detail is visible, and assert no enabled file-changing controls are present.
- Updated trust/navigation/backend docs and repo memory docs.

## What This Means For The User

Users can now open Organize and generate a preview plan for a bounded Library folder. SimSuite shows suggested buckets, reasons, and caveats, but it does not move, delete, replace, clean up, or change files. Users still need to review the plan manually.

## Trust / Safety Boundary

This sprint added no unsafe automation. Organize is still preview-only. No files are moved, deleted, cleaned up, quarantined, replaced, committed, or auto-sorted. The new UI does not expose enabled Apply or file-changing controls. Staging remains separate and preview-only. Future file-changing work still requires the Apply Safety Contract.

## Plan Review Behavior

- Scope controls: Mods/Tray root, optional folder path, nested-folder toggle, and capped item limit.
- Plan states: initial, loading, blocked/empty, success, and error.
- Plan summary: title, summary, status, item count, caveats, and “No files changed.”
- Plan item groups: Script Mods, CAS, Build/Buy, Gameplay, Presets & Sliders, Overrides & Defaults, Tray, Needs Review, and Unknown / Leave in place.
- Plan item details: file name, current root, current path, suggested destination when provided, action kind, evidence level, confidence label, reason, caveats, source signals, and blocked reasons.

## Legacy Apply Safety

- The previous visible Organize route called `previewOrganization`, `applyPreviewOrganization`, `listSnapshots`, and `restoreSnapshot`.
- The new visible Organize route does not call those APIs.
- The UI no longer renders the previous enabled “move/apply” style controls.
- Backend legacy commands still exist elsewhere, but they are not used by this generated-plan UI.

## Files Changed

- `src/screens/OrganizeScreen.tsx`: replaced legacy visible Organize surface with preview-only generated plan review.
- `src/screens/OrganizeScreen.test.tsx`: added route tests for preview generation, grouped plan rendering, blocked/error states, and no legacy API calls.
- `src/trustBoundaryCopy.test.ts`: added Organize to trust-sensitive copy coverage and checked that it stays preview-only.
- `src/styles/globals.css`: added focused Organize plan review layout styles and responsive behavior.
- `scripts/desktop/desktop-library-proof.mjs`: added Organize proof step and screenshot.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: recorded the Organize generated-plan UI boundary.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`: updated current Organize/M3 state.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: recorded that Organize now consumes the preview generator and no longer calls legacy apply-style APIs.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: added current sprint status notes.

## Tests

- `npx vitest run src/screens/OrganizeScreen.test.tsx src/trustBoundaryCopy.test.ts`: passed (`2` files, `7` tests).
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`25` files, `96` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.
- Separate Rust commands were not run because no Rust files changed. The desktop proof/smoke lanes built the release Tauri app and showed existing Rust warning noise only.

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
- Latest proof summary: `output/desktop/library-proof/latest-summary.json`.
- Proof folder: `output/desktop/library-proof/2026-05-13T23-15-26-072Z`.
- Organize screenshot: `output/desktop/library-proof/2026-05-13T23-15-26-072Z/organize-plan-review-ui-v1.png`.
- Proof verified Organize route load, generated plan details, “No files changed,” and no enabled file-changing controls.
- `npm run desktop:smoke:fixtures`: passed with `Desktop smoke passed`.

## What Could Not Be Verified

- Real-user libraries were not scanned or used.
- Selected Library file scope was not exposed in the UI because Organize has no selected-file context yet.
- No Apply/file-moving behavior was verified because it intentionally was not built.
- Linear issue state was not changed; the research helper errored, so the sprint recorded a direct VEL-17 comment instead.

## Linear Updates

- Commented on `VEL-17` with what landed, validation results, and the remaining Apply Safety Contract boundary.
- Comment id: `aae3cba9-c9fa-451f-ba90-8b3fad2e8108`.
- Did not create duplicate issues.
- Did not move/close `VEL-17` because direct issue-state inspection was limited and the research helper returned an error.

## Recommended Next Sprint

- Staging/Organize consolidation follow-up, still preview-only, if navigation simplification should continue.
- Apply Safety Contract design is the next trust-sensitive prerequisite before any real file-changing workflow.

## Docs Updated

- `simsuite-reports/ORGANIZE_PLAN_REVIEW_UI_V1_REPORT.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`

## Unrelated Worktree Changes

- Left unrelated `.cocoindex_code/*` generated files alone.
- Left unrelated `src/screens/HomeScreen.tsx` changes alone.
- `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md` had pre-existing unrelated hunks; only this sprint’s relevant hunks should be staged.

## Commit

Committed as `Show preview-only organization plans in Organize`. The final Codex report records the exact hash because amending this report changes the commit hash.

## Final Honest Verdict

Verified: Organize Plan Review UI v1 is working for the tested paths.
