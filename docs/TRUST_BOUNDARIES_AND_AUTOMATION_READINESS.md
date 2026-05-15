# SimSuite Trust Boundaries and Automation Readiness

Date: 2026-05-13

This document is the standing trust policy for SimSuite. Future Library, Inbox, Updates, Duplicates, Plan Preview/internal Staging, sorting, and AI prompts should use it before adding automation.

Navigation and workflow simplification planning now lives in `docs/planning/NAVIGATION_WORKFLOW_ARCHITECTURE_V1.md`. That plan is the source of truth for which pages should stay top-level, become Library lenses, fold into Organize, or move toward Help/Settings.

Auto Sorting rules planning now lives in `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`. That audit is the source of truth for which evidence signals may drive preview-only organization suggestions.

Apply safety planning now lives in `docs/planning/APPLY_SAFETY_CONTRACT_V1.md`. That contract is the source of truth for what must exist before SimSuite can expose any future file-changing Apply workflow.

Existing-systems integration planning now lives in `docs/planning/EXISTING_SYSTEMS_INTEGRATION_CONTRACT_V1.md`. That contract is the source of truth for how future systems must reuse scanner, file-inspector, Library, duplicate, update, review, Inbox, Organize, preview-plan, and safety evidence before adding new logic.

ApplyPlan persistence planning now lives in `docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`. That audit designs how future reviewed plans, blockers, evidence snapshots, validation/conflict results, backup/restore references, and result logs should be stored before any real Apply workflow is built.

The first ApplyPlan persistence foundation is now implemented as DB-only draft/preview storage. It can save, list, view, and soft-cancel preview snapshots, but it does not expose Apply, move files, create result logs, or provide restore execution.

## Why This Exists

Sims 4 players have good reason to distrust tools that claim they can automatically fix, quarantine, remove, or update mods without real Sims file-format evidence. SimSuite must stay clear about what it knows, what it only suspects, and what needs manual review.

SimSuite should help users understand and plan. It must not invent certainty.

## Evidence Levels

| Level | Meaning | Examples | User-facing language |
| --- | --- | --- | --- |
| Deterministic fact | SimSuite has direct, reproducible evidence. | Same non-empty file hash, real disk folder path, stored file size/date, exact configured path. | `Duplicate`, `Same file contents`, `0 files`, `Open folder`. |
| Evidence-backed cue | SimSuite has real evidence, but it does not prove the full claim. | Parser warning, DBPF metadata clue, supported watch checker result, known special-mod rule. | `Needs review`, `Update may be available`, `Could not inspect fully`. |
| Heuristic hint | SimSuite found a pattern that may help review. | Filename token, creator text, same folder, same pack, version-like string. | `Name match`, `Version review`, `Related hint`, `Same pack`, `Same folder`. |
| Review-only state | SimSuite cannot decide safely. | Weak metadata, conflicting clues, unsupported provider, failed check. | `Manual review needed`, `SimSuite has limited information here`. |
| Future work | The capability is not implemented or not proven. | Missing mesh detection, dependency graph, safe-delete proof, generic update replacement, AI verification. | Say it is not supported yet. |

## What SimSuite Can Know Today

- File identity: path, filename, extension, size, modified time, and full file hash when indexed.
- Exact duplicates: only when two real, distinct file rows have matching non-empty same-content proof.
- Folder truth: real Mods/Tray folder metadata, including empty folders, and real disk paths for Open Folder.
- Package inspection clues: DBPF metadata, parser warnings, inspection warnings, and selected-file preview hydration when available.
- Preview availability: indexed embedded/cache preview data, no-preview states, and aggregate preview diagnostics.
- Tray and script limitations: Tray/script previews remain future work; scan-time package/script content fingerprints may support exact duplicate proof only when a stored fingerprint exists.
- Update watch state: saved source, reminder-only source, supported exact checker result, provider-limited source, failed check, or no update source.
- Review signals: rule-backed or parser-backed reasons that something should remain visible for manual review.

