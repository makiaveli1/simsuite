# SimSuite Player-Journey Product Plan v2

**Date:** 3 August 2026  
**Status:** Product-direction correction and execution plan  
**Code changes in this pass:** None  
**Canonical repository:** `makiaveli1/simsuite` cloned at `/Users/gracebabalola/Downloads/SimSuite` and active through PC Bridge

## 1. Correct product definition

SimSuite is not an organization product.

SimSuite is a local-first, all-in-one mod management suite for Sims players. Its job is to support the complete mod journey, beginning when a player downloads a mod and continuing through intake, inspection, installation, organization, updates, troubleshooting, recovery, removal, and long-term collection care.

The immediate focus is The Sims 4 because it has the largest and most active mod ecosystem. The architecture should allow later support for other Sims titles and, eventually, other life-simulation games without weakening the Sims 4 experience.

### Core promise

> From download to game, SimSuite helps every player install, understand, organize, update, troubleshoot, and recover their mods safely, in the way that suits them.

### North-star outcome

A player should be able to answer these questions without needing to be technically skilled:

- What did I download?
- Where should it go?
- Is it installed correctly?
- Is anything missing, duplicated, outdated, misplaced, or risky?
- Can SimSuite handle this for me?
- What exactly will it change?
- Can I undo it?
- Is my setup ready for the game and the current patch?

## 2. Non-negotiable product principles

### 2.1 One suite, not one crowded screen

SimSuite should feel like one connected product, but each stage of the mod journey should have its own calm workspace. Downloads, Library, Organize, Updates, Review, Duplicates, Patch Recovery, and Tools are connected parts of the same system, not competing dashboards.

### 2.2 The player remains in control

Automation must be visible, previewable, reversible, and adjustable. SimSuite may recommend or prepare an action, but the player should understand what will happen before important file changes occur.

Every meaningful action should follow this pattern:

1. Detect
2. Explain
3. Propose
4. Preview
5. Approve or apply under a user-approved rule
6. Verify
7. Offer undo or recovery

### 2.3 Respect existing organization styles

SimSuite must not force everyone into one folder structure. Players organize by creator, content type, gameplay purpose, household, build project, download date, source site, update risk, or personal systems that make sense only to them.

The product should support:

- preserve my current structure
- tidy obvious mistakes only
- organize by creator
- organize by content type
- organize by gameplay purpose
- organize by source or collection
- use a custom rule template
- combine several approaches
- exclude protected folders or files

The correct question is not “What is the perfect folder structure?” It is “What structure helps this player find, maintain, and safely use their content?”

### 2.4 Low-spec systems are a first-class requirement

Many Sims players use older laptops, integrated graphics, limited memory, slower drives, and very large mod folders. SimSuite must not assume a powerful gaming PC.

Performance work is therefore part of product correctness, not optional polish.

### 2.5 Same truth, different explanation

Casual, Seasoned, and Creator/Technical views should use the same underlying facts and safety rules. The modes change wording, density, controls, and evidence depth. They must not produce different truths.

### 2.6 Local-first and privacy-respecting

Core scanning, inspection, organization, snapshots, and recovery should work locally. Network services should add optional update information or metadata, not become a requirement for understanding the player’s own files.

### 2.7 Never overstate certainty

“Duplicate,” “same creator,” “same folder,” “possible relationship,” and “confirmed dependency” are different claims. SimSuite should clearly distinguish proven facts from clues and suggestions.

## 3. Separate the three kinds of user choice

The current view modes are useful, but they should not carry every preference. SimSuite should separate three independent dimensions.

### 3.1 Experience view

**Casual**

- plain language
- fewer controls at once
- guided choices
- strong defaults
- clear risk and readiness summaries
- technical evidence available on demand

**Seasoned**

- more filters and batch tools
- concise technical context
- quicker access to rules, sources, versions, and folder paths
- greater control without requiring package-level knowledge

**Creator / Technical**

- complete evidence
- package/resource details
- namespaces, identifiers, relationships, logs, and rule reasoning
- advanced batch and diagnostic controls

### 3.2 Automation policy

This should be separate from experience view.

**Manual**

- SimSuite observes and explains
- the player performs or explicitly approves every change

**Guided**

- SimSuite prepares a recommended plan
- the player reviews and approves each batch

**Assisted**

- safe, user-approved rules can run automatically
- ambiguous or risky cases stop for review

**Custom automation**

- technical users can define detailed rules, exclusions, thresholds, and approval boundaries

No mode should silently remove control. A casual player may prefer manual control, while a technical player may prefer carefully bounded automation.

### 3.3 Organization profile

Organization should be configurable independently from both experience and automation.

