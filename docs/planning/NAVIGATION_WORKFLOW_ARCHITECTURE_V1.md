# SimSuite Navigation Workflow Architecture v1

Date: 2026-05-13

Source roadmap: `docs/planning/simsuite_master_roadmap_linear_plan.md`

This document turns the master roadmap into a route/workflow ownership plan. It exists so implementation sprints can simplify SimSuite deliberately before Auto Sorting, internal StagingPlan work, AI assistance, or file-changing workflows are built.

## Current Route Inventory

| Route / action | Visible label | Screen/component | Current purpose | Data source | Real/mocked/partial | User modes visible in today | Trust level | Top-level candidate? | Recommendation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `home` | Home | `HomeScreen` | Landing/status surface. | Frontend state and app summaries. | Real, with unrelated local changes currently dirty. | Casual, Seasoned, Creator | Informational | Yes | Keep top-level. |
| Sidebar action | Scan | `Sidebar` action using `onScan` | Starts Library scan workflow. | Tauri/backend scan command path. | Real action | Casual, Seasoned, Creator | User-confirmed action | Yes, as action | Keep as primary action, not a page. |
| Sidebar action | Guide | `FieldGuide` drawer | Help/reference content. | Frontend guide content. | Real action | Casual, Seasoned, Creator | Informational | No | Move toward Help/Settings later. |
| `downloads` | Inbox | `DownloadsScreen` | Intake and review for new/downloaded content and imported/downloaded batches before Library or Organize planning. | Downloads/inbox backend and fixture data. | Real/partial | Casual, Seasoned, Creator | Review workflow; current visible UI is review-only | Yes | Keep top-level as intake/new content. Do not expose Apply/move/delete-style actions until the future safety contract exists. |
| `library` | Library / My CC | `LibraryScreen` | Main indexed Mods/Tray browser, filters, folders, duplicates, details. | Library backend and SQLite. | Real | Casual, Seasoned, Creator | Informational to evidence-backed | Yes | Keep top-level as primary work surface. |
| `updates` | Updates | `UpdatesScreen` | Update source tracking and trust-first checks/reminders. | Updates/content version backend. | Real/partial | Casual, Seasoned, Creator | Evidence-backed/review-only | Yes | Keep top-level. |
| `organize` | Organize / Tidy Up | `OrganizeScreen` | Organization planning workspace with visible `Create plan`, `Saved plans`, and `Pending batches` preview-only tabs. Saved plans review draft preview records and can show read-only validation/conflict previews, read-only Dry-run preview classifications, plus read-only Recovery history metadata; Pending batches hands imported/downloaded batch review back to Inbox. | `generate_sorting_preview_plan`, `build_apply_plan_from_staging_plan`, `list_saved_apply_plans`, `get_apply_plan`, `preview_apply_plan_validation`, `preview_apply_plan_dry_run`, `list_apply_plan_run_logs`, `list_apply_plan_result_logs`, `list_apply_plan_restore_entries`, `delete_draft_apply_plan`, `get_staging_areas`, and `get_staging_preview_plan` through typed API/mock data. | Real preview-only v1 | Casual, Seasoned, Creator | Suggested plan, validation preview, dry-run classification UI, and recovery metadata review only; DB-only draft records; no file-changing action | Yes | Keep top-level as the owner of organization planning and saved draft preview review. Inbox owns detailed imported/downloaded batch review. |
| `review` | Review / Needs | `ReviewScreen` | Manual review queue and review workflow. | App data/API. | Real/partial | Casual, Seasoned, Creator | Review-only | Yes | Keep top-level for now. |
| `creatorAudit` | Creators | `CreatorAuditScreen` | Creator-focused browsing/audit lens. | Library/metadata. | Partial/real lens | Casual, Seasoned, Creator | Informational/review-only | No long-term | Fold into Library creator lens. |
| `categoryAudit` | Types | `CategoryAuditScreen` | Type/content-kind browsing/audit lens. | Library/metadata. | Partial/real lens | Casual, Seasoned, Creator | Informational/review-only | No long-term | Fold into Library type lens. |
| `duplicates` | Duplicates / Same file? | `DuplicatesScreen` | Duplicate and comparison workbench. | Duplicate backend. | Real | Casual, Seasoned, Creator | Deterministic duplicate proof plus review-only rows | Maybe | Move toward Library/Review comparison workbench; allow Creator top-level until migration. |
| `settings` | Settings | `SettingsScreen` | Configuration and preferences. | Frontend/backend settings. | Real | Casual, Seasoned, Creator | Informational | Yes | Keep top-level. |
| `staging` | Plan Preview | `StagingScreen` | Direct compatibility route for pending plans and app-local staged areas. The normal user-facing surface now lives inside Organize, and raw technical IDs are hidden unless technical details are opened. | `get_staging_areas` and `get_staging_preview_plan` read-only commands. | Real but intentionally preview-only | Hidden from normal sidebar; direct route remains routable | Suggested plan/readiness only | No | Keep direct route safe for compatibility; do not expose as a normal top-level workflow. |

