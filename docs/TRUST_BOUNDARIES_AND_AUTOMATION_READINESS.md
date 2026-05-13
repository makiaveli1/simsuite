# SimSuite Trust Boundaries and Automation Readiness

Date: 2026-05-13

This document is the standing trust policy for SimSuite. Future Library, Inbox, Updates, Duplicates, Staging, sorting, and AI prompts should use it before adding automation.

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

## Staging-Specific Readiness Rules

Staging is the safety bridge before future organization or file-changing workflows. The currently exposed Staging route must stay preview/readiness only unless a future sprint deliberately adds a Level 4 workflow with the full safety contract.

Current Staging may:

- list app-local staged folders.
- show file counts and sizes.
- explain that no files are changed from the Staging screen yet.
- describe future requirements for applying changes.

Current Staging must not expose enabled controls that:

- move files into Library.
- clear, remove, or delete staged folders.
- quarantine, disable, or replace files.
- claim a plan is safe.
- let AI decide a file action.

Before Staging can apply real file changes, it must have:

- a per-file preview plan.
- evidence and caveats for each suggested action.
- explicit user confirmation.
- backup or restore support.
- path validation for sources and destinations.
- duplicate destination handling.
- recoverable error handling.
- a per-file result log.
- unit tests and desktop proof for the full workflow.

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
| Auto sorting | Suggested plan only | Start with previews, staging, and confirmation; no automatic moves. |
| Mod updating | Review/check workflow only | No automatic download or replacement. |
| AI-assisted categorization | Suggestion only | AI can suggest; user and deterministic rules decide. |
| AI-assisted source suggestions | Suggestion only | AI cannot invent official proof. |
| Duplicate cleanup | Not allowed yet | Exact duplicate proof exists, but cleanup/delete needs backup/restore/product design. |
| Dependency detection | Research only | Current support-file guidance is not a general dependency graph. |
| Missing mesh detection | Not allowed yet | Needs deterministic Sims resource proof. |
| Safe-delete | Not allowed yet | Requires deterministic dependency/resource proof and recovery design. |
| Staging | Preview/readiness only in the current route | Can list staged folders, but file-changing controls stay disabled until Level 4 safety is implemented and proven. |
| Quarantine | Not allowed yet | Needs backup, restore, evidence model, and clear user confirmation. |

## Report Requirements

Every future Codex implementation report must include:

### What this means for the user

For trust-sensitive work, also include:

### Trust / safety boundary

These sections should use plain English and state what SimSuite still cannot honestly claim.
