# SimSuite Master Roadmap & Linear Execution Plan

**Date:** 2026-05-13  
**Purpose:** Slow down, consolidate what has been built, and define the next development direction before more Codex implementation sprints.

---

## 0. Executive Summary

SimSuite is now past the “rough prototype” stage for the Library. The Library has been heavily improved across frontend, backend, proof, and trust boundaries.

The next work should **not** be random feature building. The next phase should be planned as a safe product roadmap:

1. **Clarify the app’s navigation and workflow architecture.**
2. **Fold redundant pages into clearer destinations.**
3. **Make Staging the safe preview layer.**
4. **Build Auto Sorting only as a suggested plan first.**
5. **Only later build confirmed file-changing actions with backup/restore.**
6. **Keep Updates, AI, and future automation inside strict trust boundaries.**
7. **Use Linear to manage the roadmap, safety gates, proof requirements, and dependencies.**

The key product rule:

> SimSuite should help users understand, review, and plan before it changes files.

---

## 1. Current Product Situation

### 1.1 What is strong now

The Library is currently the strongest part of SimSuite.

It now has:

- List view
- Grid view
- Folder view
- Empty folder metadata
- Real folder disk paths
- Inspector
- More Details
- Safe Action Preflight
- Duplicates bridge
- Updates bridge
- Needs Review bridge
- Thumbnail fallback and preview reuse
- Duplicate truth guardrails
- Package/script duplicate fingerprints
- Large synthetic backend stress proof
- Desktop proof and smoke restored

### 1.2 What is safe today

SimSuite is safe today as a **review and understanding tool**.

It can safely:

- scan files
- index files
- show library views
- show folder structure
- show empty folders after scan
- show duplicate evidence
- show update-source gaps
- show review cues
- show previews/fallbacks
- open real folders
- explain limitations
- show staging folders as preview/readiness only

### 1.3 What is not safe yet

SimSuite is **not** ready to:

- auto-sort files
- move files automatically
- delete files
- quarantine files
- replace old mods
- claim safe deletion
- claim missing meshes
- claim dependency truth
- claim broken mod/CC
- let AI decide file actions
- auto-update mods

These can only come later after safety layers exist.

---

## 2. Trust Model Going Forward

SimSuite must continue to follow this hierarchy:

| Level | Meaning | Allowed now? | Example |
|---|---|---:|---|
| Level 0 | Informational facts | Yes | “This file is in Mods/MCCC.” |
| Level 1 | Evidence-backed cue | Yes | “Needs review.” |
| Level 2 | Review workflow | Yes | “Compare these files.” |
| Level 3 | Suggested plan | Yes | “Here is where files could be organized.” |
| Level 4 | User-confirmed action | Later | “Apply this move after backup + confirmation.” |
| Level 5 | Automated destructive action | No | “Auto-delete/quarantine/replace.” |

### 2.1 Critical rule

Auto Sorting can start only as:

> **Suggested Plan with Preview**

It must not start as:

> **Move my files automatically**

### 2.2 AI rule

AI can help with:

- summaries
- categories
- explanations
- plan drafting
- search terms
- review prioritization

AI must not decide:

- a mod is broken
- a file is safe to delete
- a dependency is missing
- a mesh is missing
- a source is official
- a file should be moved without confirmation

---

## 3. Product Navigation Problem

The app currently has many pages because development happened system by system.

That was useful while building, but the sidebar now risks feeling like a list of internal modules rather than a clear user workflow.

Current visible concepts include:

- Home
- Scan
- Guide
- Inbox
- Library / My CC
- Updates
- Organize / Tidy Up
- Review / Needs
- Creators
- Types
- Duplicates / Same File
- Settings
- Staging

### 3.1 Main issue

Some pages are true destinations. Others are better as:

- Library filters
- Library lenses
- Organize sub-pages
- Review panels
- Help/settings sections
- future workflow screens

### 3.2 Recommended destination model

| Destination | Should remain top-level? | Reason |
|---|---:|---|
| Home | Yes | Dashboard and current state |
| Scan | Yes | Primary setup/action |
| Library | Yes | Core product surface |
| Inbox | Yes | Intake for new/downloaded content |
| Organize | Yes | Future sorting/planning area |
| Updates | Yes | Dedicated source/watch workflow |
| Review | Maybe | Could stay if review queue remains strong |
| Settings | Yes | App setup and preferences |
| Duplicates | Maybe | Could become Library workbench |
| Creators | Probably no | Better as Library lens/filter |
| Types | Probably no | Better as Library lens/filter |
| Guide | Maybe no | Could move into Help/Settings |
| Staging | Probably no as-is | Better folded into Organize as “Plans/Staging” |