Profiles may include:

- preserve existing layout
- creator-first
- content-type-first
- gameplay-first
- project or household collections
- source-first
- update-risk-first
- custom hybrid

Every profile should support preview, exclusions, protected paths, and rollback.

## 4. The complete player journey

### Stage 1: Download and arrival

SimSuite watches user-approved download locations and recognizes new archives and supported files without taking ownership of the whole Downloads folder.

It should record, where available:

- original filename
- source URL or source hint
- creator hint
- download time
- archive contents
- expected game destination
- version hints
- related documentation and configuration files

### Stage 2: Intake and safe staging

New content enters a calm intake workspace before it reaches the game.

SimSuite should:

- identify ZIP, RAR, 7z, package, script, Tray, image, text, and configuration files
- inspect archives without immediately installing them
- detect unsafe paths and unexpected executables
- separate Mods content from Tray content
- preserve necessary sidecar files
- flag nested archives, duplicate copies, and ambiguous bundles
- recognize special mods that require guided setup

### Stage 3: Understand the download

The player receives an explanation appropriate to their view mode:

- what the download appears to be
- which creator or family it belongs to
- whether it contains script content
- where it should be installed
- whether dependencies are known
- whether it replaces or duplicates something already installed
- what SimSuite knows, suspects, and cannot confirm

### Stage 4: Plan installation

SimSuite prepares a proposed file plan rather than immediately moving files.

The plan should show:

- files to install
- destination paths
- folders to create
- existing files to replace, retain, archive, or ignore
- required dependencies
- configuration files to preserve
- detected placement problems
- expected organization rule
- snapshot and undo coverage

### Stage 5: Apply and verify

After approval, SimSuite performs the transaction safely.

It should:

- create a snapshot or transaction record
- apply file changes
- verify that expected files exist in the correct location
- verify script depth and Tray placement
- refresh only affected index entries
- report success, partial success, or blocked actions honestly
- provide a clear undo path

### Stage 6: Browse and organize

The Library becomes the installed-state workspace. It helps the player see and understand what is currently in Mods and Tray.

It should support:

- list, grid, and real folder views
- thumbnails loaded on demand
- fast search and filtering
- creator and category corrections
- user-defined collections and tags
- duplicate and version clues
- folder and bundle context
- open in Explorer/Finder
- reversible organization previews

The Library is important, but it is one stage of the broader lifecycle.

### Stage 7: Play readiness

Before launching the game, SimSuite can provide an optional readiness check:

- unresolved review items
- script files nested too deeply
- missing or ambiguous dependencies
- duplicate versions
- blocked or quarantined files
- recent game patch with unreviewed script mods
- incomplete installation transactions

This must be advisory and evidence-based, not a false “everything is safe” guarantee.

### Stage 8: Updates and patch day

SimSuite should reduce patch-day panic by keeping a local record of installed versions, known sources, last verified status, and player actions.

The long-term workflow should include:

- detect game version or patch change
- preserve the pre-patch state
- offer a patch-safe profile
- identify high-risk script mods first
- show known update information where a reliable source exists
- help disable, test, update, and restore groups
- warn against saving a valued game in a risky state
- record what was tested and what changed

### Stage 9: Troubleshooting

SimSuite should turn the manual 50/50 method into a guided, reversible process.

It should:

- create test groups
- preserve the original setup
- track which groups were enabled for each test
- narrow likely causes
- allow notes and results
- restore the original setup safely

Advanced diagnostics may later include resource conflicts, missing mesh evidence, recolor relationships, script namespaces, Tray usage, and package-level relationships.

### Stage 10: Remove, archive, and recover

Removing a mod should not be a blind delete.

A safe removal flow should check:

- exact duplicates
- related versions
- known dependencies
- Tray bundles
- configuration and sidecar files
- files installed in the same transaction
- player-protected items
- snapshot coverage

The player should be able to archive rather than delete, and restore a previous state when needed.

## 5. Core product architecture

### 5.1 Game adapter layer

Build Sims 4 deeply first, while isolating game-specific rules behind an adapter boundary.

The Sims 4 adapter owns:

- default paths
- supported file types
- Mods and Tray placement rules
- script-depth rules
- package and DBPF inspection
- thumbnail extraction
- cache behavior
- patch/version detection
- curated special-mod knowledge
- game-specific readiness checks

The shared SimSuite core owns:

- intake transactions
- file identity and provenance
- indexing and search
- preview/apply/verify/undo
- organization rules
- user preferences
- snapshots
- evidence levels
- activity history
- performance scheduling

Future games should add adapters rather than forcing Sims 4 logic into generic abstractions too early.

### 5.2 File identity and provenance

