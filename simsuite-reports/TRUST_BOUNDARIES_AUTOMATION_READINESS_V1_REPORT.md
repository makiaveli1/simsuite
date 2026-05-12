# Trust Boundaries and Automation Readiness v1 Report

Date: 2026-05-12
Branch: codex/trust-boundaries-automation-readiness-v1

## Audit Note Before Implementation

### Worktree state

- Started from `codex/library-large-scale-backend-stress-v1`.
- Created branch `codex/trust-boundaries-automation-readiness-v1`.
- Pre-existing unrelated worktree changes were present in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, `SESSION_HANDOFF.md`, and `docs/IMPLEMENTATION_STATUS.md`.
- This sprint will not overwrite or stage unrelated Home/global CSS/generated changes.
- If `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md` are updated, only this sprint's new notes should be staged.

### 1. Current claim inventory

| Area | Current claim | Classification | Audit finding |
| --- | --- | --- | --- |
| Duplicate files | `Duplicate` / `Same file contents` | proven/deterministic | Safe today; backed by validated same-content hash proof. |
| Name/version matches | `Name match`, `Version review` | review-only | Safe today; no longer counted as duplicates. |
| Related files | `Related hint`, same pack/folder counts | heuristic | Safe if kept separate from dependency language. |
| Same pack | same bundle/pack clue | heuristic | Safe as a related hint only. |
| Same folder | same-folder peer count | heuristic | Safe as context only. |
| Update status | `Update may be available`, `Could not check`, reminder/check lanes | evidence-backed but not definitive | Mostly trust-first; avoid latest/official proof claims unless a supported checker proves it. |
| Update source | saved source, supported source, provider-limited, reminder-only | evidence-backed/review-only | Safe if generic creator pages stay reminder-only. |
| Review needs | `Needs review`, parser/inspection warnings | evidence-backed cue | Safe; review language is appropriate. |
| Parser warnings | malformed/partial read warnings | evidence-backed cue | Safe; does not prove broken mods. |
| Script placement | shallow script placement rule | deterministic placement rule | Safe for placement; not dependency proof. |
| Thumbnails/previews | preview available/missing/unsupported diagnostics | evidence-backed cue | Safe; no broken-file implication. |
| Folder safety | real disk folder paths and `0 files` | deterministic fact | Safe; Open Folder uses real path when available. |
| Organization/sorting | suggested preview/move plan language | review workflow / suggested plan | Needs trust boundary doc before future automation expands. |
| Staging/Inbox support files | dependency wording in Downloads | overclaim risk | Needs softer support-file/review wording. |
| AI | `ai_classifier` status is planned only | future work | Safe because no AI runtime is wired. |

### 2. Current automation inventory

| Action | Current classification | Boundary |
| --- | --- | --- |
| Scan | reads files and writes app database | Allowed; must stay local and truthful. |
| Open folder | opens OS location | Allowed only with real disk path. |
| Review | opens route / app database state | Allowed. |
| Safe Action Preflight | review workflow | Allowed; must not imply safe delete/replacement. |
| Duplicate comparison | opens route / comparison | Allowed; no cleanup action. |
| Update source save | saves metadata | Allowed; source is not proof by itself. |
| Update check | supported check only | Allowed for supported checkers; no scrape/replacement. |
| Reminder-only source | saves metadata | Allowed; manual follow-up only. |
| Staging/Inbox | preview/confirmed moves may exist | Requires preview, confirmation, and restore behavior. |
| Sorting/organization | suggested plan / confirmed moves | Future automation must start as suggested plan. |
| Future auto sorting | future destructive risk | Not allowed as Level 5. |
| Future mod updating | future destructive risk | Not allowed without provider-safe checks, backup, rollback, confirmation. |
| Future AI assistance | future suggestion only | AI may assist Levels 0-3 only. |

### 3. Sims-format proof inventory

SimSuite can inspect today:

- full file hash, filename, extension, size, modified time, source root, relative path, and folder path.
- DBPF/package metadata and parser/inspection warnings where parsing succeeds.
- selected-file package preview data and localthumbcache preview data when available.
- script archive names/hints, but not normalized script-content duplicate fingerprints.
- Tray file presence/path metadata, but not full Tray preview extraction.
- saved update source fields and supported special-mod checkers.
- rule-backed special-mod support-file clues in Inbox, but not a general dependency graph.

SimSuite cannot prove today:

- missing mesh.
- true dependency graph or recolor-to-mesh relationship.
- safe to delete.
- broken mod or broken CC.
- encrypted or unavailable lastCrash contents.
- generic creator page updates.
- broad CurseForge matching.
- automatic safe replacement.
- automatic safe sorting for every file.
- AI-verified correctness.

### 4. Future feature safety model

| Future feature | Classification | Required boundary |
| --- | --- | --- |
| Auto sorting | safe as suggestion only | Preview/staging/undo and confirmation before file changes. |
| Mod updating | requires provider/API support | No automatic download or replacement. |
| AI-assisted categorization | safe as suggestion only | AI suggests; deterministic rules/user decide. |
| AI-assisted update/source suggestions | safe as suggestion only | No invented official source or latest proof. |
| Duplicate cleanup | should not be implemented yet | Needs backup/restore and explicit product decision. |
| Dependency detection | requires deterministic file-format proof | Current support-file hints are not a dependency graph. |
| Missing mesh detection | requires research spike | Not allowed without Sims resource proof. |
| Safe-delete | should not be implemented yet | Requires deterministic dependency/resource evidence and recovery model. |
| Staging | safe with preview/staging/undo | Keep explicit and recoverable. |
| Backup/undo | required for file-changing actions | Must exist before destructive or broad automation. |
| Quarantine | should not be implemented yet | Needs proof, backup, restore, and confirmation. |

## Final Report

### What was audited

- Library rows, detail surfaces, duplicate cues, update cues, Safe Action Preflight, Field Guide copy, Downloads special-mod setup copy, Downloads conflict evidence copy, Duplicates route copy, Updates route copy, and related trust-sensitive docs.
- Backend trust boundaries around duplicate truth, scanner evidence, parser/inspection warnings, update/watch checks, support-file clues, and move/repair preview wording.
- Current docs and status reports that future Codex prompts use as repo memory.

### Community warning takeaway

The warning applies directly to future SimSuite automation risk. SimSuite must not look like a tool that claims it can automatically fix, quarantine, remove, update, or classify Sims 4 mods without deterministic file-format evidence and recoverable user-confirmed workflows.

### Are we at risk?

Low risk today for Library duplicate and update truth because recent work already avoids exact duplicate overclaims, safe-delete claims, generic scraping, and automatic replacement.

The main risk found in this sprint was wording around Downloads special-mod setup and support-file clues. Some copy sounded like a safe install or dependency truth instead of a preview/review workflow. That was softened.

### Current trust boundaries

- Deterministic facts: same-content duplicate proof, real disk folder paths, file paths, hashes, sizes, dates, scan-owned folder rows, and exact configured paths.
- Evidence-backed cues: parser warnings, inspection warnings, supported watch checker results, preview availability, and special-mod rule evidence.
- Heuristic hints: filename clues, version-like tokens, same folder, same pack, same family, and weak creator/source clues.
- Review-only states: weak metadata, failed checks, unsupported providers, support-file clues, and mixed special-mod layouts.
- Future work: missing mesh detection, dependency graphs, safe-delete, automatic replacement, quarantine, generic provider matching, and AI verification.

### Automation readiness levels

- Level 0: informational facts.
- Level 1: evidence-backed cue.
- Level 2: review workflow.
- Level 3: suggested plan with preview.
- Level 4: user-confirmed action with preview, confirmation, backup/restore behavior, and recoverable errors.
- Level 5: automated action. Destructive Level 5 automation is forbidden for now.

### AI assistance boundaries

AI may summarize metadata, suggest categories/search terms, explain review clues, draft organization plans, and help rank review items.

AI must not decide that a mod is broken, safe to delete, safe to replace, missing a dependency, missing a mesh, definitely outdated, or officially sourced. It also must not invent source URLs or execute file changes without deterministic guardrails and user confirmation.

### Auto sorting readiness

Auto sorting is not ready as automatic file movement. The safe next version is a suggested organization plan with preview, staging, user confirmation, and recovery behavior.

### Mod updating readiness

Mod updating is safe only as source setup, reminders, supported checks, and review workflows. Automatic download or replacement is not ready without provider-safe checks, backup, rollback, confirmation, and policy review.

### Overclaiming found