## What SimSuite Must Not Claim Today

SimSuite must not claim:

- a mod or CC is broken unless there is deterministic proof and a documented evidence model.
- a mesh or dependency is missing.
- one file depends on, requires, or is used by another file, except for narrow seeded support-file guidance that remains review-first.
- a file is safe to delete, safe to replace, or confirmed safe.
- a generic creator page is an official source or latest-version proof.
- an update is definitely latest or definitely outdated.
- AI verified a result.
- the app automatically fixed, quarantined, removed, or updated mods.
- encrypted or unavailable files such as lastCrash contents were read unless the backend truly reads them.

## Automation Readiness Levels

| Level | Name | Allowed now? | Rule |
| --- | --- | --- | --- |
| 0 | Informational | Yes | Show facts and explain limits. |
| 1 | Evidence-backed cue | Yes | Show a cautious cue with evidence and caveats. |
| 2 | Review workflow | Yes | Route the user to compare, inspect, or review. |
| 3 | Suggested plan | Yes, with preview | Suggest organization or next steps, but do not change files automatically. |
| 4 | User-confirmed action | Limited | Requires explicit preview, user confirmation, backup/restore path, and recoverable errors. |
| 5 | Automated action | No for destructive file operations | No automatic delete, quarantine, replacement, broad update, or AI-decided file moves. |

## Action Rules

- File-moving features must start at Level 3 as suggested plans.
- Any Level 4 file action must show a preview first and require explicit confirmation.
- Any move/replace workflow must have backup or restore behavior before it is considered safe enough for broad use.
- Delete and quarantine workflows are not allowed until SimSuite has deterministic evidence, backup, restore, clear confirmation, and a product decision.
- Update replacement is not allowed until official/provider-safe checks, backup, rollback, and clear user confirmation exist.
- Safe-delete claims are forbidden until deterministic dependency/resource analysis exists and is tested.
- Provider work must respect provider/API policy and must not scrape generic pages as update proof.

## Apply Safety Contract Rules

Apply is not ready. SimSuite may show preview plans today, but it must not expose a real file-changing Apply workflow until the contract in `docs/planning/APPLY_SAFETY_CONTRACT_V1.md` is implemented and proven.

The persistence foundation from `codex/applyplan-persistence-foundation-v1` creates ApplyPlan draft/preview tables and DB-only commands, but it is not an Apply workflow. Future ApplyPlan execution must still be backed by preview, explicit confirmation, backup/restore, path validation, conflict handling, recoverable errors, and per-file result logs before any files can change.

Before any future Apply can touch files, SimSuite must have:

- an exact per-file preview.
- explicit user confirmation.
- backup or restore behavior.
- validated source and destination paths.
- destination conflict handling.
- blocked/review-only item filtering.
- recoverable error handling.
- a per-file result log.
- tests and desktop proof for the full workflow.

Future Apply must not delete, quarantine, replace, auto-update, apply AI-only suggestions, apply review-only suggestions, apply weak heuristic suggestions, or claim that a file is safe to move/delete/replace.

Existing backend mutating commands remain internal implementation history until a future ApplyPlan flow can safely replace or wrap them.

## Existing Systems Integration Rules

Future SimSuite systems must start from existing indexed evidence and typed API boundaries before adding new parsing, metadata, classification, or decision logic.

Current integration rules:

- use scanner/indexer rows for file identity, source roots, hashes, file size/date, and real folder metadata.
- use file inspector output for package, script, Tray, parser-warning, inspection-warning, thumbnail, and content-fingerprint evidence.
- use the Library index and file detail APIs for row, folder, detail, preview, duplicate, update, and review evidence.
- use the duplicate detector for duplicate truth; name/version/same-pack/same-folder cues stay review context.
- use Updates/watch systems for source state; do not create separate provider or official-source truth.
- use Inbox for downloaded/imported intake review, Organize for preview organization planning, and `StagingPlan` for preview-only suggested plans.
- use the future `ApplyPlan` contract, not direct UI actions, before any confirmed file-changing workflow.

