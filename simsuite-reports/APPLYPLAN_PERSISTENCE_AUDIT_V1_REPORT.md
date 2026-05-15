# ApplyPlan Persistence Audit v1 Report

Date: 2026-05-15

Branch: `codex/applyplan-persistence-audit-v1`

## What was audited

- `StagingPlan` / `StagingPlanItem` model fields in Rust and TypeScript.
- `generate_sorting_preview_plan` as the current read-only organization
  suggestion source.
- SQLite migration and schema-repair conventions in `database/migrations` and
  `src-tauri/src/database/mod.rs`.
- Snapshot/restore prior art in `snapshot_manager`, `snapshots`,
  `snapshot_items`, and move-engine restore helpers.
- Existing internal file-changing commands, including organization apply,
  Downloads/Inbox apply, guided apply, review-plan apply, reject/restore,
  staging commit/cleanup, and move-engine helpers.
- Current visible Inbox, Organize, Pending Plans, Plan Preview, Review,
  Duplicates, and Library trust-sensitive surfaces.
- Source-of-truth docs for Apply safety and existing systems integration.
- Linear M4 Apply safety issues.

## Persistence decision

No SQLite migration was added in this sprint.

The audit recommends designing the tables now and adding the migration later,
when the first read-only ApplyPlan builder or persistence command is
implemented. That future work should add both a migration entry and
`ensure_schema` compatibility repair.

## Existing systems reused

Future ApplyPlan persistence must reuse:

- `StagingPlan` preview data, while keeping it preview-only.
- `generate_sorting_preview_plan` for reviewed organization suggestions.
- Library file identity, file detail, indexed paths, source roots, and scan
  facts.
- scanner and file-inspector metadata, parser warnings, inspection warnings,
  and content fingerprints.
- duplicate detector truth for exact duplicates and review-only comparison rows.
- review queue membership and review reasons.
- update/watch state as context only.
- Inbox intake state for downloaded/imported batch origin.
- snapshot/restore prior art as building blocks only.
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`.
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`.

## New data or logic added

This sprint added documentation and a lightweight doc guard only.

No runtime model, SQLite table, migration, backend command, frontend route,
Apply UI, file movement, cleanup, delete, quarantine, replacement, auto-sort, or
AI decision was added.

## What changed

- Added `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md` as the source-of-truth
  persistence design.
- Added this sprint report.
- Updated current-state docs to link the persistence audit from Apply safety,
  existing systems integration, trust, navigation, backend map, handoff, and
  implementation status docs.
- Extended the trust-boundary unit test with a lightweight guard that the
  ApplyPlan persistence audit exists and contains required safety phrases.

## What this means for the user

Users still do not get an Apply button.

This work decides how SimSuite should safely remember reviewed plans, blockers,
future validation results, backup/restore references, and per-file result logs
later. The point is to make future file-changing work more careful before any
real file movement exists.

## Trust / safety boundary

This sprint is design-only.

No files changed. No Apply, move, delete, cleanup, quarantine, replacement,
auto-sort, AI decision, or mutating backend exposure was added. Existing
mutating backend commands remain internal and blocked from the current visible
Inbox/Organize/Plan Preview workflows.

## Proposed future schema

Design-only tables:

- `apply_plans`: one saved reviewed plan, including status, source plan
  metadata, confirmation/backup requirements, item counts, caveats, and
  timestamps.
- `apply_plan_items`: one row per future file action candidate, blocked item,
  or review-only item.
- `apply_plan_item_signals`: normalized source-signal/evidence snapshots for
  each item.
- `apply_plan_item_blockers`: blocked reasons, review-only reasons, validation
  failures, and conflict reasons.
- `apply_plan_results`: per-file execution/result-log entries for future Apply
  runs.
- `apply_plan_restore_entries`: restore/undo map entries scoped to one future
  Apply run and item.

## Proposed future commands

Designed but not implemented:

- `save_apply_plan_preview`: DB-only state change, no file movement.
- `list_saved_apply_plans`: read-only.
- `get_apply_plan`: read-only.
- `delete_draft_apply_plan`: DB-only state change.
- `validate_apply_plan`: read-only by default; DB-only only if validation output
  is explicitly saved.
- `build_apply_plan_from_staging_plan`: read-only draft or DB-only persistence,
  depending on future design.

All future commands must avoid move-engine apply paths, existing mutating
staging/download commands, cleanup/reject/restore paths, shell operations, and
real file writes.

## Current UI exposure

Current visible UI remains preview/review-only:

- Inbox owns new/downloaded/imported intake review and blocks visible
  file-changing controls.
- Organize owns preview organization planning and calls the read-only generator.
- Pending Plans/Plan Preview remain preview-only compatibility surfaces.
- Duplicates and Library surfaces do not claim cleanup, safe delete, safe move,
  or safe replacement.

## Files changed

- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`: new source-of-truth
  persistence design.
- `simsuite-reports/APPLYPLAN_PERSISTENCE_AUDIT_V1_REPORT.md`: sprint report.
- `src/trustBoundaryCopy.test.ts`: lightweight guard for the new audit doc.
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`: links future persistence design
  to the Apply safety contract.
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`: links the
  ApplyPlan persistence audit as the detailed future persistence source.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: notes the persistence
  design as a planning-only Level 4 prerequisite.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: records that ApplyPlan persistence is
  design-only and no migration/command exists yet.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`: keeps Organize/Inbox
  route ownership aligned with future saved-plan work.
- `SESSION_HANDOFF.md`: session baton-pass note.
- `docs/IMPLEMENTATION_STATUS.md`: current implementation status note.

## Tests

Passed:

- `npx tsc --noEmit`
- `npm run test:unit` (`28` files, `106` tests)
- `npm run build`

`npm run build` still reports the existing Vite chunk-size warning.

## Desktop/runtime proof

Desktop proof and smoke were skipped because this sprint did not change visible
route behavior.

## What could not be verified

- No runtime ApplyPlan persistence was verified because no runtime migration,
  command, API, or UI was added.
- Rust validation was not run because no Rust/backend files changed.
- Desktop proof/smoke were not run because there was no visible route behavior
  change.

## Linear updates

- Created `VEL-31` - Audit ApplyPlan persistence model - and marked it Done
  after validation passed.
- Commented on `VEL-18` with the backup/restore persistence implications.
- Commented on `VEL-19` with the dry-run/apply persistence split.
- Commented on `VEL-20` noting the confirmed apply prototype remains blocked
  until later ApplyPlan, validation, backup/restore, result-log, and
  confirmation work exists.

## Recommended next sprint

Read-only ApplyPlan persistence foundation or ApplyPlan builder design.

Do not build real Apply yet.

## Docs updated

- `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/APPLYPLAN_PERSISTENCE_AUDIT_V1_REPORT.md`

## Unrelated worktree changes

Known unrelated dirty files were left alone and must remain unstaged unless a
final partial-hunk stage is needed for this sprint's status/handoff notes:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated hunks in `SESSION_HANDOFF.md`
- pre-existing unrelated hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Pending.

## Final honest verdict

Pending validation.