SimSuite needs a durable identity model that can answer where a file came from, which installation it belongs to, and how it changed over time.

Recommended layers:

- path identity
- size and modified-time fingerprint
- content hash when needed
- archive/install transaction identity
- creator/source metadata
- mod family and version identity
- user corrections and protected relationships

### 5.3 Evidence model

Every important conclusion should carry an evidence level:

- confirmed
- strongly supported
- possible
- unknown

This should be used for dependencies, creator matching, mod families, version replacement, conflicts, and safe removal.

### 5.4 Action planner and transaction engine

All file-changing features should use one shared engine for:

- plan generation
- conflict detection
- approval boundaries
- snapshots
- execution
- verification
- rollback
- audit history

Downloads installation, organization, duplicate cleanup, patch profiles, 50/50 groups, and safe removal should not each invent separate file-move logic.

### 5.5 User preference model

Preferences should record more than theme and density. They should include:

- view mode
- automation policy
- organization profile
- protected folders
- excluded files
- naming preferences
- creator/category corrections
- risk tolerance
- notification preferences
- update-source choices

## 6. Low-spec performance strategy

### 6.1 Do less work by default

- incremental scans instead of full rescans
- watch changed directories where reliable
- compare size and modified time before hashing
- hash only when identity or duplication requires it
- defer package parsing until useful
- defer thumbnails until visible or requested
- refresh affected rows after a transaction instead of rebuilding the whole library

### 6.2 Keep heavy work bounded

- limited worker concurrency
- cancellable scans and inspections
- per-file timeouts or watchdogs
- background work that yields to the interface
- memory-bounded thumbnail and metadata caches
- back-pressure when many files arrive at once

### 6.3 Keep the interface light

- paginated or virtualized large collections
- backend-native folder queries
- indexed search rather than full wildcard scans
- thumbnail paths or cached handles instead of large base64 payloads in every row
- compact summaries first, details on demand
- no package re-parsing each time a user selects a row

### 6.4 Establish real performance budgets

Before adding more automation, benchmark on at least three reference systems:

- low-spec laptop with a modest library
- typical player laptop with a large library
- technical/power-user system with a very large library

Measure:

- cold start
- idle memory
- scan time
- incremental refresh time
- search latency
- folder opening latency
- thumbnail time-to-visible
- CPU use during background work
- cancellation responsiveness

Exact budgets should be set from measurements, not guessed.

## 7. How the current implementation fits this vision

The existing work should be preserved. It already contains major parts of the required foundation:

- Tauri, React, TypeScript, Rust, and SQLite
- Mods and Tray scanning
- package and script inspection
- duplicate detection
- thumbnails
- Downloads intake and archive recognition
- special-mod guided setup
- rules and organization previews
- snapshots and rollback
- update-watch data
- Casual, Seasoned, and Creator views
- Library, Downloads, Duplicates, Organize, Review, Settings, Creator Audit, and Category Audit workspaces

This means SimSuite does not need a conceptual restart. It needs a corrected hierarchy and an execution order that connects these parts into one dependable journey.

## 8. Immediate correction to the roadmap

### Phase 0: Establish and protect the canonical Mac baseline

Before implementation changes:

1. Treat the clean GitHub `main` branch at `/Users/gracebabalola/Downloads/SimSuite` as canonical.
2. Work only on the protected `platform/cross-platform-foundation` branch.
3. Record repository history, project structure, declared scripts, dependencies, and known platform assumptions.
4. Install the macOS Tauri prerequisites and declared project dependencies without changing lockfiles unexpectedly.
5. Run TypeScript, frontend, Rust, build, and native macOS launch baselines.
6. Record every reproduced failure before fixing it.
7. Keep the abandoned unpushed Windows state out of the migration.

### Phase 1: Restore portable runtime truth

This remains the first coding sprint because every later workflow depends on reliable facts.

1. Reproduce suspected runtime and query defects against the canonical repository instead of assuming older audit findings still apply.
2. Fix only defects that are confirmed by the current code or executable tests.
3. Add integration tests that execute real SQLite queries and platform-boundary behavior.
4. Replace Windows-only universal development commands with portable dispatch while retaining Windows-specific proof helpers behind explicit platform lanes.
5. Isolate default-path discovery, native folder reveal, cache discovery, and filesystem behavior behind tested platform boundaries.
6. Verify folder metadata, deep-folder browsing, relationship counts, summary fields, and Staging command registration against the canonical code.
7. Run the real Tauri app on macOS and verify list, grid, folder, detail, reveal, and fixture-only transaction paths.
8. Keep real Apply, Restore, deletion, replacement, and cleanup blocked until the existing backend safety contract is complete.