Notes:

- `Scan` and `Guide` are sidebar actions, not `Screen` route values.
- Current mode visibility is broader than the recommended future model. This document records the target model only; it does not change route visibility.
- Internal staging backend mutating commands still exist from earlier work, but the visible Plan Preview route no longer calls them.

## Page Purpose Definitions

| Page | Product purpose |
| --- | --- |
| Home | Quick status and re-entry surface. |
| Library | The source of truth for indexed Mods/Tray files, folders, filters, details, duplicate evidence, preview state, and review signals. |
| Inbox | Intake/new content. This is where newly downloaded or imported batches should be checked before they become part of normal Library or Organize planning. |
| Organize | Planning workspace for suggested organization. It now owns creating preview plans, reviewing saved draft preview plans, and showing pending batch handoff notes. |
| Plan Preview | Preview of a proposed plan. This is the user-facing name for the current internal Staging concept; saved preview records now appear inside Organize as `Saved plans`, pending imported/downloaded batches appear as `Pending batches`, and the direct route remains a safe compatibility surface. |
| Updates | Trust-first update tracking, reminder-only sources, and supported checker results. |
| Review | Manual review queue for files that need attention, comparison, or user judgment. |
| Duplicates | Comparison workbench for deterministic duplicates and review-only name/version rows. It should not imply cleanup. |
| Creators | Creator lens over Library metadata. Long-term this belongs inside Library, not as a top-level page. |
| Types | Type/content-kind lens over Library metadata. Long-term this belongs inside Library, not as a top-level page. |
| Guide | Help/reference content. Long-term this should move toward Help/Settings. |
| Settings | Preferences, app configuration, and future help access. |

Key distinction:

- Inbox = intake / new content.
- Plan Preview = preview of a proposed plan.
- Organize = planning workspace for suggested organization.

Future ApplyPlan persistence planning now lives in
`docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`. Saved plan and result-log
storage design lives there. The first DB-only ApplyPlan persistence foundation
now exists for draft/preview records, but no visible saved-plan UI or
file-changing Apply behavior exists yet.

The first backend-owned ApplyPlan builder now exists as a DB-only backend/API
path. Organize now exposes the first saved-plan review UI on top of that
foundation: users can save generated preview plans as draft records, list them,
open details, and cancel drafts. No file-changing Apply behavior exists yet.

The first read-only ApplyPlan validation preview command now exists behind the
API. It can inspect saved draft records for stale sources, missing files,
unsafe destinations, destination conflicts, review-only blockers, and
backup/restore requirements while keeping `canProceedToConfirmation=false`.
Organize `Saved plans` now has a visible validation preview review section. No
Apply workflow exists yet.

Backup/restore/result-log design now lives in
`docs/planning/BACKUP_RESTORE_RESULT_LOG_DESIGN_V1.md`. It defines the future
recovery contract for ApplyPlan runs, result logs, and restore maps, but no
route exposes backup, restore, Apply, or file-changing controls.

The DB-only result/restore schema foundation now exists behind backend/API
commands for future ApplyPlan run logs, per-file result logs, and restore-map
entries. Organize does not expose those controls yet, and no route exposes
backup, restore, Apply, or file-changing behavior.

The fixture-only backup prototype now exists as private backend test/prototype
code. No route exposes it, and Organize still has no backup, restore, Apply, or
file-changing controls.

The fixture-only restore prototype now exists in the same private backend
test/prototype module. No route exposes it, and Organize still has no backup,
restore, Apply, or file-changing controls.

