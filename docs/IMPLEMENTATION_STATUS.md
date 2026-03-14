# SimSuite Implementation Status

This document maps the current implementation to the active product requirements.

## Current session note (March 14, 2026)

This session tightened the first real user-facing watch flow and verified it in the real Tauri app.

Important changes and findings:

- fixed a role mix-up between `Library` and `Downloads`:
  - `Library` queries now exclude `downloads` rows
  - `Library` detail and watch actions now only work on installed items
  - this keeps `Library` as the installed-content desk instead of turning it into another Inbox view
- added stricter watch-source saving rules:
  - secure `https` links only
  - no embedded sign-in details in saved links
  - downloads rows are rejected instead of pretending they are watchable Library items
- updated the preview mocks so browser-preview testing follows the same installed-only Library rule
- extended the native desktop smoke harness:
  - added one generic installed fixture file for watch-flow checks
  - the smoke run now triggers a real installed scan before checking Library watch actions
  - the real Tauri app now proves that SimSuite can save and clear a watch source for an installed Library item
- researched CurseForge as a future watch provider using official sources:
  - CurseForge does have an official API path for 3rd-party apps
  - it requires applying for an API key
  - project owners can disable 3rd-party distribution on their projects
  - this means CurseForge should be treated as a formal provider integration, not a scraping fallback

Important remaining gap:

- the first watch flow is now real, but it is still small:
  - one item at a time
  - manual save or clear
  - no broader watch setup flow yet
- CurseForge support should only be considered through the approved API and terms
- the next product work should focus on a clean installed-content watch setup flow before adding provider-specific complexity

## Current session note (March 14, 2026)

This session was a smaller follow-up checkpoint to verify that the Lumpinou Toolbox same-version fix really holds in the live desktop app.

Important changes and findings:

- fixed two small app build blockers that were preventing a fresh native desktop verification pass:
  - `src/lib/api.ts` was using `WatchSourceKind` without importing it
  - `src/screens/LibraryScreen.tsx` had a local state value and a helper function using the same name
- removed the temporary live-database debug test after the live check was done, so the repo stays clean
- rebuilt the real debug Tauri app successfully
- confirmed the live Lumpinou Toolbox case in the real app with a read-only desktop check:
  - the Inbox queue row now shows `Installed and incoming match`
  - the selected item panel now shows `Already current`
  - the primary action now shows the reinstall path instead of a cautious unclear state
  - the live version section showed:
    - installed `1.179.6`
    - incoming `1.179.6`
    - compare `Installed and incoming match`
    - same-release reinstall evidence when fingerprints differ

Important remaining gap:

- broader live desktop validation is still strongest as targeted spot checks plus fixture-backed flows
- the next product work can now move back to watch flow and careful catalog growth instead of staying stuck on the Lumpinou version issue

## Current session note (March 13, 2026)

This session moved SimSuite from a mostly special-mod-only version story to one shared version and update-watch foundation for all content.

Important changes and findings:

- the backend now has one shared version-and-match layer for all content:
  - `file_inspector` collects structured `versionSignals`
  - `content_versions` builds local content subjects, finds the best installed match, and returns compare status plus confidence
  - weak matches stay `unknown` instead of pretending to know
- `versionHints` are still kept as the short compatibility summary, but they are now derived from stronger structured signals
- Inbox can now compare normal mods and CC too when the local installed match is strong enough
- the compare result now has a separate confidence level:
  - `exact`
  - `strong`
  - `medium`
  - `weak`
  - `unknown`
- signature matches still win as the strongest same-version proof
- if version labels match but fingerprints do not, the result stays cautious instead of calling it current
- supported special mods now sit on top of the same shared foundation instead of using a separate version world
- `versionStrategy` is now active in real profile data for the built-in supported special mods:
  - the seed model now reads `versionStrategy` correctly
  - inside-file clues can win over names where that is the right rule
  - old stored `versionHints` can still help as a migration bridge when `versionSignals` are missing
- the current built-in supported mods now run on that profile-driven version-rule path:
  - MCCC
  - XML Injector
  - Lot 51 Core Library
  - Sims 4 Community Library
  - Lumpinou Toolbox
  - Smart Core Script
- Lumpinou Toolbox is now a proof case for the new rule layer:
  - noisy runtime clues are no longer enough on their own
  - cleaner local filename clues can take priority
- Library now has installed-version awareness without turning into another Inbox:
  - selected detail shows installed version summary
  - selected detail shows local version evidence
  - selected detail shows watch status
  - Library list rows are still kept simple
- Home now rolls up the broader update-watch picture without adding more stacked boxes:
  - exact updates
  - possible updates
  - unknown watch state
