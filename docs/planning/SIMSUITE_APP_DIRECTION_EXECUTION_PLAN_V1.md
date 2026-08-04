# SimSuite App Direction and Execution Plan V1

**Date:** 4 August 2026  
**Status:** Product and architecture direction after the cross-platform path-safety foundation  
**Branch:** `platform/cross-platform-foundation`

## 1. Direction decision

SimSuite should become the player's trusted, local-first mod companion for the complete Sims mod lifecycle.

It is not merely:

- a folder organizer;
- a Library viewer;
- a duplicate finder;
- an update tracker;
- or a collection of specialist tools.

Those are connected capabilities inside one journey:

> Download -> understand -> plan -> install -> verify -> browse -> maintain -> troubleshoot -> recover.

The immediate product remains Sims 4-first. The architecture may support more games later, but no generalization should weaken the Sims 4 experience or delay the first complete journey.

## 2. Why this direction wins

### Chosen direction: lifecycle companion

Advantages:

- solves the player's real problem from arrival to recovery;
- connects the substantial features already built;
- creates a stronger trust and safety advantage than a basic mod manager;
- makes patch day, troubleshooting, and undo part of one coherent product;
- supports beginners without removing technical depth;
- creates a reusable transaction and evidence foundation rather than many unrelated file tools.

Costs:

- requires disciplined sequencing;
- requires a shared transaction engine before broad automation;
- requires native filesystem proof and low-spec performance work;
- takes longer than polishing only the Library.

### Credible alternative: Library-first utility

Advantages:

- smaller scope;
- faster path to a polished browsing product;
- lower immediate execution risk.

Losses:

- Downloads, install planning, updates, patch day, troubleshooting, and recovery remain fragmented;
- SimSuite becomes easier to compare with existing Library and organizer tools;
- the product's strongest safety and lifecycle work stays hidden or underused;
- future file-changing features risk creating separate engines and inconsistent rules.

### Decision

Keep the lifecycle-companion direction, but execute it through one narrow golden journey at a time.

## 3. Core player promise

A player should be able to answer, in plain language:

- What did I download?
- What is inside it?
- Where should it go?
- Is it already installed or duplicated?
- What does SimSuite know, suspect, or not know?
- What would SimSuite change?
- Can I undo it?
- Is my setup ready for the game and the current patch?

SimSuite should never claim certainty it does not have.

## 4. Product hierarchy

SimSuite should feel like one connected desktop product with calm workspaces, not one long dashboard and not a pile of unrelated screens.

### Home: readiness and next action

Home answers:

- Is my setup ready?
- What needs attention?
- What changed recently?
- What should I do next?

Home should be glanceable. It should not become a menu of every feature.

### Inbox: arrival and intake

Inbox owns new downloads and imported content before installation.

It explains archives, files, likely destinations, duplicates, risks, and missing information. It does not silently install.

### Library: installed-state truth

Library is the source of truth for what is currently installed in Mods and Tray.

It supports browsing, search, folders, thumbnails, details, evidence, creator/category corrections, and links into review workflows. It is central, but it is not the whole product.

### Review: uncertainty and decisions

Review collects cases that need human judgement:

- weak classifications;
- ambiguous versions;
- unknown creators;
- possible duplicates;
- placement questions;
- unsupported update sources.

It should explain why each item needs attention and route the decision back into the relevant workflow.

### Organize: planning, not ownership

Organize prepares reversible plans that respect the player's chosen structure.

It should support preserve-existing, creator-first, content-type-first, gameplay-first, source-first, collections, and custom hybrid rules. No single structure is treated as universally correct.

### Updates and Patch Day: maintenance

Updates owns reliable source checks, installed-version history, patch-risk review, and player decisions. It should not overclaim official or latest truth when evidence is weak.

### Recovery and History: trust

Every future file-changing action should produce a clear transaction receipt, verification result, and bounded undo path. Recovery should be visible as a normal product capability, not an emergency afterthought.

### Tools: advanced diagnostics

Technical and creator-facing diagnostics belong in a quieter advanced area or contextual detail panels. They should not dominate the default player journey.

## 5. Three independent user choices

The app should not overload one mode switch with every preference.

### Experience view

- Casual: plain language, guided actions, compact evidence.
- Seasoned: more filters, batch review, paths and version context.
- Creator/Technical: complete evidence, package details, logs, identifiers, and advanced diagnostics.

All views use the same facts and safety decisions.

### Automation policy

