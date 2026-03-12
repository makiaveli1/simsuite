# SimSuite Implementation Status

This document maps the current implementation to the active product requirements.

## Current session note (March 12, 2026)

This session did two important things:

1. it proved more of Inbox against the real Tauri desktop app instead of only the browser preview
2. it added a simple repo memory and handoff process so the next session can pick up quickly

Important changes and findings:

- Inbox refresh work was tightened again after a regression report:
  - local Inbox actions were still causing too many queue reload paths to pile up
  - the Downloads screen now keeps one main post-action reload path instead of mixing local reloads, watcher follow-up reloads, and workspace-triggered reloads back to back
  - watcher status refresh after local actions now happens in the background instead of blocking the whole Inbox action path
  - the right panel now reloads on selected-item `id` and `updatedAt`, instead of every queue rebuild
- the real Tauri desktop fixture app still passes after that Inbox reload cleanup:
  - open Inbox
  - refresh Inbox
  - apply MCCC update
  - keep version evidence and post-apply state correct
- special-mod review links are now checked more strictly before SimSuite opens or downloads anything
- `.7z` and `.rar` downloads are now held for review instead of being unpacked automatically
- watcher refresh now has a narrower path for ordinary file-system events instead of always rescanning the whole Downloads tree
- special-mod queue rows now use a lighter summary while the selected item keeps the full detail and evidence
- a real Tauri desktop smoke lane now exists with isolated fixture folders, so Inbox can be tested without touching the user's real Mods or Downloads folders
- the real desktop MCCC flow was checked end to end, including selection, version evidence, refresh, and safe apply
- the local compare result in the tested MCCC flow was accurate:
  - installed `2025.9.0`
  - incoming `2026.1.1`
  - result `Incoming pack is newer`
- after safe apply, SimSuite updated the installed version to `2026.1.1` and preserved the `.cfg` settings file
- the post-apply family-ranking bug has now been fixed in backend logic:
  - applied fuller family packs stay in the sibling comparison instead of disappearing
  - weaker leftover siblings no longer get the wrong “open the other Inbox item first” action in the backend decision path
- a new backend regression test now covers that exact post-apply MCCC family case
- a watcher startup bug was fixed so the first Inbox refresh no longer fails silently and leaves the watcher stuck in `processing`
- archive staging roots are now unique per fresh source, so two new downloads arriving in the same second do not contaminate each other
- the post-apply full-pack item now reuses the installed family anchor instead of being misread as an incomplete fresh download
- covered leftover special-mod items now drop stale “download missing files” actions once a fuller family pack is already installed
- the blocked leftover panel wording now matches the backend truth better instead of still sounding like an unresolved blocker
- the native Tauri smoke wrapper is now steadier:
  - it reads body text in a safer way
  - it survives short body-refresh gaps after apply
  - apply mode follows a simpler path instead of doing extra pre-apply refresh work
  - it runs `tauri build -- --debug` by default so it uses the real desktop app surface
- `.ts4script` same-version comparison is now more trustworthy for supported special mods:
  - SimSuite now hashes the real inner script contents instead of trusting the outer zip wrapper bytes
  - this avoids false “unknown” results when two logically identical script mods were zipped at different times
- the native desktop smoke lane is now more trustworthy too:
  - it clicks the real Inbox queue row buttons by item name instead of any matching text on the page
  - that removed a false failure where the smoke test was reacting to a different item elsewhere in the Inbox
- XML Injector is now covered better in the real desktop fixture app:
  - same-version flow
  - older-version flow
  - version evidence display
- Sims 4 Community Library is now covered too:
  - same-version flow
  - older-version flow
  - real desktop version evidence display
- a live helper-only latest check gap was confirmed:
  - direct app-style requests to CurseForge and Lot 51 still hit Cloudflare challenge pages
  - this means the remaining helper-only latest gaps need safe official endpoints, not brittle workarounds

Important follow-up result:

- the original post-apply family bug was real and is now fixed in backend code, UI wording, and real desktop checks
- the real desktop base Inbox smoke passes
- the real desktop special-mod apply smoke passes
- the real desktop base smoke now also proves XML Injector same-version and older-version handling
- the real desktop base smoke now also proves Sims 4 Community Library same-version and older-version handling
- the real desktop XML Injector same-version result is now correct:
  - installed `4.0`
  - incoming `4.0`
  - result `Installed and incoming match`
  - inner-file evidence wins over outer zip noise
