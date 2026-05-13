# Auto Sorting Rules Audit v1 Report

Date: 2026-05-13

Branch: `codex/auto-sorting-rules-audit-v1`

## Pre-Implementation Audit

### 1. StagingPlan foundation

- `StagingPlan` and `StagingPlanItem` exist in Rust and TypeScript.
- Current action kinds are `suggest_move`, `suggest_group`, `suggest_review`, and `no_action`.
- Current evidence levels are `deterministic`, `evidence_backed`, `heuristic`, and `review_only`.
- `wouldTouchFiles` is typed and returned as `false`.
- `get_staging_preview_plan` is read-only and currently returns folder-level review items only.
- The current contract is enough for a preview-only generator, but the next sprint should consider adding `leave_in_place`, `sourceSignals[]`, `blockedReasons[]`, and `bucket`.

### 2. Evidence-source audit

Audited evidence from:

- scanner fields: extension, source root, paths, depth, hashes, content fingerprints, warnings, and safety notes.
- file inspector: package DBPF metadata, script archive metadata, preview/thumbnail state, and fingerprint state.
- Library index: rows, folder metadata, duplicate flags, problem signals, relationship hints, watch/update cues, and detail fields.
- duplicate detector: exact duplicate proof, name match, version review, and package/script fingerprint exacts.
- bundle detector: same-pack grouping.
- rule engine / validator: legacy organization preview, candidate fields, path validator notes, collision checks, and script depth handling.
- content watch systems: update source states and checker limits.

### 3. Organization/move system audit

- `StagingPlan` is the safe preview-only contract.
- `get_staging_preview_plan` is read-only.
- Legacy `preview_organization` is read-only but returns the older `OrganizationPreview` shape.
- Legacy `apply_preview_organization`, move-engine apply paths, Downloads apply paths, guided install apply paths, and Staging commit/cleanup commands can move, restore, or delete app-managed files.
- The future generator should build against `StagingPlan`, not the legacy apply-ready organization path.

### 4. User experience impact plan

Users should eventually get suggested organization plans with clear reasons and caveats. This audit alone changes no app behavior and moves no files.

## What Was Audited

- Library evidence sources.
- Scanner/indexed metadata.
- Package/script parser outputs.
- Duplicate truth signals.
- Bundle/same-pack hints.
- Review queue and parser warning behavior.
- Update/watch states.
- StagingPlan contract.
- Legacy Organize/Move paths.
- Trust and navigation docs.
- Linear issues `VEL-15`, `VEL-16`, and `VEL-17`.

## Current Sorting Readiness

SimSuite has enough evidence to design a preview-only suggestion generator, but not enough safety infrastructure to move files. Strong signals can suggest review buckets or proposed destinations with caveats. Weak signals must stay review-only or leave-in-place.

Current Staging data is still folder-level. The next generator needs selected Library file or staged batch file-level data before it can return honest per-file `suggest_move` items.

## Sorting Rules Defined

Created `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md` with:

- Auto Sorting product definition.
- evidence source inventory.
- destination buckets: Script Mods, CAS, Build/Buy, Gameplay, Presets & Sliders, Overrides & Defaults, Tray, Needs Review, Unknown / Leave in place.
- action kinds: `suggest_move`, `suggest_group`, `suggest_review`, proposed `leave_in_place`, and `no_action`.
- evidence-level rules.
- do-not-move rules.
- per-file item requirements.
- v1 generator scope.
- future UI expectations.
- future test strategy.

## What This Means For The User

Nothing changes in the app today. This makes the future sorting feature safer because SimSuite now has a written rulebook for what it may suggest, what needs review, and what it must leave alone.

## Trust / Safety Boundary

No auto sorting was built. No files were moved, deleted, cleaned up, quarantined, replaced, or changed. AI is not deciding anything. The next implementation must remain preview-only and keep `wouldTouchFiles=false`.

## Linear Updates

- `VEL-15` was commented and moved to `Done` as the completed rules audit.
- `VEL-16` was commented as the next backend implementation candidate, still preview-only and no file movement.
- `VEL-17` was commented with the future UI expectations from the audit.

## Files Changed

- `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md` - new implementation-ready rules audit for future Auto Sorting Suggested Plans.
- `simsuite-reports/AUTO_SORTING_RULES_AUDIT_V1_REPORT.md` - sprint report.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md` - cross-link and Auto Sorting rules boundary.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md` - current M3 note.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md` - backend implications for future preview-only generator.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` - current sprint notes.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`23` files / `90` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

## Desktop / Runtime Proof

Skipped. This sprint changed docs/planning only and did not change route behavior, backend commands, schemas, or UI code.

## What Could Not Be Verified

- No Auto Sorting generator was verified because no generator was built.
- No desktop/runtime proof was run because runtime behavior did not change.
- No file movement behavior was verified because file movement is intentionally out of scope.

## Recommended Next Sprint

Suggested Plan Generator v1, still preview-only and no file movement.

## Docs Updated

- `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`
- `simsuite-reports/AUTO_SORTING_RULES_AUDIT_V1_REPORT.md`
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

Pending.

## Final Honest Verdict

Verified: Auto Sorting Rules Audit v1 is complete and ready for the preview-only generator sprint.