Future reports for trust-sensitive systems must include `### Existing systems reused` and `### New data or logic added` so new logic cannot quietly bypass the existing evidence model.

## Plan Preview / Internal Staging Readiness Rules

Plan Preview is the user-facing safety bridge before future organization or file-changing workflows. It is currently backed by internal staging names such as `StagingPlan`, `StagingScreen`, `get_staging_areas`, and `get_staging_preview_plan`. Those internal names may remain for code stability, but visible copy should say `Plan Preview`, `Pending Plans`, or `Preview plan`.

The normal user-facing Plan Preview experience now lives inside Organize as the `Pending plans` tab. The direct internal Plan Preview route remains available for compatibility, but it must stay preview/readiness only unless a future sprint deliberately adds a Level 4 workflow with the full safety contract.

Current Plan Preview may:

- list app-local staged folders.
- show file counts and sizes.
- summarize internal pending batch data with friendly labels instead of exposing raw staging folder IDs as primary plan names.
- reveal internal IDs only as technical details.
- explain that no files are changed from Organize, Pending plans, or the direct Plan Preview route yet.
- describe future requirements for applying changes.
- return a preview-only `StagingPlan` with `wouldTouchFiles=false`.
- show folder-level review items with reasons and caveats when per-file organization suggestions do not exist yet.

Current Plan Preview must not expose enabled controls that:

- move files into Library.
- clear, remove, or delete staged folders.
- quarantine, disable, or replace files.
- claim a plan is safe.
- let AI decide a file action.

`StagingPlan` v1 is a Level 3 suggested-plan contract only. It may describe pending folders and future review needs, but it must not create destinations, claim a move is ready, or imply a plan can be applied until per-file preview, user confirmation, backup/restore, path validation, conflict handling, recoverable errors, and proof exist.

Current `Pending plans` copy should also be honest that generated organization plans are not saved yet. Imported/downloaded app-local batches may be summarized there for now, but they remain review/intake data and should not be presented as completed or apply-ready organization plans.

Detailed imported/downloaded batch review belongs in Inbox. Organize may link to Inbox for batch review, but it should not duplicate a full intake workflow or imply those batches are saved organization plans.

Before Plan Preview/internal Staging can apply real file changes, it must have:

- a per-file preview plan.
- evidence and caveats for each suggested action.
- explicit user confirmation.
- backup or restore support.
- path validation for sources and destinations.
- duplicate destination handling.
- recoverable error handling.
- a per-file result log.
- unit tests and desktop proof for the full workflow.

## Auto Sorting Suggested Plan Rules

Auto Sorting may start only as a Level 3 suggested plan. It may suggest destinations, groups, review routes, or leave-in-place decisions, but it must keep `wouldTouchFiles=false` until a separate Apply Safety Contract exists.

The first generator command, `generate_sorting_preview_plan`, is read-only. It returns preview-only `StagingPlan` data for selected Library files or a bounded Library folder scope. It must not call legacy Organize apply paths, Staging commit/cleanup commands, move-engine apply paths, shell operations, AI, delete, quarantine, or cleanup behavior.

The current Organize route may display generated `StagingPlan` results for review. It must keep the plan surface preview-only, show reasons and caveats, keep `wouldTouchFiles=false`, and avoid enabled Apply, move, cleanup, delete, quarantine, replacement, or automatic sorting controls.

Auto Sorting generator work must use the rules in `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`:

- deterministic and evidence-backed signals may suggest buckets with caveats.
- heuristic signals may support review-only suggestions, but they must not drive strong move suggestions alone.
- exact duplicates route to duplicate review, not cleanup or automatic movement.
- parser warnings, inspection failures, weak metadata, unsupported types, and conflicting clues route to review or leave-in-place.
- filename/version/folder/pack/family hints remain review-only unless backed by stronger evidence.
- AI must not decide the category, destination, safety, dependency, or update truth.
- suggested destination paths are preview strings only; SimSuite does not create folders or move files from this generator.

## Inbox Intake Review Rules

Inbox is the user-facing intake area for new downloads and imported batches. It may explain what arrived, show review lanes, show local evidence, and route users toward Library or Organize planning.

Current visible Inbox behavior must stay review-only:

- show `No files changed` or equivalent safety copy.
- avoid primary raw internal IDs as batch names.
- show technical details only as secondary/collapsed information when possible.
- provide safe next steps such as `Create preview plan`, `Open Organize`, and `Open Library`.
- avoid enabled Apply, Reject, move, cleanup, delete, quarantine, commit, or fix controls until a future safety contract exists.

Existing Downloads backend mutation commands and handlers are not removed by the current UI clarity work, but they must remain unexposed or blocked in the visible workflow until preview, confirmation, backup/restore, recoverable errors, and proof exist.

## AI Assistance Boundary

AI may help with:

- summarizing metadata and parser clues.
- suggesting categories or search terms.
- explaining why an item needs review.
- drafting an organization plan for the user to inspect.
- ranking review items by visible evidence.
- explaining what deterministic evidence exists or is missing.

AI must not:

- decide a file is broken.
- decide a file is safe to delete, replace, quarantine, or disable.
- decide a dependency or mesh is missing.
- bypass provider rules or invent source URLs.
- claim it read Sims files that SimSuite did not parse.
- execute file changes without deterministic guardrails and user confirmation.

## Automation Readiness Checklist

Before adding any workflow that moves, disables, replaces, quarantines, deletes, downloads, or auto-updates files, the sprint must answer:

- What exact evidence supports the action?
- Is the evidence deterministic, evidence-backed, heuristic, or review-only?
- Does the action touch real user files?
- Does it preserve a backup or restore point?
- Can it be undone?
- Does it show a preview first?
- Does it require explicit user confirmation?
- Does it avoid safe-delete and safe-replace claims?
- Does it avoid unsupported provider or scraping assumptions?
- Are errors recoverable?
- Are logs private and sanitized?
- Are tests and desktop proof included where relevant?
- Does the final report include `### What this means for the user`?
- If sorting, updating, duplicate handling, review, move/disable/delete, provider/source logic, or AI is involved, does the final report include `### Trust / safety boundary`?

## Future Feature Readiness

| Feature | Current readiness | Boundary |
| --- | --- | --- |
| Auto sorting | Suggested plan only | Start with previews, Plan Preview/internal Staging, and confirmation; no automatic moves. |
| Mod updating | Review/check workflow only | No automatic download or replacement. |
| AI-assisted categorization | Suggestion only | AI can suggest; user and deterministic rules decide. |
| AI-assisted source suggestions | Suggestion only | AI cannot invent official proof. |
| Duplicate cleanup | Not allowed yet | Exact duplicate proof exists, but cleanup/delete needs backup/restore/product design. |
| Dependency detection | Research only | Current support-file guidance is not a general dependency graph. |
| Missing mesh detection | Not allowed yet | Needs deterministic Sims resource proof. |
| Safe-delete | Not allowed yet | Requires deterministic dependency/resource proof and recovery design. |
| Plan Preview / internal Staging | Preview/readiness only in the current route | Can list pending/staged folders, but file-changing controls stay disabled until Level 4 safety is implemented and proven. |
| Quarantine | Not allowed yet | Needs backup, restore, evidence model, and clear user confirmation. |

## Report Requirements

Every future Codex implementation report must include:

### What this means for the user

For trust-sensitive work, also include:

### Trust / safety boundary

These sections should use plain English and state what SimSuite still cannot honestly claim.