- generic watch results now have a proper model:
  - exact page vs creator page
  - current vs exact update vs possible update vs unknown
  - helper-only status that does not override local Inbox truth
- the long-term growth scaffolding is now in the repo:
  - `docs/SPECIAL_MOD_ONBOARDING.md`
  - `docs/SPECIAL_MOD_CANDIDATES.json`
- the frozen external Sims mod index stays reference-only and is not used as runtime truth or a maintenance source
- the real fixture-backed desktop smoke still passes after the shared version foundation work
- the current shared foundation builds on the earlier Inbox performance work instead of undoing it:
  - queue stays light
  - selected detail keeps the heavier evidence work
  - no new network dependence was added to the local compare hot path

Important follow-up result:

- the earlier Inbox speed work still holds:
  - real live first-open is still about `1.07s`
  - the queue still stays light
  - selected special detail is still the heavier path at about `1.95s`
- the special-mod rework did not pull the app back into the old freeze state
- the real fixture-backed desktop smoke still proves the current built-in supported special-mod flows
- the app now has a path to scale beyond the current six supported special mods without growing a new Rust branch for every version rule change

Important remaining gap:

- the next missing product layer is deeper user-facing watch management:
  - the app can show watch results now
  - Library detail can save or clear an approved watch source for one installed subject
  - but broader batch setup, editing flows, and helper-only polling still need careful product work
- helper-only official latest support is still narrow where the source is not safely readable by plain app requests
- deeper non-MCCC apply and repair desktop checks still need to be widened
- the first curated post-foundation expansion wave has not started yet
- heavy selected-item special-mod detail is still slower than the queue in real desktop use
- the raw debug Tauri desktop smoke lane still expects the local Vite frontend to be reachable at `http://localhost:1420`

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
- Downloads queue rows now stay on light lane and summary logic while the selected item panel carries the full special-mod compare work
- Inbox startup now begins from a real watcher state and retries locked reads more gracefully

Still not solved well enough:

- Inbox is much steadier now in real desktop use
- the main remaining live cost is richer selected-item special-mod detail, not queue open or basic refresh
- the next session can go back to broader supported special-mod coverage and only return to performance if selected-item detail still feels too heavy

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
- XML Injector helper-only latest parsing from a safe readable official page
- one shared special-mod decision result feeding queue, side panel, and main action state more consistently

Missing:

- user-extensible local catalog packs
- broader curated incompatibility coverage beyond the initial seed set
- auto-resolving multi-item dependency install order inside Inbox
- guided option-pack choice flows
- deeper Inbox performance cleanup for large live queues and heavy special-mod families after the duplicate rebuild fix
- deeper live-scan performance cleanup now that selection and refresh responsiveness are materially better
- final cleanup of stale Inbox ownership and repeated special-mod recomputation during interactive use
- deeper native desktop apply and blocked-flow coverage beyond the current base lane that now covers all six built-in supported families
- a clean product decision for unsupported unrelated archive types:
  - keep `.7z` and `.rar` visible as safety-held intake items
  - or add a stricter ignore path that still avoids hiding real Sims archives by mistake

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
- the native Tauri desktop base smoke now covers all six built-in supported special-mod families

Missing:

- deeper archive-content heuristics for unsupported/edge archive layouts
- dedicated watcher controls beyond the current general Settings surface
- final removal of any remaining real-world Inbox hangs during heavy live-folder use
- helper-only official latest parsing is still too narrow for several supported mods because some official sources are still blocked by challenge pages for plain app requests

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

The highest-value next step is to finish the first user-facing layer on top of the new shared version and update-watch foundation:

1. add a careful watch-source flow for installed content
2. widen helper-only official latest parsing only where there is a safe official endpoint the app can fetch without brittle bypass work
3. expand the real desktop special-mod fixture lane deeper for non-MCCC apply and blocked flows
4. use `docs/SPECIAL_MOD_ONBOARDING.md` and `docs/SPECIAL_MOD_CANDIDATES.json` for the first small post-foundation expansion wave
5. keep watching selected-item Inbox detail performance so the broader compare system does not make the screen feel heavy again

After that first layer is solid, the next large product steps remain:

1. snapshot-backed duplicate cleanup actions
2. full Mirror Mode / Assisted Migration / Fresh Setup workflows
3. broader special-mod catalog curation and dependency coverage
4. editable rule templates and presets in the UI

After those are complete, the next effort should be Phase 8:

1. local AI classification integration
2. AI schema validation tests

Patch recovery should stay after those phases, consistent with the planned development order.