---

## 4. The Difference Between Inbox, Staging, and Organize

### 4.1 Inbox

**Meaning:** New things arrived.

Inbox should handle:

- downloaded archives
- newly imported files
- extracted folders
- files waiting for review
- source setup prompts
- “before it enters the main Library” workflow

Simple user wording:

> “New files are waiting here before they become part of your organized Library.”

### 4.2 Staging

**Meaning:** A proposed change is being prepared.

Staging should handle:

- preview plans
- suggested moves
- proposed organization
- result review before applying
- later: apply history and recovery

Simple user wording:

> “This is where SimSuite shows a plan before anything changes.”

### 4.3 Organize

**Meaning:** The user wants help arranging the Library.

Organize should contain:

- suggested sorting plans
- staging previews
- future apply workflow
- future plan history
- future cleanup workflows, only if safe

Simple user wording:

> “Organize helps you plan how files could be arranged. It does not move files unless you approve a safe plan later.”

---

## 5. Roadmap Overview

### Phase 0 — Project Control and Linear Setup

Goal: Stop sprint sprawl and organize the next stages.

Deliverables:

- Linear project created
- milestones created
- labels created
- issue template created
- Codex report requirements added to issues
- current known gaps entered as backlog items

Status: Next planning step.

### Phase 1 — Navigation & Workflow Architecture

Goal: Decide what each page is for and simplify the app map.

Deliverables:

- route inventory
- page purpose matrix
- top-level vs sub-page decisions
- Casual/Seasoned/Creator visibility rules
- recommended sidebar model
- implementation plan for navigation changes

Status: Should happen before Auto Sorting code.

### Phase 2 — Staging Preview Plan Foundation

Goal: Turn Staging from folder-count preview into a structured plan surface.

Deliverables:

- `StagingPlan` model
- `StagingPlanItem` model
- evidence/caveat fields
- preview-only backend command
- no file-changing actions
- proof that it does not mutate files

Status: First implementation phase after planning.

### Phase 3 — Auto Sorting Suggested Plan v1

Goal: Generate organization suggestions without moving files.

Deliverables:

- deterministic sorting rules
- suggestion reasons
- evidence levels
- caveats
- preview UI
- no apply button unless disabled/not ready
- tests and desktop proof

Status: Safe first “Auto Sorting” product feature.

### Phase 4 — Apply Safety Contract

Goal: Prepare for future user-confirmed file moves.

Deliverables:

- dry-run
- explicit confirmation
- backup/restore design
- path validation
- conflict handling
- per-file result logs
- rollback/recovery behavior
- desktop proof

Status: Required before any real file movement.

### Phase 5 — Updates Provider Onboarding

Goal: Make updates more useful without unsafe replacement.

Deliverables:

- provider model
- exact supported checkers
- manual source review
- provider confidence
- no scraping
- no automatic replacement
- future CurseForge official API path if allowed

Status: Later, after navigation/staging clarity.

### Phase 6 — AI-Assisted Review and Sorting

Goal: Add AI only as suggestion/explanation support.

Deliverables:

- AI suggestion interface
- no AI file-changing decisions
- privacy/logging rules
- deterministic guardrails
- mocked tests first
- user-facing caveats

Status: Later, after deterministic workflows are stable.

### Phase 7 — Real-Library Validation and Beta Readiness

Goal: Prove SimSuite works on real messy libraries.

Deliverables:

- consent-based local validation plan
- sanitized logs
- scan-time measurements
- thumbnail coverage measurements
- duplicate findings review
- update-source gap review
- no private files committed

Status: Required before beta.

---

# 6. Detailed Plan: Phase 0 — Linear Setup

## Objective

Use Linear to make SimSuite development trackable, reviewable, and safer.

## Linear project

Create one project:

```text
SimSuite — Safe Automation Roadmap
```

## Milestones