### Phase 2: Build the golden end-to-end journey

The first major product proof should be:

> Download to installed, understood, organized, verified, and undoable.

Implement and verify one connected path for:

- a single `.package` file
- a `.ts4script` mod
- a normal archive
- a Tray bundle
- one curated special mod with configuration files

For each case, verify:

1. arrival detected
2. contents explained
3. correct destination proposed
4. organization choice respected
5. snapshot created
6. files applied
7. placement verified
8. Library refreshed incrementally
9. action history recorded
10. undo restores the previous state

### Phase 3: Flexible organization system

1. Preserve-existing mode
2. creator-first template
3. content-type template
4. gameplay-purpose template
5. custom hybrid rules
6. protected paths and exclusions
7. preview comparison showing before and after
8. reversible batch actions
9. naming and folder-depth checks
10. migration paths for messy existing libraries

### Phase 4: Readiness, updates, and patch recovery

1. readiness check based on evidence
2. game-patch detection
3. pre-patch snapshot
4. high-risk script review
5. local installed-version history
6. reliable-source update tracking
7. patch-safe profile
8. guided disable/test/restore groups
9. patch recovery workspace
10. player-visible activity history

### Phase 5: Troubleshooting and safe removal

1. guided 50/50 workflow
2. safe-delete preflight
3. duplicate cleanup transactions
4. archive instead of delete
5. stronger mod-family and version relationships
6. missing-dependency evidence
7. recolor-to-mesh evidence where technically supportable
8. resource-conflict evidence with clear limitations
9. Tray/save usage evidence where supportable

### Phase 6: Advanced automation

Only after the transaction engine and evidence model are trusted:

- user-approved automatic intake rules
- automatic organization recipes
- bounded update routines
- smart patch-day preparation
- recurring collection-health checks
- custom technical rules

AI may later explain ambiguous evidence or suggest classifications, but deterministic rules, local inspection, and user corrections remain the source of truth.

### Phase 7: Multi-game expansion

After the Sims 4 journey is complete and stable:

1. extract the proven game-adapter contract
2. identify shared versus game-specific concepts
3. add one additional Sims title as the first adapter test
4. validate that multi-game support does not slow or complicate Sims 4
5. expand to other life-simulation games only where SimSuite can provide a genuinely complete journey

## 9. Acceptance criteria for the next product milestone

The next milestone should not be “Library looks finished.” It should be:

### Player outcome

A player can download supported Sims 4 content, review it in plain language, install it correctly, keep their preferred organization style, see it in the Library, and undo the installation without touching File Explorer manually.

### Safety outcome

- no unapproved destructive action
- every file move belongs to a transaction
- every transaction is verified
- important actions are recoverable
- uncertainty is visible
- unsupported cases stop safely

### performance outcome

- the workflow remains responsive on the low-spec reference system
- large libraries do not require loading all rows or thumbnails at once
- scans can be cancelled
- one changed download does not trigger a complete library rebuild

### mode outcome

The same workflow succeeds in Casual, Seasoned, and Creator/Technical views, with different detail levels but identical file decisions and safety results.

## 10. What should not happen next

- Do not turn SimSuite into a folder organizer with extra pages.
- Do not force one “correct” organization structure.
- Do not add broad AI automation before deterministic actions are trustworthy.
- Do not call clue-level relationships dependencies.
- Do not add more top-level screens without connecting the existing journey.
- Do not optimize only for powerful development machines.
- Do not generalize for many games before the Sims 4 adapter is complete.
- Do not let visual polish outrun runtime correctness.

## 11. Next execution sequence

1. Finish the canonical macOS toolchain and dependency setup.
2. Capture the untouched frontend, Rust, build, and native desktop baselines.
3. Complete the platform-boundary audit against the current repository.
4. Implement the smallest portable development-command and native folder-reveal foundation.
5. Run cross-platform path and query integration tests before broader refactoring.
6. Document the current end-to-end journey and identify the first broken handoff between Downloads, transaction planning, Library refresh, and future undo.
7. Implement the fixture-only golden end-to-end slice without enabling real user-file mutation.
8. Benchmark it on a low-spec reference machine before broadening automation.

## 12. Final direction

SimSuite should become the player’s trusted mod companion, not merely a prettier way to view folders.

Its strongest long-term advantage is the combination of:

- full-lifecycle coverage
- player-controlled automation
- reversible file operations
- clear evidence and uncertainty
- flexible organization styles
- low-spec performance
- beginner-friendly explanations
- deep technical visibility when requested
- a Sims 4-first architecture that can later support more games

That is the standard every screen, feature, and technical decision should now be measured against.