- the real desktop Sims 4 Community Library result is now correct:
  - same-version downloads settle into the already-current path
  - older downloads stay out of the update path
  - local evidence still drives the result in the real app
- the current fixture-backed real desktop result for MCCC after apply is now:
  - the full pack lands in the done lane
  - the full pack reads as matching the installed version
  - the leftover partial pack reads as already covered by the fuller installed family pack
  - the leftover pack recommends ignoring the archive instead of trying to fetch another copy first

Important remaining gap:

- the latest Inbox performance cleanup is proven in the real fixture-backed desktop app, but it still needs a live check against the user's real Downloads folder and real queue size
- helper-only official latest support is still too narrow for supported special mods whose official pages are readable today
- direct non-browser requests to CurseForge and Lot 51 are still blocked by Cloudflare, so those helpers need a safe official machine-readable source before they can be widened in the app

Repo memory is now expected to live in:

- `SESSION_HANDOFF.md` for the current baton-pass
- `docs/IMPLEMENTATION_STATUS.md` for broader progress
- `docs/ARCHITECTURE.md` only when real structure or behavior changes

## Previous session note (March 11, 2026)

This session focused on two connected areas:

1. making the special-mod Inbox logic more trustworthy
2. reducing repeated Inbox work and startup mistakes

Important changes already landed:

- special-mod support now uses stronger per-mod rules instead of loosely sharing MCCC behavior
- special-mod decisions now use a clearer family model so related downloads can be compared together
- local installed-vs-downloaded comparison is now the main update decision path
- internal file inspection now feeds special-mod identity and version checks
- official latest checks remain helper-only
- same-version downloads can be treated as already current
- MCCC update handling now preserves `.cfg` settings and tolerates disk-only older files during replace steps
- trusted “open official page” handling was fixed to use the real browser path
- workspace refresh moved toward targeted domain invalidation instead of broad reloads
- Downloads queue loading and selected-item loading were split to reduce repeated heavy work
- Inbox startup now begins from a real watcher state and retries locked reads more gracefully

Still not solved well enough:

- Inbox is still the main performance and stability pain point in real desktop use
- the page can still hang or feel heavy when selecting items or waiting for richer special-mod details
- the next session should investigate real desktop Inbox timings and repeated work before adding more special-mod coverage

## Fully implemented or materially in place

### Platform and storage

- Tauri + React + TypeScript frontend shell
- Rust backend core
- SQLite schema and migrations
- seed loading for creators, aliases, taxonomy, keyword dictionaries, and rule presets
- built-in special mod catalog seed for guided profiles, dependency rules, incompatibility rules, and review-only patterns
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
- special mod catalog engine for guided installs, dependency checks, incompatibility warnings, and review-only download patterns

### Current tests

Automated Rust tests exist for:

- filename parsing
- rule preset evaluation
- validator safety rules
- filesystem move simulation
- rollback reliability
- special mod catalog assessment, dependency review paths, guided update plans, false-positive avoidance, and rollback-backed guided installs

## Partially implemented

### Scanner

Implemented:

- Mods scanning
- Tray scanning
- downloads watcher-backed intake indexing for supported direct files and extracted archive contents
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

### Special mod catalog and Inbox routing

Implemented:

- built-in guided install catalog seeded from local curated data
- built-in dependency rule catalog seeded from local curated data
- built-in incompatibility warnings seeded from local curated data
- review-only pattern catalog for option packs and manual-step archives
- MCCC guided first install and guided update flow
- XML Injector guided flow
- Lot 51 Core Library guided flow
- Sims 4 Community Library guided flow
- Lumpinou Toolbox guided flow
- Smart Core Script guided flow
- guided install routing based on staged evidence, installed-layout checks, dependency checks, and incompatibility checks
- Inbox routing into `Normal`, `Special setup`, `Needs review`, or `Blocked`
- special review plans for downloads that match a special pattern but cannot be auto-applied safely
- dependency status checks against already-installed libraries and other active Inbox items
- snapshot-backed guided apply with preserve-file handling for profile sidecars such as MCCC `.cfg` files
- local-first special-mod version comparison using downloaded packs, installed files, saved family state, and file-signature fallback
- internal file inspection hints feeding special-mod identity and version evidence
- special-mod family grouping so duplicate downloaded versions can be compared together
- helper-only official latest checks for reviewed built-in special mods
- one shared special-mod decision result feeding queue, side panel, and main action state more consistently