- Manual: explain only; every change is explicitly approved.
- Guided: prepare plans; the player approves each batch.
- Assisted: approved low-risk rules may run, while ambiguity stops for review.
- Custom: technical users define detailed boundaries, exclusions, and thresholds.

Automation policy must be separate from experience view.

### Organization profile

Examples:

- preserve current structure;
- tidy obvious mistakes only;
- creator-first;
- content-type-first;
- gameplay-purpose-first;
- source or collection-first;
- custom hybrid.

Every profile requires preview, exclusions, protected paths, and recovery coverage.

## 6. Technical backbone

### Installation Profile V1

This is the next architecture gate.

A profile should represent:

- profile ID and display name;
- game ID;
- operating environment;
- user-data root;
- Mods root;
- Tray root;
- approved Downloads/Inbox roots;
- root identities and filesystem capabilities;
- detection evidence;
- explicit confirmation state;
- profile status and last validation time.

Manual selection must always work. Detected candidates must be ranked and explained. Ambiguous candidates must never be selected silently.

### Sims4Adapter

The Sims 4 adapter owns:

- expected user-data layouts;
- Mods and Tray placement rules;
- supported file types;
- script-depth rules;
- DBPF/package inspection;
- thumbnail and cache rules;
- patch/version detection;
- Sims 4 readiness checks;
- curated special-mod knowledge.

The adapter must consume platform capabilities rather than contain Windows, macOS, Wine, or Proton mechanics itself.

### Shared evidence model

Important conclusions should carry one of:

- confirmed;
- strongly supported;
- possible;
- unknown.

This applies to creator identity, category, duplicates, mod family, version replacement, dependencies, conflicts, and safe removal.

### Shared transaction engine

All future file-changing workflows must use one engine for:

1. plan generation;
2. validation;
3. exact operation snapshot;
4. confirmation boundary;
5. recovery preparation;
6. execution;
7. verification;
8. receipt and audit history;
9. bounded undo.

Installation, organization, duplicate cleanup, patch profiles, 50/50 testing, archiving, and safe removal must not invent separate move logic.

### Incremental index and provenance

Each managed item should gradually gain:

- profile and root identity;
- relative path identity;
- content fingerprint when needed;
- source/archive identity;
- installation transaction identity;
- creator, family, and version evidence;
- user corrections;
- protected relationships;
- change history.

## 7. Golden product milestone

The first meaningful beta milestone is not "the Library looks finished."

It is:

> A player can take one supported Sims 4 download from arrival to an installed, understood, verified, and undoable result without manually managing files.

This journey must be proved first with isolated fixtures, not live Sims folders.

Required fixture cases:

1. one `.package` file;
2. one `.ts4script` mod;
3. one normal archive;
4. one Tray bundle;
5. one curated special mod with configuration or sidecar files.

For every case, prove:

1. arrival is detected;
2. contents are explained;
3. the correct profile and root are selected;
4. destination and organization rules are proposed;
5. uncertainty and blockers are visible;
6. recovery material is prepared;
7. the fixture-only transaction executes;
8. placement and hashes are verified;
9. only affected index entries refresh;
10. undo restores the exact previous fixture state.

## 8. Execution sequence

### Phase A: close the platform foundation

1. Record hosted Windows, macOS, and Linux CI evidence when authenticated access is available.
2. Verify path semantics on native Windows and Linux hosts.
3. Keep native CI, native desktop proof, and fixture proof as separate evidence categories.
4. Preserve the current preview-only command gates.

### Phase B: Installation Profile V1 and Sims4Adapter

1. Define profile, root, environment, evidence, and confirmation models.
2. Add manual profile creation and validation first.
3. Add ranked macOS candidates without silent selection.
4. Add Windows Documents/OneDrive candidate evidence.
5. Add Linux manual profiles before Wine/Proton/Lutris discovery.
6. Migrate global Mods/Tray settings behind a compatibility layer.
7. Make presentation show which profile and root produced each decision.

### Phase C: complete shared path identity adoption

Integrate profile/root-relative identity into small verified batches:

1. duplicate same-path exclusion and destination identity;
2. scanner folder keys;
3. Library folder scoping and queries;
4. Downloads/Inbox intake identity;
5. hidden move preflight;
6. snapshot and recovery records.

Do not perform a broad path rewrite in one batch.

### Phase D: fixture-only transaction core

1. Create a hidden fixture executor unavailable to normal users.
2. Use backend-owned operation snapshots only.
3. Prepare recovery material before every fixture mutation.
4. Verify every operation and record observed results.
5. Prove bounded undo.
6. Add failure injection for partial-copy, permission, collision, changed-source, and interrupted-run cases.
7. Do not issue real confirmation tokens or register a real user-folder executor.

