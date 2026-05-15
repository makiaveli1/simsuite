# Existing Systems Integration Contract v1

Date: 2026-05-15

This document defines how future SimSuite systems must reuse existing indexed
evidence, typed APIs, and trust boundaries before adding new metadata,
parsing, classification, or file-action logic.

It does not implement Apply, file movement, cleanup, delete, quarantine,
replacement, auto-sorting, or AI decisions.

## 1. Purpose

SimSuite already knows a lot about Sims 4 Mods, CC, Tray files, downloads, and
review state. New systems should feel connected to that shared knowledge, not
like isolated tools with their own private facts.

Product principle:

Every new SimSuite system must first reuse existing scanner, file-inspector,
Library, duplicate, update, review, Inbox, Organize, preview-plan, and safety
systems before introducing new metadata, parsing, or decision logic.

If a sprint adds new data or logic, it must explain why the existing systems
were not enough.

ApplyPlan persistence planning now lives in
`docs/planning/APPLYPLAN_PERSISTENCE_AUDIT_V1.md`. That audit applies this
contract to future saved plans, blockers, evidence snapshots, validation and
conflict results, backup/restore references, and result logs. It is design-only:
no migration, runtime command, UI, or real Apply workflow exists yet.

Current implementation note: the DB-only ApplyPlan persistence foundation now
implements draft/preview storage for saved plan snapshots, items, signals, and
blockers. It reuses `StagingPlan` evidence and remains persistence-only. Result
logs, restore entries, visible saved-plan UI, and real Apply remain future work.

Current implementation note: the backend-owned ApplyPlan builder now reuses the
existing sorting preview generator and ApplyPlan persistence foundation to save
draft preview records. It does not introduce a second classifier, reparse
files, or expose file-changing actions.

Current implementation note: the first saved-plan review UI now lives inside
Organize. It reuses `build_apply_plan_from_staging_plan`,
`list_saved_apply_plans`, `get_apply_plan`, and `delete_draft_apply_plan`
instead of creating a frontend-only saved-plan store or a second preview model.
It remains review-only and does not expose file-changing actions.

Current planning note: validation/conflict preview design now lives in
`docs/planning/VALIDATION_CONFLICT_PREVIEW_V1.md`. Future validation must reuse
saved ApplyPlan snapshots, Library identity/current paths, scanner-owned roots,
duplicate/review/update context, blocker/signal snapshots, and the Apply Safety
Contract before adding any new validation logic.

Current implementation note: `preview_apply_plan_validation` now implements the
first read-only validation preview using saved ApplyPlan records, item
snapshots, blocker/signal rows, Library file identity/current paths, and
configured Mods/Tray roots. It does not reparse packages, rescan folders,
reimplement duplicate truth, or create frontend-owned validation state.

Current implementation note: Organize `Saved plans` now reuses
`previewApplyPlanValidation` to display validation/conflict previews for saved
draft records. The UI does not create a parallel validation store, does not run
frontend-owned validation from raw plan data, and does not expose file-changing
actions.

## 2. Existing Evidence Inventory