The fixture-only backup + restore integration proof now exists in the private
backend test/prototype module. It has no route, command, or API exposure, and
Organize still has no backup, restore, Apply, or file-changing controls.

Organize `Saved plans` now has a read-only `Recovery history` section. It uses
the DB-only result/restore list APIs to show existing run logs, result logs,
and restore-map records for a saved draft. It does not call create/record APIs,
does not expose fixture proof helpers, and still has no backup, restore, Apply,
or file-changing controls.

Dry-run Apply design now lives in
`docs/planning/DRY_RUN_APPLY_DESIGN_V1.md`. It recommends a future read-only
dry-run preview near saved-plan validation and recovery history, but this
planning sprint adds no route, command, API, confirmation workflow, Apply,
Restore, backup, or file-changing control.

The first `preview_apply_plan_dry_run` command now exists as a backend/API-only
read-only preview. Organize `Saved plans` now exposes a read-only `Dry-run
preview` section that calls this API and displays item classifications without
adding Apply, Restore, Backup, confirmation, fixture proof helpers, result-log
writes, restore-entry writes, backup execution, restore execution, or
file-changing controls.

## Overlap Audit

| Overlap | Decision | Reason | Migration note |
| --- | --- | --- | --- |
| Inbox vs Plan Preview | Keep separate concepts. | Inbox is intake. Plan Preview is proposed change preview. Current imported/downloaded batch data belongs more naturally in Inbox. | Inbox may send selected items into Organize preview plans later, but Inbox should own detailed downloaded/imported batch review. |
| Organize vs Plan Preview | Folded for v1 into Organize as `Create plan`, `Saved plans`, and `Pending batches`. | Users should not have to understand a separate technical Staging page before a proposed organization plan exists. | Keep the internal `staging` route safe if directly opened during migration. |
| Duplicates vs Library duplicate filter | Make Duplicates a Library/Review comparison workbench; allow Creator-mode top-level until migration. | Library already owns duplicate counts and filters; Duplicates is useful when comparing evidence. | Do not remove until duplicate review flows have an equivalent Library/Review entry. |
| Creators vs Library creator lens/filter | Fold into Library lens. | Creator browsing is a Library view, not a separate workflow. | Preserve filtering and summaries. |
| Types vs Library type lens/filter | Fold into Library lens. | Type browsing is a Library view, not a separate workflow. | Preserve type summaries and filters. |
| Review vs Library Needs Review filter | Keep Review top-level for now. | Manual review is a workflow, not only a filter. | Revisit after Library/Review handoff is clearer. |
| Guide vs Settings/help | Move toward Help/Settings later. | Guide is support content, not a core workflow page. | Keep sidebar action until Help/Settings destination is ready. |

## Mode Visibility Model

This is the recommended future model. It is not implemented by this planning sprint.

| Mode | Recommended visible items |
| --- | --- |
| Casual | Home, Scan, Library, Inbox, Updates, Review, Settings |
| Seasoned | Home, Scan, Library, Inbox, Organize, Updates, Review, Settings |
| Creator | Home, Scan, Library, Inbox, Organize, Updates, Review, Duplicates, Settings |

What happens to current pages:

- Plan Preview: folds into Organize as saved preview plans plus pending batch
  handoff notes; internal `staging` route/name can remain during migration.
- Creators: becomes a Library lens/filter.
- Types: becomes a Library lens/filter.
- Duplicates: becomes a Library/Review comparison workbench; Creator top-level can remain until migration is proven.
- Guide: moves toward Help/Settings.

## Recommended Final Sidebar Model

Recommended long-term top-level sidebar:

1. Home
2. Scan
3. Library
4. Inbox
5. Organize
6. Updates
7. Review
8. Settings

Creator mode may keep Duplicates as a top-level item until duplicate comparison is fully folded into Library/Review.

## Migration Plan

### Phase 1 - Clarify Names And Copy Only

- Do not remove routes.
- Clarify route ownership in docs.
- Keep Plan Preview preview-only.
- Keep Organize from implying automatic sorting or file changes.

### Phase 2 - Fold Plan Preview Into Organize As Preview-Only

- Implemented first as `Create plan` and `Pending plans` tabs under Organize,
  then expanded to `Create plan`, `Saved plans`, and `Pending batches`.