1. `M0 — Project Control & Roadmap`
2. `M1 — Navigation & Workflow Simplification`
3. `M2 — Staging Preview Plan Foundation`
4. `M3 — Auto Sorting Suggested Plans`
5. `M4 — Apply Safety Contract`
6. `M5 — Updates Provider Onboarding`
7. `M6 — AI-Assisted Suggestions`
8. `M7 — Real-Library Validation & Beta Readiness`
9. `M8 — Technical Debt & Proof Infrastructure`

## Suggested labels

### Area labels

- `area:library`
- `area:inbox`
- `area:staging`
- `area:organize`
- `area:updates`
- `area:duplicates`
- `area:review`
- `area:navigation`
- `area:settings`
- `area:proof`
- `area:docs`
- `area:backend`
- `area:frontend`
- `area:trust`

### Work type labels

- `type:audit`
- `type:implementation`
- `type:proof`
- `type:docs`
- `type:test`
- `type:refactor`
- `type:bug`
- `type:research`

### Safety labels

- `trust:informational`
- `trust:evidence-backed`
- `trust:review-only`
- `trust:suggested-plan`
- `trust:file-changing-blocked`
- `trust:requires-confirmation`
- `trust:requires-backup`
- `trust:ai-boundary`

### Priority labels

- `P0`
- `P1`
- `P2`
- `P3`

## Linear statuses

Recommended statuses:

1. `Backlog`
2. `Ready for Planning`
3. `Ready for Codex`
4. `In Progress`
5. `Needs Review`
6. `Proof Required`
7. `Blocked`
8. `Done`

## Default Linear issue template

```md
## Goal
What should this issue accomplish?

## User outcome
What changes for the person using SimSuite?

## Scope
What is included?

## Out of scope
What must not be changed?

## Trust / safety boundary
What can SimSuite prove? What remains review-only?

## Files / systems to inspect
- ...

## Implementation steps
1.
2.
3.

## Acceptance criteria
- ...

## Validation
- npm run build
- npx tsc --noEmit
- npm run test:unit
- cargo test / npm run test:rust if Rust touched
- npm run desktop:proof:fixtures if route/runtime touched
- npm run desktop:smoke:fixtures if route/runtime touched

## Final report must include
- What changed
- What this means for the user
- Trust / safety boundary
- Tests
- Desktop/runtime proof
- What could not be verified
- Commit hash
```

---

# 7. Detailed Plan: Phase 1 — Navigation & Workflow Architecture

## Objective

Decide what pages SimSuite should have and what each page should do.

## Why this matters

The current sidebar has many pages. Some overlap. Some are internal systems. Users should not need to understand the implementation to know where to go.

## Steps

### Step 1 — Route inventory

Codex should inspect:

- `src/App.tsx`
- `src/components/layout/Sidebar.tsx`
- `src/lib/experienceMode.ts`
- all `src/screens/*Screen.tsx`
- all route-related tests
- desktop proof navigation steps

Create a table:

| Route | Visible label | Mode visibility | Current purpose | Real data? | Safe? | Keep top-level? |
|---|---|---|---|---|---|---|

### Step 2 — Page overlap audit

Compare:

- Inbox vs Staging
- Organize vs Staging
- Duplicates vs Library duplicate filter
- Creators vs Library creator filter
- Types vs Library type filter
- Review vs Library Needs Review filter
- Guide vs Settings/help

For each pair, decide:

- separate pages
- one page becomes sub-view
- one page becomes filter
- one page becomes modal/panel
- one page should be hidden

### Step 3 — User mode audit

For each mode:

#### Casual

Should show fewer destinations:

- Home
- Scan
- Library
- Inbox
- Updates
- Settings
- maybe Review

#### Seasoned

Can show:

- Home
- Scan
- Library
- Inbox
- Organize
- Updates
- Review
- Settings

#### Creator

Can show deeper tools:

- Library
- Duplicates
- Review
- Updates
- Organize
- creator/type lenses if justified

### Step 4 — Proposed sidebar model

Produce a recommended final sidebar.

Candidate:

1. Home
2. Scan
3. Library
4. Inbox
5. Organize
6. Updates
7. Review
8. Settings

Fold:

- Creators into Library
- Types into Library
- Duplicates into Library or Review workbench
- Staging into Organize
- Guide into Settings/Help

### Step 5 — Decision doc

Create:

```text
docs/NAVIGATION_WORKFLOW_ARCHITECTURE.md
```