| Evidence / data | Source system | Where stored or returned | Current reliability level | Current consumers | Future consumers | Do not duplicate rule | Trust boundary |
| --- | --- | --- | --- | --- | --- | --- | --- |
| File identity | Scanner and Library index | `files.id`, path, filename, extension, kind, source fields; Library rows and file detail | Deterministic when indexed | Library, detail, Review, Duplicates, Organize | ApplyPlan, AI explanations, saved plans | Do not create separate file identity lists outside indexed rows unless they are app-local intake records. | Identity does not prove safety, dependency, or update state. |
| Full hash | Scanner duplicate-candidate hashing | `files.hash`; duplicate detector inputs | Deterministic when present and non-empty | Duplicate detector, Library duplicate flags, detail | ApplyPlan blockers, duplicate review, AI explanations | Do not hash ad hoc in UI or feature code; use scanner/indexed hash state. | Matching hash can prove same file contents, not safe delete. |
| Package content fingerprint | File inspector during scan | `files.content_fingerprint` and related version/type metadata | Deterministic when successfully populated | Duplicate detector, Library backend map | Duplicate review, Apply blockers, future analysis | Do not reimplement DBPF fingerprinting outside scanner/file inspector. | Fingerprint failures are metadata, not proof of broken content. |
| Script content fingerprint | File inspector during scan | `files.content_fingerprint` and script fingerprint metadata | Deterministic when successfully populated | Duplicate detector, Library backend map | Duplicate review, Apply blockers | Do not inspect `.ts4script` archives in UI render paths. | Matching script fingerprint can support duplicate proof, not compatibility claims. |
| File size/date | Scanner and filesystem metadata | `files.size`, `files.modified_at`; Library rows/detail | Deterministic when indexed | Library, Duplicates, detail, summaries | ApplyPlan validation, review sorting | Do not rescan size/date in frontend components. | Size/date can explain current state, not version truth by itself. |
| Source root | Scanner, Downloads/Inbox watcher, settings | `source_location`, configured Mods/Tray/Downloads roots | Deterministic when configured/indexed | Library, Inbox, Organize, Plan Preview | ApplyPlan path validation, AI explanations | Do not infer roots from raw strings when typed source fields exist. | Source root does not prove whether a file should move. |
| Real folder metadata | Scanner | `library_folders`; folder tree metadata | Deterministic scan-time fact | Library folder tree, Open Folder | Organize, ApplyPlan validation | Do not create separate folder truth outside scanner-owned folder metadata. | Folder metadata is scan-time state, not live watcher truth. |
| Folder tree | Library index | `get_folder_tree_metadata` | Deterministic from indexed folders and files | Library folder view | Organize folder scope, AI explanations | Do not broad-load all files to rebuild folder trees in UI. | Empty folders require scan/rescan before old state appears. |
| Direct folder files | Library index | `list_library_folder_files` | Deterministic from indexed rows and scoped SQL filters | Library folder view, Organize scope | ApplyPlan builder, saved plans | Do not replace with unbounded whole-Library filtering. | Results are indexed state, not live filesystem watches. |
| Package metadata | File inspector and scanner | `files.insights`, kind/subtype/category fields, detail | Evidence-backed cue | Library detail, Review, Organize plan generator | Auto Sorting improvements, AI explanations | Do not parse `.package` files in UI render paths. | Metadata can suggest category/review; it does not prove safety. |
| Script metadata | File inspector and scanner | kind/subtype, warnings, fingerprint metadata | Evidence-backed cue | Library, Review, Organize plan generator | Apply blockers, AI explanations | Do not add separate script archive parsing outside inspector/scanner. | Script placement needs caution; do not claim broken script. |
| Tray metadata | File inspector/scanner classification | source and extension/kind fields, detail | Evidence-backed to deterministic for source/extension | Library, Organize plan generator | Tray-specific review and Apply validation | Do not invent Tray grouping outside indexed evidence. | Tray grouping is not dependency or safety proof. |
| Parser warnings | File inspector/scanner/rule engine | parser warning fields, review queue, detail | Evidence-backed cue | Review, Library problem signals, Organize generator | Apply blockers, AI explanations | Do not create new warning vocabularies without mapping to review reasons. | Warning means manual review, not broken content. |
| Inspection warnings | File inspector/scanner | safety notes, inspection failure state, review queue | Evidence-backed cue | Review, Library detail, Organize generator | Apply blockers, AI explanations | Do not hide inspection failures behind confident categories. | Failed inspection routes to review, not Apply. |
| Preview/thumbnail state | File inspector and Library detail hydration | `files.insights`; preview diagnostics; detail preview | Evidence-backed cue | Library grid/detail, preview diagnostics | Review, AI explanations | Do not parse thumbnails during normal UI browsing. | No preview does not mean broken content. |
| Duplicate proof | Duplicate detector | `duplicates`, duplicate APIs, exact duplicate flags | Deterministic only for validated exact rows | Duplicates, Library, detail | Apply blockers, duplicate review, AI explanations | Do not reimplement duplicate truth outside duplicate detector. | Exact duplicate proof does not authorize cleanup/delete. |
| Name match | Duplicate detector comparison rows | `list_duplicate_pairs` comparison/review rows | Heuristic | Duplicates review | AI explanations, review sorting | Do not label name matches as duplicates. | Name match is review-only, not duplicate proof. |
| Version review | Duplicate detector comparison rows | `list_duplicate_pairs` version-review rows | Heuristic | Duplicates review | Updates review, AI explanations | Do not infer outdated/latest from version-like names alone. | Version review is not update truth. |
| Same pack | Bundle detector | `bundles`, bundle fields, peer counts | Heuristic grouping hint | Library relationships, Organize generator | AI explanations, review grouping | Do not treat same pack as dependency proof. | Same pack is related context only. |
| Same folder | Library index relationship counts | folder peer counts, folder APIs | Heuristic grouping hint | Library, Organize generator | AI explanations, review grouping | Do not infer dependency or safety from folder proximity. | Same folder is context only. |
| Update/watch state | Updates and watch polling | `content_watch_sources`, `content_watch_results`, update APIs | Evidence-backed or review-only by provider state | Updates, Library update cues | AI explanations, Apply blockers for replacement work | Do not create update-source truth outside Updates/watch systems. | No generic scraping or official-source claim. |
| No update source | Updates/watch state | watch setup lists, Library cues | Deterministic absence of saved source | Updates, Library | AI explanations, review planning | Do not call a file outdated because no source is saved. | Missing source means setup/review, not outdated. |
| Review queue membership | Rule engine/scanner | `review_queue`, `get_review_queue`, problem signals | Evidence-backed review state | Review, Library, Organize generator | Apply blockers, AI explanations | Do not create separate review queues per feature. | Review means manual review, not broken content. |
| Creator/category/type/subtype metadata | Scanner, seed data, user metadata | file rows, facets, detail, creator/category audit APIs | Evidence-backed to heuristic depending confidence | Library filters, detail, Organize generator | Auto Sorting, AI explanations | Do not add hidden category classifiers without mapping to existing metadata. | Missing or weak metadata remains a caveat. |
| StagingPlan | Preview-plan foundation | Rust/TypeScript models and preview commands | Level 3 suggested plan | Organize, direct Plan Preview route | Saved plans, ApplyPlan builder | Do not create a second preview-plan model for organization suggestions. | `wouldTouchFiles=false`; not Apply-ready. |
| Sorting preview plan | Rule engine sorting plan generator | `generate_sorting_preview_plan` returning `StagingPlan` | Suggested plan using approved evidence | Organize Create plan | Saved plans, ApplyPlan builder, AI explanations | Do not call legacy Organize apply paths for preview generation. | Suggested destinations are preview strings only. |
| Inbox intake batch state | Downloads watcher / Inbox backend | Downloads inbox APIs, app-local batch metadata | Review/intake state | Inbox, Organize handoff | Saved intake review, future plan seeds | Do not present raw staging/download IDs as organization plans. | Intake review does not change files. |
| Future ApplyPlan | Not implemented; Apply contract only | Proposed in `APPLY_SAFETY_CONTRACT_V1.md` | Future Level 4 contract | None yet | Confirmed Apply only after safety validation | Do not create direct UI file actions without ApplyPlan. | Only future bridge to confirmed file-changing work. |

