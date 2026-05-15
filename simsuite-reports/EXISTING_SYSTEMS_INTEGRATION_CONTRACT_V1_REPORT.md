# Existing Systems Integration Contract v1 Report

Date: 2026-05-15

Branch: `codex/existing-systems-integration-contract-v1`

## What Was Audited

This sprint audited the current source-of-truth docs and integration surfaces:

- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `simsuite-reports/APPLY_SAFETY_CONTRACT_DESIGN_V1_REPORT.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `src-tauri/src/models.rs`
- `src-tauri/src/commands/mod.rs`
- `src/lib/types.ts`
- `src/lib/api.ts`
- `src/trustBoundaryCopy.test.ts`

The audit confirmed that SimSuite already has reusable evidence paths for
scanner/indexer facts, file inspection clues, Library APIs, duplicate proof,
update/watch state, Review signals, Inbox intake data, Organize preview plans,
`StagingPlan`, and the future Apply safety boundary.

## Integration Principle

Every new SimSuite system must reuse existing indexed evidence, typed APIs,
review/update/duplicate/preview models, and trust boundaries before introducing
new metadata, parsing, classification, or decision logic.

If a sprint adds new data or logic, it must explain why existing systems were
not enough.

## Existing Systems Mapped

- **Scanner**: file identity, source root, size/date, hashes, real folder
  metadata, parser warnings, and scan-time insights.
- **File inspector**: package/script/Tray clues, DBPF metadata, script archive
  clues, fingerprints, parser warnings, inspection warnings, and selected-file
  preview hydration.
- **Library index**: paged rows, facets, folder metadata, direct folder files,
  file detail, duplicate flags, update cues, and review signals.
- **Duplicate detector**: exact duplicate/comparison truth. Name/version rows
  remain review comparisons.
- **Bundle detector**: same-pack hints only.
- **Updates/watch**: saved source state, reminder-only state, supported checker
  results, failed checks, and no-source states.
- **Review**: manual review queue membership and reasons.
- **Inbox**: new/downloaded/imported intake review.
- **Organize**: preview organization planning and generated suggested plans.
- **Plan Preview/Pending Plans**: preview/pending work only.
- **ApplyPlan future**: the only allowed bridge to confirmed file-changing work
  after safety validation exists.

## What Changed

- Added `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`.
- Added this sprint report.
- Updated trust, navigation, backend map, Apply contract, handoff, and
  implementation status docs with current-state references.
- Added a lightweight guard assertion in `src/trustBoundaryCopy.test.ts` so the
  required contract phrases stay present.
- No Rust, schema, API, command, UI, Apply, file movement, cleanup, delete,
  quarantine, replacement, auto-sort, or AI behavior was added.

### What this means for the user

Future SimSuite features should feel more connected and less repetitive. When
SimSuite already knows something about a file, folder, duplicate, update source,
review issue, intake batch, or preview plan, future work is now expected to use
that existing knowledge instead of starting over.

No files are changed by this sprint.

### Trust / safety boundary

This sprint adds documentation and guardrails only. It does not expose Apply,
move files, delete files, clean up folders, quarantine, replace, auto-sort, or
make AI decisions. Existing backend mutation commands remain internal and
unexposed by current visible workflows.

## Existing Systems Reused Requirement

Future implementation reports for trust-sensitive or data-producing systems must
include:

### Existing systems reused

[List existing scanner/indexer/API/review/update/duplicate/preview systems used.]

### New data or logic added

[Explain why it was necessary and why existing systems were not enough.]

This requirement is intended to stop future systems from quietly bypassing
scanner, inspector, Library, duplicate, update, review, Inbox, Organize,
preview-plan, or safety contracts.

## Files Changed

- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`: new durable
  integration contract.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: linked the integration contract from
  the backend source-of-truth map.
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: added integration rules
  and future report requirements.
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`: added a current
  integration-contract note for future workflow surfaces.
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`: added the existing-systems
  requirements that future ApplyPlan work must satisfy.
- `src/trustBoundaryCopy.test.ts`: added a lightweight contract-doc guard.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: updated top session
  notes.

## Tests

- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`28` files, `105` tests).
- `npm run build`: passed; existing Vite chunk-size warning remains.

## Desktop / Runtime Proof

Skipped by design unless visible route behavior changes. This sprint is
docs/test-guard work only.

## What Could Not Be Verified

- No runtime UI path changed, so no desktop proof was expected.
- No Rust/backend files changed, so Rust validation was not expected.
- The contract cannot prove future teams will follow it; it adds documentation
  and a lightweight guard test so future work has a clear standard.

## Linear Updates

Completed after validation:

- Commented on `VEL-6 - Current repo/docs source-of-truth audit` with the
  contract path, report path, validation results, and note that the contract
  links future ApplyPlan, Auto Sorting, Updates, Duplicates, and AI work back to
  existing evidence systems (`338df018-3648-4968-a324-276a1d25d5f3`).
- Did not create a duplicate issue.

## Recommended Next Sprint

ApplyPlan persistence audit or ApplyPlan builder design. Do not build real Apply
or move files yet.

## Docs Updated

- `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`
- `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`

## Unrelated Worktree Changes

Known unrelated dirty files must remain unstaged:

- `.cocoindex_code/*`
- `src/screens/HomeScreen.tsx`
- `src/styles/globals.css`
- pre-existing unrelated hunks in `SESSION_HANDOFF.md`
- pre-existing unrelated hunks in `docs/IMPLEMENTATION_STATUS.md`

## Commit

Commit hash is recorded in the final Codex report after commit.

## Final Honest Verdict

Verified: Existing Systems Integration Contract v1 is complete and ready to
guide future sprints.