Include:

- final route decisions
- mode visibility rules
- migration plan
- test plan
- what not to remove yet

## Acceptance criteria

- No route is deleted without understanding what uses it.
- Every current page has a clear future role.
- Inbox, Staging, and Organize are clearly separated.
- Linear issues are created for follow-up implementation.
- No runtime behavior changes unless approved.

---

# 8. Detailed Plan: Phase 2 — Staging Preview Plan Foundation

## Objective

Create a structured preview plan model before Auto Sorting.

## User outcome

Users can see what SimSuite is proposing before anything changes.

## Proposed data model

```ts
type StagingPlan = {
  id: string;
  createdAt: string;
  source: "library" | "inbox" | "organize" | "manual";
  status: "preview_only" | "blocked" | "ready_for_review";
  itemCount: number;
  caveats: string[];
  items: StagingPlanItem[];
};

type StagingPlanItem = {
  fileId: number;
  fileName: string;
  currentPath: string;
  suggestedDestinationPath: string | null;
  actionKind: "suggest_move" | "suggest_group" | "suggest_review" | "no_action";
  evidenceLevel: "deterministic" | "evidence_backed" | "heuristic" | "review_only";
  reason: string;
  caveats: string[];
  wouldTouchFiles: false;
};
```

## Steps

### Step 1 — Audit existing preview/apply models

Inspect:

- `rule_engine`
- `move_engine`
- Downloads preview models
- Staging commands
- Inbox commands

### Step 2 — Define read-only backend command

Possible command:

```text
generate_staging_preview_plan
```

Rules:

- read-only
- no file movement
- no delete
- no cleanup
- no quarantine
- no apply
- no AI decisions

### Step 3 — Add tests

Tests should prove:

- command returns plan items
- `wouldTouchFiles` is false
- no file operation helper is called
- evidence/caveats are present
- unsupported files become review-only

### Step 4 — Update Staging UI

Staging should show:

- plan summary
- item rows
- reason
- caveats
- “No files changed”
- disabled future Apply area or no Apply at all

### Step 5 — Proof

Desktop proof should open Staging and capture the preview-only plan.

## Acceptance criteria

- Staging has a structured plan.
- No file changes are possible.
- Every item explains why it is suggested.
- Unsafe labels are absent.
- Tests and proof pass.

---

# 9. Detailed Plan: Phase 3 — Auto Sorting Suggested Plan v1

## Objective

Generate organization suggestions without moving files.

## User outcome

Users can ask SimSuite:

> “How should I organize this?”

And receive a preview-only plan.

## Suggested sorting rules

### Deterministic rules

Safe when metadata is strong:

- `.ts4script` files should usually remain no deeper than one folder under Mods.
- Tray files belong to Tray-related groupings.
- Files already in a clear creator folder can stay grouped.
- Empty folders are real folders but not content.
- Exact duplicate files should be reviewed, not auto-moved.

### Evidence-backed rules

Safe as suggestions:

- known content type
- known creator
- known pack/mod family
- package/script classification
- update source exists/missing
- folder grouping

### Heuristic rules

Must be marked review-only:

- filename-only guesses
- creator guessed from brackets
- unknown type
- weak version tokens
- same folder hints
- same pack hints

## Suggested destination categories

Initial safe buckets:

- `Script Mods`
- `CAS`
- `Build/Buy`
- `Gameplay`
- `Presets & Sliders`
- `Overrides & Defaults`
- `Tray`
- `Needs Review`
- `Unknown / Leave in place`

## Steps

### Step 1 — Rule inventory

Codex should list all possible evidence sources:

- extension
- package metadata
- content kind
- creator
- folder
- source root
- update source
- parser warning
- review queue
- duplicate truth
- package/script fingerprint
- thumbnail/preview availability

### Step 2 — Scoring model

Each suggestion should include:

- confidence
- evidence level
- reason
- caveats
- do-not-move flag when unsafe

### Step 3 — Plan generation

Generate suggestions for:

- selected files
- current folder
- Inbox batch
- whole Library later, but not v1

### Step 4 — UI

Inside Organize/Staging:

- show plan
- group suggestions by destination
- show evidence
- show caveats
- show “No files changed”

### Step 5 — Proof

Desktop proof should confirm:

- plan loads
- no apply action is enabled
- suggested destinations render
- no files change