- The direct internal `staging` route remains safe during transition and points users back to Organize.
- The normal sidebar no longer exposes Plan Preview as a top-level item.
- No apply/move/delete/quarantine controls are exposed.
- Pending batch data is summarized with friendly labels; raw internal folder IDs are hidden behind technical details.
- Generated organization plans can now be saved as draft preview records through the backend-owned ApplyPlan builder and viewed in `Saved plans`.
- Saved draft plans can now show read-only validation/conflict preview results
  through `preview_apply_plan_validation`; `canProceedToConfirmation=false`
  remains the visible boundary.
- Inbox now owns detailed imported/downloaded batch review. Organize `Pending batches` should hand batch inspection back to Inbox and stay focused on preview plan work.

Current M2 foundation note:

- `StagingPlan` v1 is defined as a preview-only contract with `wouldTouchFiles=false`.
- `get_staging_preview_plan` is read-only and can describe current app-local staged folders as review items.
- The direct Plan Preview route can show this plan, but it still does not apply, move, delete, clean up, quarantine, or auto-sort files.
- Per-file organization suggestions remain future Auto Sorting Suggested Plan work.

Current M3 rules note:

- `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md` defines the approved evidence levels, destination buckets, do-not-move rules, generator scope, and future UI expectations for Auto Sorting Suggested Plans.
- `generate_sorting_preview_plan` now builds preview-only `StagingPlan` items for selected Library files or bounded Library folder scopes, not through the legacy Organize apply path.
- Generated plan items carry source signals, blocked reasons, buckets, confidence labels, current root, and `wouldTouchFiles=false`.
- The existing Organize route now shows the first preview-only plan review UI for generated plans.
- The visible Organize route no longer calls the legacy preview/apply/snapshot APIs.
- Auto Sorting remains preview-only until the Apply Safety Contract exists.

Current M4 safety-contract note:

- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md` defines what must be true before any future Apply workflow can change files.
- Inbox, Organize, Pending Plans, and the direct Plan Preview route remain review/preview-only until Apply has exact per-file preview, confirmation, backup/restore, path validation, conflict handling, recoverable errors, per-file result logs, tests, and proof.
- Existing internal mutating commands remain implementation history and are not normal user-facing workflows.
- `docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md` defines the next safety gate for saved draft plans: future validation must explain stale sources, missing roots, unsafe destinations, destination conflicts, review-only blockers, and backup/restore requirements before any confirmation work can be considered. No validation command or Apply UI exists yet.

Current integration-contract note:

- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md` defines how future SimSuite systems must reuse existing scanner, file-inspector, Library, duplicate, update, review, Inbox, Organize, preview-plan, and safety evidence.
- New navigation or workflow surfaces should not create parallel parsing, duplicate, update-source, folder, review, AI, or file-action truth when an existing backend system already owns that evidence.
- Future implementation reports for trust-sensitive systems should name the existing systems reused and explain any new data or logic added.

Current naming note:

- User-facing `Staging` language has been renamed to `Plan Preview` / `Pending Plans` in the visible route and navigation.
- Internal code and backend names such as `StagingScreen`, `StagingPlan`, `get_staging_areas`, and `get_staging_preview_plan` remain stable for now.
- Plan Preview is now folded into Organize as saved draft preview plans plus a
  pending-batch handoff area for normal navigation.
- The direct internal route remains available for compatibility and does not add any file-changing workflow.

### Phase 3 - Fold Creators And Types Into Library Lenses

- Preserve the existing creator/type browsing value.
- Move entry points into Library filters/lenses.
- Remove top-level exposure only after parity and proof.

### Phase 4 - Decide Duplicates Top-Level Vs Library/Review Workbench

- Keep deterministic duplicate language intact.
- Preserve exact duplicate vs review-only separation.
- Decide whether Creator mode still needs top-level Duplicates.

### Phase 5 - Move Guide Toward Help/Settings

- Keep help content reachable.
- Reduce sidebar action clutter only after Settings/Help replacement exists.

## Safety Plan

- No file-changing workflows are added by this planning sprint.
- Plan Preview remains preview-only.
- Organize is not Auto Sorting yet.
- Auto Sorting must start as a suggested plan only.
- Any future apply/move action requires the Apply Safety Contract: preview, explicit user confirmation, backup/restore support, path validation, destination conflict handling, recoverable errors, per-file result logs, tests, and desktop proof.
- AI may explain or suggest, but it must not decide broken/safe/delete/dependency/update truth.

