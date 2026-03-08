# SimSuite Implementation Status

This document maps the current implementation to the active product requirements.

## Fully implemented or materially in place

### Platform and storage

- Tauri + React + TypeScript frontend shell
- Rust backend core
- SQLite schema and migrations
- seed loading for creators, aliases, taxonomy, keyword dictionaries, and rule presets
- expanded local creator alias and category keyword knowledge base informed by current Sims creator naming patterns
- separate user-learned creator alias storage layered on top of seed data without overwriting seed packs
- local schema evolution support for new scan metadata such as file inspection insights

### Core safety pipeline

Implemented in Rust and exposed through Tauri commands:

- scanner
- filename parser with creator window matching, bracketed-tag matching, camel-case tokenization, phrase-aware subtype detection, and creator recognition heuristics
- folder-aware creator hinting from nearby path segments
- local file inspection for `.ts4script` and `.package` content hints before rule evaluation
- user creator overrides, learned aliases, and locked creator path preferences that feed future scans and preview routing
- user category overrides that persist across rescans and win over heuristic classification
- batch category clustering and batch category learning from unresolved files
- bundle detector for tray content
- duplicate detector for exact, filename, and version duplicates
- rule engine with seeded presets
- validator for script depth, tray placement, depth limiting, and collisions
- preview generation
- approval-gated move engine
- snapshot creation and rollback
- read-only library index queries

### Current tests

Automated Rust tests exist for:

- filename parsing
- rule preset evaluation
- validator safety rules
- filesystem move simulation
- rollback reliability

## Partially implemented

### Scanner

Implemented:

- Mods scanning
- Tray scanning
- metadata extraction
- folder-based creator hinting
- internal inspection of `.ts4script` namespaces and `.package` DBPF resources, including compressed resource decoding for common Sims package formats
- selective hashing for duplicate candidates
- review queue seeding
- incremental scan cache reuse for unchanged files
- background scan worker and scan status events
- tray bundle rebuild
- duplicate rebuild

Missing:

- downloads folder scanning
- deeper scan prioritization / scheduling controls

### Duplicate detection

Implemented:

- exact duplicate detection via SHA-256
- filename duplicate detection
- version duplicate detection
- Duplicates screen for pair inspection

Missing:

- safe duplicate actions with snapshot-backed approval

### Rule engine and organization modes

Implemented:

- preset-driven previews
- basic detected-structure labeling
- validator-corrected path generation
- creator/type enrichment from filename hints, folder hints, and inspected file contents before previews are generated
- locked creator preferred paths overriding preset output when the user has explicitly fixed that creator's routing

Missing:

- full Mirror Mode behavior
- Assisted Migration Mode workflow
- Fresh Setup Mode workflow
- editable custom rules and templates in the UI

### UI coverage

Implemented:

- Home
- Library
- Duplicates
- Organize
- Review
- Creator Audit
- Category Audit
- compact Library inspector controls for saving creator overrides and learned aliases
- compact Library inspector controls for manual category overrides
- batch creator clustering and batch creator learning from unresolved files
- batch category clustering and batch category learning from unresolved files

Missing:

- Tray
- Patch Recovery
- Tools
- Settings

## Not implemented yet

### AI classification

Current state:

- module exists as a placeholder only
- current creator/category improvements are still fully local and deterministic; AI fallback is not required for the cases now covered by seed, path, and inspection hints

Missing:

- Ollama or llama.cpp integration
- strict JSON classification interface
- review queue fallback from AI
- AI schema validation tests

### Downloads watcher and archive intake

Current state:

- module exists as a placeholder only

Missing:

- downloads monitoring
- archive detection
- archive extraction and intake pipeline

### Patch recovery

Current state:

- snapshot primitives exist

Missing:

- patch recovery screen
- snapshot comparison tools
- creator grouping for recovery
- mod isolation and hold flows

## Important implementation notes

- The current backend does satisfy the rule that file operations are approval-gated and snapshot-backed.
- The current backend does satisfy the rule that AI never moves files directly, because AI is not wired into file movement at all yet.
- The current creator and parser improvements remain fully local/offline; public web sources were used only to strengthen seed data and implementation research, not as a runtime dependency.
- `.package` inspection does not rely on a universal embedded creator field, because Sims package files do not expose one consistently. Creator inference is therefore layered across filename, folder, script namespace, and embedded-name hints.
- User-learned creator aliases are now stored in SQLite and merged into the runtime recognition pack for later scans, instead of being baked into seed files.
- The Creator Audit workflow now works from the indexed database instead of the raw filesystem, so batch creator cleanup stays fast even on large libraries.
- Manual category overrides are stored separately from seed data and are intended to outrank both heuristics now and AI fallback later.
- The Category Audit workflow now works from the indexed database instead of the raw filesystem, so batch category cleanup stays fast even on large libraries.
- The Library screen now exposes inspection metadata such as detected format, script namespaces, creator hints, resource summaries, and embedded names for Standard and Power views.
- The current backend now materially covers the planned work through Phase 6 and the scan-performance parts of Phase 7.
- The previous `docs/ARCHITECTURE.md` statement that moves were still disabled was outdated and has been corrected.

## Recommended next effort

The highest-value next step is to finish Phase 7 properly before moving on:

1. downloads watcher
2. archive intake
3. downloads/inbox workflow

After Phase 7 is complete, the next effort should be the remaining safe duplicate actions and then Phase 8:

1. snapshot-backed duplicate cleanup actions
2. local AI classification integration
3. AI schema validation tests

Patch recovery should stay after those phases, consistent with the planned development order.