## Acceptance criteria

- No file movement.
- No unsafe claims.
- Suggestions are explainable.
- Low-confidence suggestions are marked review-only.
- Tests and proof pass.

---

# 10. Detailed Plan: Phase 4 — Apply Safety Contract

## Objective

Prepare for future file movement, but only after preview plans are proven.

## Required safety pieces

Before any apply/move action:

1. Dry-run result
2. Explicit confirmation
3. Backup or restore path
4. Source path validation
5. Destination path validation
6. Duplicate destination handling
7. Permission failure handling
8. Partial failure handling
9. Per-file result log
10. Undo/recovery plan
11. Desktop proof
12. Trust-boundary copy guard

## Steps

### Step 1 — Apply readiness audit

Audit:

- move_engine
- existing Downloads apply paths
- commit staging commands
- cleanup commands
- rollback/snapshot concepts

### Step 2 — Backup/restore design

Design:

- backup location
- manifest format
- restore command
- validation
- cleanup policy

### Step 3 — Dry-run/apply split

Require:

- `preview_plan`
- `dry_run_plan`
- `apply_plan_confirmed`

### Step 4 — Conflict handling

Handle:

- destination exists
- file locked
- permission denied
- disk missing
- path too long
- source missing
- cross-drive behavior

### Step 5 — Result log

Each file action must record:

- source
- destination
- action
- status
- error
- timestamp
- rollback reference

## Acceptance criteria

No broad apply feature until all safety pieces exist.

---

# 11. Detailed Plan: Phase 5 — Updates Provider Onboarding

## Objective

Make update tracking more useful without unsafe downloads/replacement.

## Current safe update model

Allowed:

- no source
- watched
- reminder-only
- could not check
- update may be available
- checked recently
- no update found
- open source page
- manual review

Not allowed:

- automatic replacement
- scraping generic pages
- claiming official source
- claiming definitely latest/outdated
- bypassing creator/provider restrictions

## Steps

### Step 1 — Provider architecture audit

Inspect:

- watch tables
- source types
- GitHub checks
- special mod checks
- manual URLs
- settings for API keys

### Step 2 — Source confidence model

Create clearer levels:

- exact supported source
- saved manual source
- creator/reference page
- reminder-only
- unsupported provider
- unknown

### Step 3 — Provider onboarding plan

Future providers:

- GitHub releases
- special mod fixed pages
- CurseForge official API, only if allowed
- ModTheSims if official/source-safe path exists
- manual reminder-only pages

### Step 4 — UI

Updates should show:

- what SimSuite can check
- what it cannot
- why source is reminder-only
- no replacement claims

## Acceptance criteria

- No scraping.
- No downloads.
- No replacement.
- Source confidence is clear.
- Tests and proof pass.

---

# 12. Detailed Plan: Phase 6 — AI-Assisted Suggestions

## Objective

Use AI to help explain and suggest, not decide or change files.

## Safe AI features

Allowed first:

- summarize file metadata
- explain review cues
- suggest category
- suggest search terms
- draft organization plan
- rank review queue
- explain why evidence is weak

Not allowed:

- AI decides broken mod
- AI decides safe delete
- AI decides dependency missing
- AI moves files
- AI replaces files
- AI invents official source
- AI reads files the backend did not parse

## Steps

### Step 1 — AI boundary doc

Create:

```text
docs/AI_ASSISTED_WORKFLOWS_BOUNDARY.md
```

### Step 2 — Mock AI output first

Before any model call:

- define input schema
- define output schema
- build mock response
- test UI with mock data
- add copy caveats

### Step 3 — Privacy design

Define:

- what metadata can be sent
- what is never sent
- local-only option
- logging rules
- no raw paths if avoidable
- no file contents unless explicit future consent

### Step 4 — First AI feature

Recommended first AI feature:

> AI explanation of an organization plan

Not AI sorting itself.

## Acceptance criteria

- AI is clearly labeled suggestion-only.
- No file-changing action depends on AI.
- Tests use mocked model calls.
- Privacy rules are documented.

---

# 13. Detailed Plan: Phase 7 — Real-Library Validation

## Objective

Test SimSuite on real messy data without risking privacy or files.

## User consent rule

Real-library validation must be explicitly approved and local.

Do not commit:

- real file names
- real paths
- thumbnails
- archives
- logs with private paths
- screenshots exposing private paths