## 3. System Ownership Map

- Scanner owns indexing facts: file identity, source root, size/date, hash where
  available, real folder metadata, parser warnings, and scan-time insights.
- File inspector owns deep `.package`, `.ts4script`, and Tray clues, including
  DBPF metadata, script archive clues, content fingerprints, parser warnings,
  and selected-file preview hydration.
- Library index owns serving indexed facts to UI and workflows: paged rows,
  facets, folder tree metadata, direct folder files, file detail, relationship
  counts, review signals, duplicate flags, and update cues.
- File detail owns deeper evidence presentation for one selected file.
- Duplicate detector owns duplicate/comparison truth. Exact duplicate rows can
  support same-file-content proof; name/version rows remain review comparisons.
- Bundle detector owns same-pack grouping only. Same pack is not dependency
  proof.
- Updates/watch owns update source configuration, supported checker results,
  reminder-only source state, failed checks, and no-source states.
- Review owns manual review queue signals and review reasons.
- Inbox owns new/downloaded/imported intake review before content becomes part
  of normal Library or Organize planning.
- Organize owns preview organization planning, generated suggested plans, saved
  draft preview plan review, and batch handoff notes.
- Plan Preview/Pending Plans owns compatibility preview/pending work only and
  must stay preview-only.
- ApplyPlan is future work and is the only allowed confirmed file-changing
  contract after safety validation, backup/restore, path checks, confirmation,
  and result logging exist.

## 4. New System Checklist

Every future system must answer these questions before implementation:

- Which existing data does this need?
- Which existing command, API, model, or database table already returns it?
- Is each evidence source deterministic, evidence-backed, heuristic,
  review-only, or future work?
- Does this duplicate scanner, inspector, duplicate, update, review, Inbox,
  Organize, or preview-plan logic?
- Can this reuse FileDetail, Library row fields, Review queue rows,
  DuplicatePair data, Watch state, Inbox intake state, or StagingPlan?