### Phase E: connected golden journey

Connect Inbox, explanation, install plan, fixture transaction, Library refresh, receipt, and undo for the five fixture cases.

Keep the UI simple:

- explain first;
- show the proposed change;
- show blockers and uncertainty;
- show recovery coverage;
- verify the result;
- offer undo.

### Phase F: native proof and low-spec budgets

1. Add macOS fixture desktop automation.
2. Run the existing Windows fixture proof on a real Windows host.
3. Add Linux native launch, file-manager, and manual-profile proof.
4. Define Light, Balanced, and Fast resource profiles.
5. Benchmark cold start, idle memory, scan, incremental refresh, search, folder opening, thumbnails, cancellation, and background CPU.
6. Fix backend-native folder queries and lazy media paths before loading very large libraries through the frontend.

### Phase G: first safe beta boundary

A beta discussion may begin only when:

- the golden fixture journey passes on all required cases;
- transaction recovery and failure injection pass;
- native Windows and macOS proof pass;
- Linux support is honestly classified and documented;
- low-spec budgets are measured;
- user-facing uncertainty and safety copy are accurate;
- no legacy mutating command bypass exists.

Real user-folder Apply remains a separate explicit release decision.

### Phase H: patch day, troubleshooting, and safe removal

After the transaction engine is trusted:

- play-readiness checks;
- pre-patch snapshot and patch-safe profile;
- high-risk script review;
- reliable-source update tracking;
- guided disable/test/restore groups;
- 50/50 troubleshooting workflow;
- archive instead of delete;
- safe-removal preflight;
- stronger relationship and dependency evidence where technically supportable.

### Phase I: advanced automation and later game expansion

Only after deterministic rules and recovery are trusted:

- user-approved intake recipes;
- automatic organization within explicit boundaries;
- recurring collection-health checks;
- bounded patch-day routines;
- optional AI explanations for ambiguous evidence.

AI must not become the authority for file identity, safety, placement, or deletion.

Multi-game expansion starts only after the Sims 4 journey is complete and stable.

## 9. Immediate next three implementation sprints

### Sprint 1: Installation Profile V1 foundation

Deliver:

- profile and root models;
- manual profile creation and validation;
- explicit confirmation state;
- compatibility mapping from current global settings;
- root capability evidence display;
- tests for missing, ambiguous, relocated, and custom roots.

No automatic selection and no file mutation.

### Sprint 2: Sims4Adapter discovery and profile UX

Deliver:

- Sims 4 default-layout rules behind the adapter;
- ranked macOS discovery candidates;
- Windows OneDrive/custom Documents evidence model;
- Linux manual environment model;
- simple profile setup UI shared across Casual, Seasoned, and Creator views;
- same truth, different explanation tests.

### Sprint 3: profile-aware identity integration

Deliver:

- duplicate path identity;
- scanner folder identity;
- Library folder scoping;
- Downloads/Inbox identity;
- migration tests from legacy absolute settings;
- no hidden change to existing player files.

After these three sprints, begin the fixture-only transaction core.

## 10. What should not be built next

- more top-level screens;
- a second transaction or move engine;
- real Apply or Restore against player folders;
- automatic duplicate cleanup;
- broad AI classification or autonomous file decisions;
- multi-game abstractions beyond the narrow adapter boundary;
- forced organization templates;
- decorative dashboard complexity;
- expensive always-on scanning;
- new update/watch breadth before the core golden journey is connected;
- claims of dependency, safe delete, or patch compatibility without deterministic evidence.

## 11. Product quality rules

Every feature should pass these questions:

1. Which player-journey stage does this improve?
2. Does it use the same evidence and transaction backbone?
3. Is uncertainty visible?
4. Is the action previewable and recoverable?
5. Does it work for Casual, Seasoned, and Creator views without changing truth?
6. Does it remain usable on a low-spec machine?
7. Does it avoid loading or parsing more than needed?
8. Is native platform evidence separated from CI evidence?
9. Does it keep unsupported cases safely blocked?

## 12. Current decision checkpoint

The cross-platform path-semantics foundation and read-only ApplyPlan validation are implemented, committed, verified, and published on `platform/cross-platform-foundation`.

The next product and architecture gate is **Installation Profile V1 with the Sims4Adapter boundary**.

The next milestone after profiles and shared identity adoption is the hidden, fixture-only golden journey. Real Apply, Restore, delete, replace, quarantine, cleanup, and automatic mutation remain disabled.