## Validation targets

- scan time
- memory usage
- thumbnail coverage
- duplicate detection
- package/script fingerprint coverage
- folder count correctness
- Updates no-source volume
- review queue quality
- false positives
- UI responsiveness

## Steps

### Step 1 — Sanitized diagnostic mode

Add or use diagnostics that output:

- counts
- timings
- types
- error categories
- no private paths

### Step 2 — Controlled local run

Run against user-approved folders only.

### Step 3 — Report

Report:

- scan duration
- total files
- errors
- preview coverage
- duplicate counts
- name/version review counts
- update source gaps
- performance warnings

## Acceptance criteria

- No private files committed.
- No destructive actions.
- User gets clear evidence of real-world readiness.

---

# 14. Detailed Plan: Phase 8 — Technical Debt and Release Readiness

## Objective

Keep the app maintainable and easier to ship.

## Known debt

- existing Rust warnings
- existing Vite chunk warning
- recurring unrelated Home/status worktree changes
- `.cocoindex_code/*` local changes
- stale schema mirror concerns
- desktop proof can be sensitive to proof/smoke timing
- Staging backend mutating commands exist but are not exposed
- old route/page naming may confuse users

## Workstreams

### 14.1 Warnings cleanup

- categorize warnings
- remove dead code if safe
- suppress only if justified
- keep tests green

### 14.2 Bundle/chunk cleanup

- inspect Vite chunk warning
- identify large imports
- lazy load routes if missing
- avoid premature optimization

### 14.3 Worktree hygiene

- document ignored/generated files
- review `.gitignore`
- isolate Home work
- avoid accidental staging

### 14.4 Proof reliability

- ensure desktop proof and smoke remain sequential-safe
- add Staging proof once route changes matter
- keep latest-summary useful

### 14.5 Documentation cleanup

- keep backend map current
- keep trust doc current
- keep implementation status concise
- archive old reports if needed

---

# 15. Linear Issue Breakdown

## Project: SimSuite — Safe Automation Roadmap

### M0 — Project Control & Roadmap

#### Issue M0-1: Create Linear project and labels

Priority: P0  
Type: planning  
Area: project

Steps:

1. Create Linear project.
2. Add milestones.
3. Add labels.
4. Add issue template.
5. Add required report sections.
6. Link current docs.

Acceptance:

- Project exists.
- Labels exist.
- Template exists.
- First backlog issues created.

#### Issue M0-2: Current repo/docs source-of-truth audit

Priority: P0  
Type: audit  
Area: docs

Steps:

1. Inspect `docs/IMPLEMENTATION_STATUS.md`.
2. Inspect backend map.
3. Inspect trust doc.
4. Identify stale or duplicate notes.
5. Recommend cleanup plan.

Acceptance:

- Current state source-of-truth is clear.
- Old notes are not accidentally deleted.
- Cleanup issues created.

---

### M1 — Navigation & Workflow Simplification

#### Issue M1-1: Full route inventory

Priority: P0  
Type: audit  
Area: navigation

Steps:

1. Inspect route registration.
2. Inspect sidebar visibility.
3. Inspect user-mode visibility.
4. Inspect tests/proof route assumptions.
5. Build route matrix.

Acceptance:

- Every route has owner/purpose/status.

#### Issue M1-2: Inbox vs Staging vs Organize decision

Priority: P0  
Type: product architecture  
Area: workflow

Steps:

1. Define Inbox.
2. Define Staging.
3. Define Organize.
4. Map current data sources.
5. Decide which route owns which workflow.
6. Document future structure.

Acceptance:

- No overlap/confusion between the three.

#### Issue M1-3: Sidebar simplification proposal

Priority: P1  
Type: planning  
Area: navigation

Steps:

1. Propose final sidebar.
2. Define Casual sidebar.
3. Define Seasoned sidebar.
4. Define Creator sidebar.
5. Identify pages to fold.
6. Identify proof/test updates.

Acceptance:

- Clear implementation plan approved.

#### Issue M1-4: Fold Creators and Types into Library

Priority: P2  
Type: implementation  
Area: library/navigation

Steps:

1. Audit Creators page.
2. Audit Types page.
3. Confirm if any unique behavior exists.
4. Add Library lenses if missing.
5. Hide/fold top-level nav if approved.
6. Update tests/proof.