## Linear Issue Plan

Linear project: [SimSuite - Safe Automation Roadmap](https://linear.app/veliveli/project/simsuite-safe-automation-roadmap-b038fd0f2cdb)

Team: Veliveli

Milestones:

- M0 - Project Control & Roadmap
- M1 - Navigation & Workflow Simplification
- M2 - Staging Preview Plan Foundation
- M3 - Auto Sorting Suggested Plans
- M4 - Apply Safety Contract
- M5 - Updates Provider Onboarding
- M6 - AI-Assisted Suggestions
- M7 - Real-Library Validation & Beta Readiness
- M8 - Technical Debt & Proof Infrastructure

Label groups created:

- `area:*`: library, inbox, staging, organize, updates, duplicates, review, navigation, settings, proof, docs, backend, frontend, trust.
- `type:*`: audit, implementation, proof, docs, test, refactor, bug, research.
- `trust:*`: informational, evidence-backed, review-only, suggested-plan, file-changing-blocked, requires-confirmation, requires-backup, ai-boundary.
- `P0`, `P1`, `P2`, `P3`.

Status customization was not changed in this sprint. Created issues use the existing Linear `Backlog` status.

| Issue | Milestone | Priority | Labels | Goal | User outcome | Scope | Out of scope | Acceptance criteria | Validation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [VEL-5](https://linear.app/veliveli/issue/VEL-5/m0-1-create-linear-project-and-labels) Create Linear project and labels | M0 | P0 | area:docs, area:trust, type:docs, trust:informational | Create Linear project, milestones, labels, and first issue set. | Roadmap work is tracked before automation starts. | Linear setup. | App code and feature work. | Project, milestones, labels, and issues exist. | List Linear project/issues and record in report. |
| [VEL-6](https://linear.app/veliveli/issue/VEL-6/m0-2-current-repodocs-source-of-truth-audit) Current repo/docs source-of-truth audit | M0 | P0 | area:docs, area:trust, type:audit, trust:informational | Map current docs and reports. | Future sessions start from the right files. | Repo docs and recent reports. | Runtime changes. | Source-of-truth map exists. | Docs review and requested checks. |
| [VEL-7](https://linear.app/veliveli/issue/VEL-7/m1-1-full-route-inventory) Full route inventory | M1 | P0 | area:navigation, area:frontend, type:audit, trust:informational | Classify every route/action. | Navigation can be simplified deliberately. | Route inventory. | Route changes. | Each route has purpose, data source, visibility, trust level, and recommendation. | Typecheck/unit/build for planning sprint. |
| [VEL-8](https://linear.app/veliveli/issue/VEL-8/m1-2-inbox-vs-staging-vs-organize-decision) Inbox vs Staging vs Organize decision | M1 | P0 | area:navigation, area:inbox, area:staging, area:organize, type:audit, trust:suggested-plan | Decide workflow ownership. | Intake, plan preview, and organization planning are not confused. | Product boundary decision. | Auto Sorting or file movement. | Clear ownership decision exists. | Docs review and requested checks. |
| [VEL-9](https://linear.app/veliveli/issue/VEL-9/m1-3-sidebar-simplification-proposal) Sidebar simplification proposal | M1 | P1 | area:navigation, area:frontend, type:docs, trust:informational | Propose future sidebar by mode. | App can become less crowded later. | Mode model and phases. | Sidebar implementation. | Future sidebar model is documented. | Docs review. |
| [VEL-10](https://linear.app/veliveli/issue/VEL-10/m1-4-fold-creators-and-types-into-library) Fold Creators and Types into Library | M1 | P1 | area:navigation, area:library, area:frontend, type:refactor, trust:informational | Plan Creator/Type lenses. | Users keep filtering power with fewer pages. | Migration plan. | Immediate route removal. | Parity-preserving plan exists. | Later proof when implemented. |
| [VEL-11](https://linear.app/veliveli/issue/VEL-11/m1-5-fold-staging-into-organize) Fold Staging into Organize | M1 | P1 | area:navigation, area:staging, area:organize, type:refactor, trust:suggested-plan, trust:file-changing-blocked | Plan Staging as Organize preview/plans. | Proposed changes are reviewed in one place. | Fold plan. | Apply/move/delete/quarantine. | Route remains safe during migration. | Later proof when implemented. |
| [VEL-12](https://linear.app/veliveli/issue/VEL-12/m2-1-define-stagingplan-model) Define StagingPlan model | M2 | P0 | area:staging, area:backend, area:frontend, type:docs, trust:suggested-plan, trust:file-changing-blocked | Define preview-only plan contract. | Users later see proposed changes with reasons and caveats. | Model fields and evidence levels. | Command/apply implementation. | Contract has no file-changing default. | Docs/API review. |
| [VEL-13](https://linear.app/veliveli/issue/VEL-13/m2-2-add-preview-only-staging-plan-command) Add preview-only staging plan command | M2 | P1 | area:staging, area:backend, type:implementation, trust:suggested-plan, trust:file-changing-blocked | Add read-only backend preview command. | Real plan data can be previewed without file changes. | Command/tests. | File movement/apply. | Returns bounded preview data and `would_touch_files=false`. | Rust/API/build/proof if consumed by UI. |
| [VEL-14](https://linear.app/veliveli/issue/VEL-14/m2-3-staging-plan-ui) Staging plan UI | M2 | P1 | area:staging, area:organize, area:frontend, type:implementation, trust:suggested-plan | Render preview-only plan UI. | Users can review suggestions without file changes. | Plan UI states. | Enabled apply controls. | No file-changing action is enabled. | Typecheck, unit, build, desktop proof. |
| [VEL-15](https://linear.app/veliveli/issue/VEL-15/m3-1-sorting-rules-audit) Sorting rules audit | M3 | P1 | area:organize, area:library, type:audit, trust:review-only, trust:suggested-plan | Audit safe sorting signals. | Suggestions are explainable and cautious. | Sorting signal table. | Auto Sorting implementation. | Signals are classified by evidence level. | Docs review. |
| [VEL-16](https://linear.app/veliveli/issue/VEL-16/m3-2-suggested-plan-generator-v1) Suggested plan generator v1 | M3 | P1 | area:organize, area:staging, area:backend, type:implementation, trust:suggested-plan, trust:file-changing-blocked | Generate preview-only organization plans. | Users can see proposed organization without file movement. | Bounded deterministic generator. | Apply, cleanup, AI decisions. | Stable suggestions with reasons/caveats and no file changes. | Rust/API/build/proof if UI consumes it. |
| [VEL-17](https://linear.app/veliveli/issue/VEL-17/m3-3-organize-plan-review-ui) Organize plan review UI | M3 | P1 | area:organize, area:staging, area:frontend, type:implementation, trust:suggested-plan | Show suggested plans in Organize. | Users review reasons before deciding anything. | Review UI. | Apply/move/delete/quarantine. | Review-first copy and no enabled file-changing action. | Typecheck, unit, build, desktop proof/smoke. |
| [VEL-18](https://linear.app/veliveli/issue/VEL-18/m4-1-backuprestore-design) Backup/restore design | M4 | P0 | area:backend, area:trust, type:docs, trust:requires-backup, trust:requires-confirmation, trust:file-changing-blocked | Design recovery contract. | Future file changes have a restore path. | Backup/restore design. | Real backup implementation. | Design covers data, flow, failures, and tests. | Docs review. |
| [VEL-19](https://linear.app/veliveli/issue/VEL-19/m4-2-dry-runapply-split) Dry-run/apply split | M4 | P0 | area:backend, area:staging, area:trust, type:docs, trust:requires-confirmation, trust:requires-backup, trust:file-changing-blocked | Define dry-run vs apply. | Users see exactly what would happen before future apply. | Safety contract. | Apply implementation. | Contract describes blockers and confirmation. | Docs review. |
| [VEL-20](https://linear.app/veliveli/issue/VEL-20/m4-3-confirmed-apply-prototype) Confirmed apply prototype | M4 | P2 | area:backend, area:staging, area:frontend, area:trust, type:implementation, trust:requires-confirmation, trust:requires-backup, trust:file-changing-blocked | Prototype confirmed apply only after safety contract. | Future moves require confirmation and recovery. | Minimal prototype after blockers. | Delete/quarantine/update replacement. | No unconfirmed file changes. | Rust/unit/build/desktop/recovery proof. |
| [VEL-21](https://linear.app/veliveli/issue/VEL-21/m5-1-provider-architecture-audit) Provider architecture audit | M5 | P1 | area:updates, area:backend, type:audit, trust:evidence-backed, trust:review-only | Audit update provider architecture. | Update checks stay honest. | Supported checkers and extension points. | Scraping/auto replacement. | Provider contracts are documented. | Docs review. |
| [VEL-22](https://linear.app/veliveli/issue/VEL-22/m5-2-source-confidence-model-v2) Source confidence model v2 | M5 | P1 | area:updates, area:trust, area:backend, area:frontend, type:implementation, trust:evidence-backed, trust:review-only | Refine source confidence states. | Users can tell checker vs source vs reminder. | Model/API/UI if needed. | Latest/official/safe replacement claims. | Cautious states and tests. | Typecheck, unit, build, Rust/proof if touched. |
| [VEL-23](https://linear.app/veliveli/issue/VEL-23/m5-3-curseforge-official-api-planning) CurseForge official API planning | M5 | P2 | area:updates, area:backend, type:research, trust:evidence-backed, trust:review-only | Plan possible official API support. | Provider support is evaluated before runtime work. | API constraints/research. | Runtime matching/scraping. | Planning doc says whether/how support can be added. | Research notes with sources. |
| [VEL-24](https://linear.app/veliveli/issue/VEL-24/m6-1-ai-boundary-design) AI boundary design | M6 | P2 | area:trust, area:frontend, area:backend, type:docs, trust:ai-boundary, trust:review-only | Define AI assistance limits. | AI cannot decide safety or truth. | Boundary design. | AI runtime/classification. | Allowed/forbidden behavior is documented. | Docs review. |
| [VEL-25](https://linear.app/veliveli/issue/VEL-25/m6-2-mock-ai-organization-explanation) Mock AI organization explanation | M6 | P2 | area:organize, area:frontend, area:trust, type:implementation, trust:ai-boundary, trust:suggested-plan | Prototype mock explanations. | Users see explanation shape without AI decisions. | Static/mock UI after boundary design. | AI runtime or file changes. | Copy is non-authoritative and tested. | Typecheck, unit, build, proof if visible. |
| [VEL-26](https://linear.app/veliveli/issue/VEL-26/m7-1-sanitized-diagnostics-plan) Sanitized diagnostics plan | M7 | P2 | area:proof, area:library, area:trust, type:docs, trust:informational | Plan privacy-safe real-library diagnostics. | Real validation can protect user privacy. | Redaction and reporting plan. | Real scan. | Collectable vs redacted data is explicit. | Docs review. |
| [VEL-27](https://linear.app/veliveli/issue/VEL-27/m7-2-user-approved-real-scan) User-approved real scan | M7 | P2 | area:proof, area:library, area:trust, type:proof, trust:informational | Run approved sanitized real-library proof. | Synthetic proof is complemented by real data. | Opt-in sanitized validation. | Copying or committing private files. | Report records sanitized counts and gaps. | Diagnostics, tests/proof, git artifact check. |
| [VEL-28](https://linear.app/veliveli/issue/VEL-28/m8-1-rust-warning-cleanup-audit) Rust warning cleanup audit | M8 | P2 | area:backend, area:proof, type:audit, trust:informational | Audit Rust warning noise. | Backend work is easier to review. | Warning inventory. | Broad refactor. | Cleanup priorities are documented. | `cargo check` in follow-up. |
| [VEL-29](https://linear.app/veliveli/issue/VEL-29/m8-2-vite-chunk-warning-audit) Vite chunk warning audit | M8 | P2 | area:frontend, area:proof, type:audit, trust:informational | Audit Vite chunk warnings. | Build health stays manageable. | Build output and chunk boundaries. | Route redesign. | Recommendation says fix vs accept. | `npm run build` in follow-up. |
| [VEL-30](https://linear.app/veliveli/issue/VEL-30/m8-3-worktree-hygiene-cleanup) Worktree hygiene cleanup | M8 | P3 | area:docs, area:proof, type:docs, trust:informational | Plan cleanup of unrelated dirty artifacts. | Future diffs stay focused. | Generated and unrelated dirty files. | Destructive reset without approval. | Cleanup plan protects user work. | `git status --short` and diff review. |
