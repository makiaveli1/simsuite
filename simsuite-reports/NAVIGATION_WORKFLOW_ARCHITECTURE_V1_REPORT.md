# Navigation Workflow Architecture v1 Report

Date: 2026-05-13

Branch: `codex/navigation-workflow-linear-roadmap-v1`

## What Was Audited

- Route/navigation ownership in `src/App.tsx`, `src/components/layout/Sidebar.tsx`, `src/lib/experienceMode.ts`, and `src/lib/uiLanguage.ts`.
- Current screen inventory under `src/screens/*`, with focus on Inbox, Library, Updates, Organize, Review, Creators, Types, Duplicates, Settings, and Staging.
- Staging safety/readiness docs and trust-boundary policy.
- The user-provided master roadmap from `C:\Users\likwi\Downloads\simsuite_master_roadmap_linear_plan.md`.
- Linear workspace/project state for the Veliveli team.

## Current Situation

SimSuite currently exposes more top-level routes than the next product phase likely needs. Several pages are useful, but some are really lenses over Library data rather than independent workflows.

Current examples:

- Creators and Types are better treated as Library lenses.
- Duplicates is partly a Library filter and partly a comparison workbench.
- Inbox, Staging, and Organize need clearer ownership before Auto Sorting starts.
- Guide is help/reference content and can move toward Settings/Help later.

No route/sidebar behavior was changed in this sprint.

## Navigation Recommendation

Recommended future sidebar:

- Home
- Scan
- Library
- Inbox
- Organize
- Updates
- Review
- Settings

Mode recommendations:

- Casual: Home, Scan, Library, Inbox, Updates, Review, Settings.
- Seasoned: Home, Scan, Library, Inbox, Organize, Updates, Review, Settings.
- Creator: Home, Scan, Library, Inbox, Organize, Updates, Review, Duplicates, Settings.

Future fold decisions:

- Staging folds into Organize as a preview/plans tab.
- Creators and Types become Library lenses.
- Duplicates becomes a Library/Review comparison workbench, with Creator-mode top-level visibility allowed until migration is proven.
- Review stays top-level for now.
- Guide moves toward Help/Settings later.

## Inbox vs Staging vs Organize

- Inbox = intake / new content.
- Staging = preview of a proposed plan.
- Organize = planning workspace for suggested organization.

This distinction is important because Auto Sorting should not start as file movement. It should start as a suggested plan that users can review.

## Linear Setup

Linear project created:

- [SimSuite - Safe Automation Roadmap](https://linear.app/veliveli/project/simsuite-safe-automation-roadmap-b038fd0f2cdb)

Team:

- Veliveli

Milestones created:

- M0 - Project Control & Roadmap
- M1 - Navigation & Workflow Simplification
- M2 - Staging Preview Plan Foundation
- M3 - Auto Sorting Suggested Plans
- M4 - Apply Safety Contract
- M5 - Updates Provider Onboarding
- M6 - AI-Assisted Suggestions
- M7 - Real-Library Validation & Beta Readiness
- M8 - Technical Debt & Proof Infrastructure

Labels created:

- `area:*` labels for Library, Inbox, Staging, Organize, Updates, Duplicates, Review, Navigation, Settings, Proof, Docs, Backend, Frontend, and Trust.
- `type:*` labels for audit, implementation, proof, docs, test, refactor, bug, and research.
- `trust:*` labels for informational, evidence-backed, review-only, suggested-plan, file-changing-blocked, requires-confirmation, requires-backup, and ai-boundary.
- `P0`, `P1`, `P2`, and `P3`.

Linear status customization was not changed. The direct issue tools created issues in the existing `Backlog` state.

An earlier Linear research helper check was unavailable, so setup used direct Linear project, milestone, label, and issue tools. Those direct tools were sufficient for the sprint.

## What This Means For The User

This does not change how the app behaves today. It gives SimSuite a clearer development map so the next work happens in the right order.

For users, the practical result is that future navigation cleanup, Staging, Auto Sorting, Updates, and AI work now has a tracked plan with safety gates. The app should become less crowded later, but no pages were removed in this sprint.

## Trust / Safety Boundary

No unsafe automation was added.

Staging remains preview/readiness work only. Auto Sorting remains future suggested-plan work. No files are moved, deleted, disabled, quarantined, replaced, or auto-sorted. AI is not deciding anything. Any future file-changing workflow still needs preview, user confirmation, backup/restore, path validation, conflict handling, recoverable errors, result logs, tests, and proof.

## Files Changed

- `docs/planning/simsuite_master_roadmap_linear_plan.md`: repo-local copy of the user-provided master roadmap Markdown.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`: current route inventory, overlap decisions, mode model, migration plan, safety plan, and Linear issue plan.
- `simsuite-reports/NAVIGATION_WORKFLOW_ARCHITECTURE_V1_REPORT.md`: sprint report.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: cross-link to the navigation/workflow plan.
- `SESSION_HANDOFF.md`: top session handoff note for this planning sprint.
- `docs/IMPLEMENTATION_STATUS.md`: top status note for this planning sprint.

## Linear Items Created

Project:

- [SimSuite - Safe Automation Roadmap](https://linear.app/veliveli/project/simsuite-safe-automation-roadmap-b038fd0f2cdb)

Issues:

- [VEL-5](https://linear.app/veliveli/issue/VEL-5/m0-1-create-linear-project-and-labels) `[M0-1] Create Linear project and labels`
- [VEL-6](https://linear.app/veliveli/issue/VEL-6/m0-2-current-repodocs-source-of-truth-audit) `[M0-2] Current repo/docs source-of-truth audit`
- [VEL-7](https://linear.app/veliveli/issue/VEL-7/m1-1-full-route-inventory) `[M1-1] Full route inventory`
- [VEL-8](https://linear.app/veliveli/issue/VEL-8/m1-2-inbox-vs-staging-vs-organize-decision) `[M1-2] Inbox vs Staging vs Organize decision`
- [VEL-9](https://linear.app/veliveli/issue/VEL-9/m1-3-sidebar-simplification-proposal) `[M1-3] Sidebar simplification proposal`
- [VEL-10](https://linear.app/veliveli/issue/VEL-10/m1-4-fold-creators-and-types-into-library) `[M1-4] Fold Creators and Types into Library`
- [VEL-11](https://linear.app/veliveli/issue/VEL-11/m1-5-fold-staging-into-organize) `[M1-5] Fold Staging into Organize`
- [VEL-12](https://linear.app/veliveli/issue/VEL-12/m2-1-define-stagingplan-model) `[M2-1] Define StagingPlan model`
- [VEL-13](https://linear.app/veliveli/issue/VEL-13/m2-2-add-preview-only-staging-plan-command) `[M2-2] Add preview-only staging plan command`
- [VEL-14](https://linear.app/veliveli/issue/VEL-14/m2-3-staging-plan-ui) `[M2-3] Staging plan UI`
- [VEL-15](https://linear.app/veliveli/issue/VEL-15/m3-1-sorting-rules-audit) `[M3-1] Sorting rules audit`
- [VEL-16](https://linear.app/veliveli/issue/VEL-16/m3-2-suggested-plan-generator-v1) `[M3-2] Suggested plan generator v1`
- [VEL-17](https://linear.app/veliveli/issue/VEL-17/m3-3-organize-plan-review-ui) `[M3-3] Organize plan review UI`
- [VEL-18](https://linear.app/veliveli/issue/VEL-18/m4-1-backuprestore-design) `[M4-1] Backup/restore design`
- [VEL-19](https://linear.app/veliveli/issue/VEL-19/m4-2-dry-runapply-split) `[M4-2] Dry-run/apply split`
- [VEL-20](https://linear.app/veliveli/issue/VEL-20/m4-3-confirmed-apply-prototype) `[M4-3] Confirmed apply prototype`
- [VEL-21](https://linear.app/veliveli/issue/VEL-21/m5-1-provider-architecture-audit) `[M5-1] Provider architecture audit`
- [VEL-22](https://linear.app/veliveli/issue/VEL-22/m5-2-source-confidence-model-v2) `[M5-2] Source confidence model v2`
- [VEL-23](https://linear.app/veliveli/issue/VEL-23/m5-3-curseforge-official-api-planning) `[M5-3] CurseForge official API planning`
- [VEL-24](https://linear.app/veliveli/issue/VEL-24/m6-1-ai-boundary-design) `[M6-1] AI boundary design`
- [VEL-25](https://linear.app/veliveli/issue/VEL-25/m6-2-mock-ai-organization-explanation) `[M6-2] Mock AI organization explanation`
- [VEL-26](https://linear.app/veliveli/issue/VEL-26/m7-1-sanitized-diagnostics-plan) `[M7-1] Sanitized diagnostics plan`
- [VEL-27](https://linear.app/veliveli/issue/VEL-27/m7-2-user-approved-real-scan) `[M7-2] User-approved real scan`
- [VEL-28](https://linear.app/veliveli/issue/VEL-28/m8-1-rust-warning-cleanup-audit) `[M8-1] Rust warning cleanup audit`
- [VEL-29](https://linear.app/veliveli/issue/VEL-29/m8-2-vite-chunk-warning-audit) `[M8-2] Vite chunk warning audit`
- [VEL-30](https://linear.app/veliveli/issue/VEL-30/m8-3-worktree-hygiene-cleanup) `[M8-3] Worktree hygiene cleanup`

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`23` files, `89` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

Rust was not planned because no Rust files were touched.

## Desktop / Runtime Proof

Desktop proof and smoke were not run because this sprint did not change route code, frontend runtime behavior, backend commands, schemas, or app UI behavior.

## What Could Not Be Verified

- Actual navigation simplification was not verified because no route/sidebar behavior was implemented.
- Linear custom workflow/status changes were not configured; created issues use existing Linear status behavior.
- The roadmap HTML copy was not added because the Markdown source was available and is the repo-local source artifact.

## Recommended Next Sprint

Next: Staging Preview Plan Foundation v1.

That should define the `StagingPlan` contract first, then add a read-only preview command and UI only after the contract is clear. It should still avoid file movement.

## Docs Updated

- `docs/planning/simsuite_master_roadmap_linear_plan.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `simsuite-reports/NAVIGATION_WORKFLOW_ARCHITECTURE_V1_REPORT.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated Worktree Changes

Existing unrelated dirty files were observed and left unstaged:

- `.cocoindex_code/cocoindex.db/mdb/data.mdb`
- `.cocoindex_code/target_sqlite.db`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated hunks in `SESSION_HANDOFF.md`
- pre-existing unrelated hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

This report was written before the final git commit hash existed. The final Codex response records the completed commit hash.

## Final Honest Verdict

Verified: Navigation/workflow roadmap and Linear setup v1 is complete for planning.