Acceptance:

- No lost functionality.
- Navigation is simpler.

#### Issue M1-5: Fold Staging into Organize

Priority: P1  
Type: implementation  
Area: staging/organize

Steps:

1. Audit Staging route.
2. Audit Organize route.
3. Decide sub-route/tab model.
4. Move preview-only staging into Organize if approved.
5. Keep direct route safe.
6. Update desktop proof.

Acceptance:

- Staging is no longer confusing as standalone.
- No file-changing actions exposed.

---

### M2 — Staging Preview Plan Foundation

#### Issue M2-1: Define StagingPlan model

Priority: P0  
Type: backend design  
Area: staging

Steps:

1. Inspect existing rule/move models.
2. Define TypeScript and Rust shape.
3. Define evidence levels.
4. Define caveats.
5. Define read-only guarantee.

Acceptance:

- Model documented.
- No file-changing behavior.

#### Issue M2-2: Add preview-only staging plan command

Priority: P0  
Type: backend implementation  
Area: staging

Steps:

1. Add command.
2. Return plan items.
3. Ensure `wouldTouchFiles = false`.
4. Add tests proving no file operations.
5. Add API wrapper.

Acceptance:

- Backend returns plan.
- No file changes possible.

#### Issue M2-3: Staging plan UI

Priority: P1  
Type: frontend implementation  
Area: staging/organize

Steps:

1. Show plan summary.
2. Show item rows.
3. Show reason/caveats.
4. Show no-files-changed copy.
5. No apply button yet.
6. Add proof screenshot.

Acceptance:

- User can understand plan.
- No apply/move/delete action exists.

---

### M3 — Auto Sorting Suggested Plans

#### Issue M3-1: Sorting rules audit

Priority: P0  
Type: audit  
Area: organize

Steps:

1. List all available evidence.
2. Classify deterministic/evidence/heuristic.
3. Define safe sorting buckets.
4. Define do-not-move cases.
5. Document limits.

Acceptance:

- Sorting rules are trust-safe.

#### Issue M3-2: Suggested plan generator v1

Priority: P1  
Type: backend implementation  
Area: organize

Steps:

1. Generate suggestions for selected scope.
2. Assign destination bucket.
3. Add reason/caveats.
4. Mark weak suggestions review-only.
5. Return preview-only plan.
6. Add tests.

Acceptance:

- Suggestions exist.
- No files change.

#### Issue M3-3: Organize plan review UI

Priority: P1  
Type: frontend implementation  
Area: organize

Steps:

1. Show groups by destination.
2. Show evidence/caveats.
3. Show confidence/evidence level.
4. Show no-files-changed banner.
5. Add screenshot proof.

Acceptance:

- Plan is easy to review.

---

### M4 — Apply Safety Contract

#### Issue M4-1: Backup/restore design

Priority: P0  
Type: design  
Area: safety

Steps:

1. Audit move engine rollback.
2. Define backup manifest.
3. Define restore command.
4. Define failure cases.
5. Document user flow.

Acceptance:

- Safety design approved before apply.

#### Issue M4-2: Dry-run/apply split

Priority: P0  
Type: backend  
Area: safety

Steps:

1. Add dry-run command.
2. Validate source/destination.
3. Detect conflicts.
4. Return per-file result.
5. Do not move files.

Acceptance:

- Dry run works before apply exists.

#### Issue M4-3: Confirmed apply prototype

Priority: P2  
Type: implementation  
Area: safety

Blocked by:

- backup/restore design
- dry-run tests
- plan UI
- confirmation UI

Acceptance:

- Only after safety contract is complete.

---

### M5 — Updates Provider Onboarding

#### Issue M5-1: Provider architecture audit

Priority: P1  
Type: audit  
Area: updates

Steps:

1. Inspect watch source model.
2. Inspect provider states.
3. Inspect GitHub/special checker.
4. Identify provider abstraction gaps.
5. Document no-scraping boundaries.

Acceptance:

- Provider plan is clear.

#### Issue M5-2: Source confidence model v2

Priority: P1  
Type: backend/frontend  
Area: updates

Steps:

1. Define confidence levels.
2. Update API if needed.
3. Update UI wording.
4. Add tests.

Acceptance:

- User understands what can/cannot be checked.

#### Issue M5-3: CurseForge official API planning