- Field Guide used `auto-fix` wording.
- Downloads and special-mod setup copy used `safe install`, `safe repair`, `safe fix`, and `Fix old setup` style labels.
- Downloads conflict evidence used dependency-style labels such as `missing dependency` / `Depends on` language for support-file clues.
- Some mock/API evidence used `Official latest`, `trusted`, and dependency resolution wording that could read stronger than the product can prove.

### What changed

- Added the standing trust policy doc.
- Added this sprint report with pre-implementation audit inventories and final findings.
- Added a focused copy guard test for current trust-sensitive UI surfaces.
- Replaced risky user-facing copy with preview, approval, source-check, support-file, and manual-review language.
- Updated the backend systems map to point future trust-sensitive work at the new boundary doc.
- Updated session/status memory notes for the sprint.

### What this means for the user

SimSuite should sound more honest and less like it is promising magic fixes. It still helps users review, compare, and plan, but it does not claim it can automatically fix, quarantine, remove, replace, or prove dependencies for Sims 4 mods. Future sorting, updating, and AI ideas now have clearer rules before they can touch real files.

### Trust / safety boundary

This sprint added no automation feature. SimSuite still will not auto-fix, auto-sort, auto-update, quarantine, delete, or safe-delete mods. Any future file-changing workflow must start with a preview, require user confirmation, preserve a recovery path, and explain what evidence it has and what remains manual review.

### Files changed

- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`: new standing policy for evidence levels, automation levels, AI boundaries, and future report requirements.
- `simsuite-reports/TRUST_BOUNDARIES_AUTOMATION_READINESS_V1_REPORT.md`: sprint audit and final report.
- `src/trustBoundaryCopy.test.ts`: focused user-facing forbidden-claim guard.
- `src/components/FieldGuide.tsx`: replaced `auto-fix` with preview/approval wording.
- `src/screens/DownloadsScreen.tsx`: softened safe/fix/dependency-style UI copy.
- `src/screens/downloads/ConflictEvidenceDisplay.tsx`: changed dependency labels to support-file/review labels.
- `src/lib/api.ts`: softened mock/API user-facing evidence and action text.
- `src-tauri/src/core/install_profile_engine/mod.rs`: softened backend-generated special-mod evidence and review action labels.
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`: referenced the new trust-boundary policy.
- `SESSION_HANDOFF.md` and `docs/IMPLEMENTATION_STATUS.md`: added sprint memory notes.

### Tests

- `npm run build`: passed; existing Vite chunk-size warning remains.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed, `23` files and `88` tests.
- `cargo fmt`: ran.
- `cargo check`: passed with existing unused-code warnings.
- `cargo test`: passed, `241` passed and `2` ignored stress tests.
- `cargo build --release`: passed with existing unused-code warnings.
- `npm run test:rust`: passed, `241` passed and `2` ignored stress tests.

### Desktop/runtime proof

Desktop proof and smoke were not run because this sprint did not change route flow, geometry, backend command contracts, schema, or runtime behavior. The visible changes are narrow copy changes plus a static copy guard test. Full build, TypeScript, unit, and Rust validation passed.

### What could not be verified

- A full manual click-through of every route label was not performed in desktop proof.
- Historical reports still contain old wording as history; the new copy guard intentionally scans current user-facing source files, not historical sprint reports.
- Internal enum/field names such as `officialLatest` and `missingDependencies` still exist where they are API/model compatibility details; current user-facing copy was softened.

### Recommended next sprint

Relationship count SQL aggregation/cache v2 if real-library traces show broad count cost; otherwise Duplicate Truth Engine v3 scan-time package/script fingerprints. Auto Sorting Readiness v1 should stay suggestion/preview/staging-only if it is chosen later.

### Docs updated

- `docs/TRUST_BOUNDARIES_AND_AUTOMATION_READINESS.md`
- `docs/LIBRARY_BACKEND_SYSTEMS_MAP.md`
- `SESSION_HANDOFF.md`
- `docs/IMPLEMENTATION_STATUS.md`
- `simsuite-reports/TRUST_BOUNDARIES_AUTOMATION_READINESS_V1_REPORT.md`

### Unrelated worktree changes

Pre-existing unrelated changes in `.cocoindex_code/*`, `src/screens/HomeScreen.tsx`, `src/styles/globals.css`, and older unrelated status/handoff hunks were left alone and should not be included in this sprint commit.

### Commit

Pending commit.

### Final honest verdict

Verified: SimSuite trust boundaries and automation readiness v1 are documented and current risky claims are corrected.