- Does this truly need a new backend field, or can it use existing indexed
  insights?
- What user-facing claim is allowed by the evidence level?
- What must remain a caveat?
- Does this touch real files, or could it lead to a future file-changing action?
- If it could lead to a file action, how does it connect to the Apply Safety
  Contract instead of direct UI commands?
- What tests prove the integration reuses existing systems?

## 5. Anti-Duplication Rules

- Do not parse `.package` files in UI render paths.
- Do not reimplement DBPF parsing outside file inspector/scanner.
- Do not inspect `.ts4script` archives in UI render paths.
- Do not reimplement duplicate truth outside duplicate detector.
- Do not create new update-source truth outside Updates/watch systems.
- Do not create separate folder truth outside scanner-owned `library_folders`
  and folder metadata APIs.
- Do not create AI classifications that bypass indexed evidence.
- Do not create Apply/action logic directly from UI without ApplyPlan.
- Do not create new `safe`, `fixed`, `broken`, `official`, `outdated`, or
  dependency labels outside trust-boundary rules.
- Do not create a second organization preview model when `StagingPlan` already
  represents preview-only plans.

## 6. Future Feature Integration Requirements

### ApplyPlan Persistence

Must use:

- `StagingPlan` or sorting preview plan as source.
- Library file identity and current paths.
- evidence levels, source signals, and blocked reasons.
- Apply Safety Contract rules.
- snapshot/restore prior art only as building blocks.

Must not invent:

- new unreviewed file lists.
- weak heuristic Apply items.
- direct move-engine calls from UI.
- hidden file-action state outside an ApplyPlan/result-log model.

### ApplyPlan Builder

Must use:

- `StagingPlan` items.
- Library file detail and indexed evidence.
- duplicate, review, and update states as blockers or caveats.
- path validation and blocked-item filtering from the Apply Safety Contract.

Must not:

- apply review-only or heuristic-only suggestions.
- build destination paths from UI-only strings.
- skip conflict handling or backup/restore requirements.

### Auto Sorting Future Improvements

Must use:

- scanner metadata.
- file inspector metadata.
- Library categories/subtypes.
- review queue state.
- duplicate proof.
- update/watch caveats.
- `docs/planning/AUTO_SORTING_RULES_AUDIT_V1.md`.

Must not:

- create a parallel parser or classifier.
- claim strong moves from filename-only clues.
- create Apply-ready output.

### AI Assistance

Must use:

- existing evidence as input.
- trust-boundary language.
- citations or explanations tied back to visible evidence.
- preview-only outputs unless a future safety contract explicitly allows more.

Must not:

- decide file actions.
- invent official sources.
- claim broken content, dependency truth, missing mesh truth, safe delete, or
  safe replacement.

### Updates Provider Work

Must use:

- watch source tables.
- provider/source boundaries.
- supported checker results where available.
- official APIs only when provider work is implemented.

Must not:

- use generic scraping as update proof.
- claim a source is official without provider evidence.
- trigger automatic download or replacement.

### Duplicates Cleanup Future

Must use:

- exact duplicate proof from duplicate detector.
- Apply Safety Contract requirements.
- backup/restore and result logs.
- explicit user confirmation.

Must not:

- use name/version/same-pack/same-folder review rows as cleanup proof.
- delete, quarantine, or replace files.
- claim a duplicate can be safely removed.

## 7. Required Final-Report Addition

Every future implementation report for a system that adds data, metadata,
classification, suggestions, preview plans, AI assistance, provider logic,
duplicate handling, review logic, or file-action preparation must include:

### Existing systems reused

[List existing scanner/indexer/API/review/update/duplicate/preview systems used.]

### New data or logic added

[Explain why it was necessary and why existing systems were not enough.]

If the sprint adds no new data or logic, the report should say that clearly.

## 8. Testing / Proof Expectations

- Mapper/reuse logic needs unit tests showing it consumes existing models or
  API responses.
- API contract changes need integration or command-registration tests where the
  repo has that pattern.
- Scanner/indexer changes need backend tests and, when practical, fixture proof.
- Visible workflow changes need desktop proof or browser proof matching the
  repo's proof lane.
- Large-library query changes need bounded or stress proof.
- Trust-sensitive copy changes need the trust-boundary copy guard.
- Any future file-changing preparation must reference the Apply Safety Contract
  and must not expose real Apply until the Level 4 requirements are implemented
  and proven.