Missing:

- user-extensible local catalog packs
- broader curated incompatibility coverage beyond the initial seed set
- auto-resolving multi-item dependency install order inside Inbox
- guided option-pack choice flows
- deeper Inbox performance cleanup for large live queues and heavy special-mod families after the latest refresh-deduping pass
- final cleanup of stale Inbox ownership and repeated special-mod recomputation during interactive use
- full native desktop fixture coverage beyond the current MCCC, XML Injector, and Sims 4 Community Library smoke lane

### UI coverage

Implemented:

- Home
- Downloads
- Library
- Duplicates
- Organize
- Review
- Settings
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

Implemented:

- downloads folder monitoring
- inbox indexing for supported direct downloads
- archive detection for `.zip`, `.7z`, and `.rar`
- staged archive extraction into app-managed intake folders
- downloads watcher status events
- Downloads screen with queue, safe preview, guided special setup, review/blocked states, apply, and ignore flows
- Inbox bootstrap loading so first-open Downloads can begin from the real watcher state instead of a guessed empty/setup state
- locked-read retries for read-only Inbox commands
- targeted `downloads-sync-finished` workspace change event after watcher passes complete
- watcher startup now reports a real error state if the first refresh fails instead of silently staying in `processing`
- archive staging roots now use a unique timestamp plus source name so new downloads do not share one staging folder
- a native Tauri desktop smoke wrapper now launches an isolated fixture app, builds the real desktop app when needed, and can cover both base Inbox flow and a safe MCCC apply flow

Missing:

- deeper archive-content heuristics for unsupported/edge archive layouts
- dedicated watcher controls beyond the current general Settings surface
- final removal of any remaining real-world Inbox hangs during heavy live-folder use
- helper-only official latest parsing is still too narrow for several supported mods because some official sources are still blocked by Cloudflare for plain app requests

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
- The special mod catalog is also fully local/offline at runtime. Mod Hound-style knowledge was used only during curation of the built-in seed data; the app does not call Mod Hound or depend on it at runtime.
- `.package` inspection does not rely on a universal embedded creator field, because Sims package files do not expose one consistently. Creator inference is therefore layered across filename, folder, script namespace, and embedded-name hints.
- User-learned creator aliases are now stored in SQLite and merged into the runtime recognition pack for later scans, instead of being baked into seed files.
- The Creator Audit workflow now works from the indexed database instead of the raw filesystem, so batch creator cleanup stays fast even on large libraries.
- Manual category overrides are stored separately from seed data and are intended to outrank both heuristics now and AI fallback later.
- The Category Audit workflow now works from the indexed database instead of the raw filesystem, so batch category cleanup stays fast even on large libraries.
- The Library screen now exposes inspection metadata such as detected format, script namespaces, creator hints, resource summaries, and embedded names for Standard and Power views.
- The current backend now materially covers the planned work through Phase 7, including downloads intake and inbox review.
- The current backend now materially covers Phase 7.6 special-mod routing for the first curated wave, including guided setup, dependency review, incompatibility review, and review-only patterns for ambiguous archives.
- The previous `docs/ARCHITECTURE.md` statement that moves were still disabled was outdated and has been corrected.

## Recommended next effort

The highest-value next step is to stabilize Inbox fully before moving on to broader feature growth:

1. measure the real desktop Inbox slow paths and lock windows
2. remove repeated special-mod and detail-panel work from interactive Inbox flows
3. keep Home, Library, Organize, Review, and Duplicates correctly synced without broad refreshes
4. widen helper-only official latest parsing only where there is a safe official endpoint the app can fetch without brittle bypass work
5. expand the real desktop special-mod fixture lane beyond MCCC, XML Injector, and Sims 4 Community Library

After Inbox is solid again, the next large product steps remain:

1. snapshot-backed duplicate cleanup actions
2. full Mirror Mode / Assisted Migration / Fresh Setup workflows
3. broader special-mod catalog curation and dependency coverage
4. editable rule templates and presets in the UI

After those are complete, the next effort should be Phase 8:

1. local AI classification integration
2. AI schema validation tests

Patch recovery should stay after those phases, consistent with the planned development order.