Priority: P2  
Type: research/design  
Area: updates

Steps:

1. Confirm official API route.
2. Define API key storage.
3. Define matching limits.
4. Define provider policy.
5. Do not implement runtime matching yet.

Acceptance:

- Safe provider path documented.

---

### M6 — AI-Assisted Suggestions

#### Issue M6-1: AI boundary design

Priority: P1  
Type: design  
Area: AI

Steps:

1. Define allowed inputs.
2. Define forbidden inputs.
3. Define output schema.
4. Define privacy/logging rules.
5. Define user-facing caveats.

Acceptance:

- AI cannot touch files or decide truth.

#### Issue M6-2: Mock AI organization explanation

Priority: P2  
Type: frontend/backend mock  
Area: AI/organize

Steps:

1. Use mocked response.
2. Explain plan items.
3. Add caveats.
4. Add tests.
5. No model call yet.

Acceptance:

- UI proves usefulness before model integration.

---

### M7 — Real-Library Validation & Beta

#### Issue M7-1: Sanitized diagnostics plan

Priority: P1  
Type: backend  
Area: validation

Steps:

1. Define diagnostic counts.
2. Remove private paths.
3. Add timing.
4. Add command/report.
5. Test with fixture.

Acceptance:

- Safe to run locally without leaking data.

#### Issue M7-2: User-approved real scan

Priority: P2  
Type: validation  
Area: beta

Steps:

1. Get explicit approval.
2. Run scan locally.
3. Save sanitized report.
4. Do not commit files/logs.
5. Review findings.

Acceptance:

- Real-world readiness measured safely.

---

### M8 — Technical Debt & Proof Infrastructure

#### Issue M8-1: Rust warning cleanup audit

Priority: P2  
Type: tech debt  
Area: backend

Steps:

1. Capture warnings.
2. Classify dead code vs future code.
3. Remove or document.
4. Keep tests green.

Acceptance:

- Warnings reduced or tracked.

#### Issue M8-2: Vite chunk warning audit

Priority: P2  
Type: performance  
Area: frontend

Steps:

1. Inspect bundle.
2. Identify large chunks.
3. Lazy-load where safe.
4. Verify build.

Acceptance:

- Warning reduced or documented.

#### Issue M8-3: Worktree hygiene cleanup

Priority: P1  
Type: repo hygiene  
Area: devex

Steps:

1. Inspect `.cocoindex_code/*`.
2. Inspect Home changes.
3. Decide ignore/commit/discard strategy.
4. Keep sprint diffs clean.

Acceptance:

- No recurring dirty files confuse Codex.

---

# 16. Recommended Immediate Next Step

Do not implement Auto Sorting yet.

The next Codex sprint should be:

```text
Navigation & Workflow Architecture Audit v1
```

Purpose:

- map all routes
- define what belongs top-level
- decide where Staging belongs
- decide where Inbox/Organize overlap ends
- prepare Linear issues

After that, implement:

```text
Staging Preview Plan Foundation v1
```

Then:

```text
Auto Sorting Suggested Plan v1
```

---

# 17. Codex Planning Prompt Strategy

The next prompt to Codex should not say:

> “Build auto sorting.”

It should say:

> “Audit navigation and workflow architecture, create a route decision doc, and optionally create Linear issues.”

Then the following prompt should say:

> “Implement Staging Preview Plan backend v1, preview-only, no file movement.”

This prevents rushing into unsafe automation.

---

# 18. Final Product Direction

SimSuite should become:

> A calm, trustworthy Sims 4 mod management workbench that helps users understand, review, organize, and update their mods without pretending to magically fix things.

The product promise should be:

- easier mod management
- clearer Library
- safer reviews
- previewed plans
- update awareness
- AI-assisted explanations later

The product must not promise:

- automatic fixing
- safe deletion
- magic dependency detection
- automatic updating/replacement
- AI-decided mod cleanup

---

## Final Roadmap Order

1. Linear setup and issue structure
2. Navigation & Workflow Architecture Audit
3. Sidebar/page consolidation plan
4. Staging Preview Plan Foundation
5. Auto Sorting Suggested Plan v1
6. Staging plan UI
7. Apply Safety Contract design
8. Updates provider onboarding
9. AI-assisted explanation/suggestion layer
10. Real-library validation
11. Beta readiness cleanup
