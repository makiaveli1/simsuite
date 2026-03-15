# SimSuite Implementation Status

This document maps the current implementation to the active product requirements.

## Current session note (March 15, 2026)

This session kept the feature freeze in place and pulled one more safe creator clue out of script mods.

Important changes and findings:

- ts4script manifest parsing now also reads safe author and creator fields, including simple string lists in JSON and YAML-style manifests
- this can improve creator matching when a script mod already names its author inside the file itself
- this is additive only:
  - script mods without manifests still continue through the older clue paths
  - manifests are still not required truth
- added direct regression tests for:
  - JSON manifest author lists feeding creator hints
  - YAML-style manifest author lists feeding creator hints
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `180` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- this makes local script clues a bit better, but the wider stabilization pass still is not done:
  - more messy live-library validation is still needed on generic mods and CC
  - there is still room to inspect more safe inside-file identity clues, especially beyond manifest-heavy cases
  - watch bugs still need cleanup before feature work resumes

## Current session note (March 15, 2026)

This session kept the feature freeze in place and hardened the next missing generic compare path too.

Important changes and findings:

- full compare can now use inspected `family_hints` to search installed rows, so strong local family clues no longer get left out of the candidate-search step
- this widening still stays cautious:
  - very short family labels are skipped
  - only stronger normalized family clues are used to widen the installed candidate pool
- added direct regression tests for:
  - the family-hint shortlist itself staying picky about short values
  - family hints finding the right installed match during full compare even when hashes, filenames, and creators do not line up first
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `178` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- generic compare is firmer again, but the confidence hardening is still not finished:
  - more messy live-library validation is still needed on generic mods and CC
  - there is still room to inspect more safe inside-file identity clues, especially when filenames are weak
  - watch bugs still need cleanup before feature work resumes

## Current session note (March 15, 2026)

This session kept the feature freeze in place and tightened the generic Inbox compare rules too.

Important changes and findings:

- generic compare is now slower to say `not installed`:
  - creator plus version alone now stays `unknown`
  - generic compare now requires a medium-strength incoming identity before it will report `not installed`
  - trusted version clues now only count toward that incoming identity when the version confidence is at least medium
- full compare can now use inspected `creator_hints` to search installed rows, so generic matching can still find good installed candidates even when a creator has not been saved into the database yet
- added direct regression tests for:
  - creator plus version only => `unknown`
  - creator plus family plus version => still `not installed`
  - creator hints can find the right installed match during full compare
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `176` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- generic compare is more trustworthy now, but it is still not fully proven:
  - family-hint candidate loading may still need a careful follow-up pass
  - more real-world live validation is still needed on generic mods and CC, not just fixtures
  - Inbox queue wording may need a small polish pass after more live compare cases are reviewed

## Current session note (March 15, 2026)

This session intentionally paused feature growth and started tightening the shared confidence base instead.

Important changes and findings:

- the first confidence-hardening pass landed in the shared Rust backend:
  - the `Library` watch-setup shortlist now checks real parsed clue values instead of only checking whether JSON field names exist in stored `insights`
  - weak version-only candidates now stay out of watch setup suggestions instead of being pushed toward exact-page setup
  - inspected `creator_hints` now feed the shared subject match tokens, so both watch setup and broader local matching can use creator clues already found inside files
- ts4script inspection now pulls optional identity names from manifest payloads when they exist:
  - this helps matching when a script mod ships a clear internal `name`
  - script mods without manifests still continue through the older local clue paths
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `173` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- this was only the first stabilization pass:
  - the broader generic Inbox installed-match thresholds still need the same audit treatment
  - real live validation is still needed on messy mixed mod and CC libraries, especially where creator hints exist but saved creator links do not
  - watch bugs still need more cleanup before new watch features should resume

## Current session note (March 15, 2026)

This session pushed the watch system one step closer to a fuller `Library` workflow without adding another crowded management screen.

Important changes and findings:

- `Library` now has the next compact watch-management layer inside the existing watch center:
  - strongest exact-page candidates are split into a bulk exact-page strip
  - saved reminder-only and provider-needed links now have a real review queue lane
- `Home` now gets a real watch-review count from backend truth, so the wider app can point users toward that unfinished follow-up work honestly
- fixed a real handoff bug in `Library`:
  - if setup or review started while the inspector was empty, the pending handoff could be lost before the file detail opened
  - `Library` now opens the target file first and then applies the pending watch intent
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests
- `npm run build` passed
- the native desktop smoke passed after it was updated to prove:
  - the generic watch source can enter the review queue in the real app
  - clearing that source returns the item to setup suggestions

Important remaining gap:

- bulk setup and review are better, but they are still not fully finished:
  - exact-page setup is still one URL save at a time, even inside the bulk strip
  - the review queue is compact, but it is not yet a full many-item batch workflow
  - there is still no watch history or source audit trail yet
- the desktop smoke has one current harness caveat:
  - the Wry webdriver still does not reliably select `Library` rows in this watch flow
  - the generic watch smoke now uses the live Tauri command bridge for save and clear, then verifies the real `Library` UI reaction

## Current session note (March 15, 2026)

This session connected the watch system more cleanly across `Home` and `Library`.

Important changes and findings:

- `Home` watch rows now open `Library` with intent instead of only opening the generic Library screen:
  - `Watch setup` lands on the setup suggestions lane
  - `Exact updates` / `Updates ready` lands on the tracked confirmed-updates lane
- the `Library` watch center now highlights and scrolls to the right section when that focused handoff happens, so the user does not have to hunt around the screen
- review flow now moves more like setup flow:
  - if a saved review item no longer needs review after save, clear, or refresh, SimSuite can move on to the next review item
  - review mode now also has a skip action
- the watch center now has direct “start from here” actions inside the existing surface:
  - `Work through setup` / `Set up watched pages`
  - `Work through review` / `Review watched pages`
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests
- `npm run build` passed
- the native desktop smoke passed after it was widened to prove:
  - `Home` can land on the setup lane
  - `Home` can land on the confirmed-updates lane
  - the earlier Library watch save/clear path still works in the real Tauri app

Important remaining gap:

- the watch flow is smoother now, but it is still not true bulk setup or true batch review:
  - users still add one real watch URL at a time
  - there is still no full review lane for many reminder-only or provider-needed links at once
  - there is still no watch history or source audit trail yet

## Current session note (March 15, 2026)

This session made the Library watch follow-up feel less repetitive without adding a new management screen.

Important changes and findings:

- `Library` setup suggestions now behave more like a queue:
  - `Set up` / `Start setup` still opens the existing watch editor
  - after saving one suggestion, SimSuite can move straight to the next strong suggestion instead of making the user go back into the list first
  - setup mode now includes `Skip for now` and `Stop setup`
- tracked watch rows can now show a direct `Review` / `Review source` action for saved user-managed watch links that still need human follow-up:
  - reminder-only creator pages
  - provider-needed exact pages
  - other saved watch rows that are still unclear
- the same existing detail panel now handles both flows:
  - setup opens the editor with the suggested source type and label
  - review opens the editor with the saved watch source already loaded
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests
- `npm run build` passed
- the native desktop smoke passed after it was widened to prove:
  - setup can start from the shortlist
  - a generic watch page can be saved
  - the tracked list can open `Review`
  - the watch source can still be cleared after review

Important remaining gap:

- this is smoother, but it is not full bulk setup yet:
  - users still confirm one watch URL at a time
  - there is still no dedicated batch review lane for many saved reminder or provider-needed links

## Current session note (March 15, 2026)

This session focused on Library responsiveness first because the screen had started showing the same whole-app freezing behavior that Inbox used to have.

Important changes and findings:

- the main Library hot path now runs on background workers instead of the window thread:
  - `get_home_overview`
  - `get_library_facets`
  - `list_library_files`
  - `list_library_watch_items`
  - `get_file_detail`
  - `save_watch_source_for_file`
  - `clear_watch_source_for_file`
  - `save_creator_learning`
  - `save_category_override`
- this matches the earlier Inbox fix pattern, so Library work can still be busy without locking the whole desktop window while Rust is working
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests
- `npm run build` passed
- the native desktop smoke passed after the threading fix

Important remaining gap:

- this should remove the freezing path, but it does not automatically make every Library action fast:
  - `get_home_overview` still does real watch-summary work
  - `get_file_detail` still does deeper version and watch resolution
  - if the real app still feels sluggish after this fix, the next step is trimming those workloads instead of changing the threading layer again

## Current session note (March 15, 2026)

This session added the first real watch-setup shortlist for installed content and wired it into the wider app instead of treating it like a Library-only side feature.

Important changes and findings:

- `Library` now shows a compact watch-setup shortlist inside the existing watch center:
  - beginner label: `Ready to set up`
  - standard label: `Setup suggestions`
- the shortlist only shows installed items that have enough local clues to be worth setting up, but do not already have:
  - a user-saved watch page
  - a built-in supported special-mod watch page
- each setup suggestion includes:
  - subject label
  - creator when known
  - installed version summary
  - suggested watch type
  - a short setup hint
- clicking a setup suggestion opens the existing Library inspector instead of branching into a second watch workflow
- setup suggestions now also have a direct `Set up` / `Start setup` action:
  - it opens the existing watch editor for that installed item
  - it prefills the suggested watch type and label
  - it still leaves the URL to the user, so SimSuite does not invent or guess watch links
- `Home` now shows a real `Watch setup` count from the backend so the wider app can point users toward that unfinished work without extra guesswork in the UI
- `Home` watch rows now jump straight to `Library`, so update and watch summaries can lead directly into the real follow-up surface
- the backend setup scan now accepts both extension styles:
  - `.package` / `.ts4script`
  - `package` / `ts4script`
  - this fixed a real bug where valid installed items could be skipped from the setup shortlist
- the browser-preview mocks now match the real watch-setup response shape
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests
- `npm run build` passed
- the native desktop smoke passed after it was widened to wait for the new setup section in the real Tauri app
- `npm run build` passed again after the `Home` jump links and prefilled setup flow were added
- the native desktop smoke passed again after the wider watch follow-up wiring

Important remaining gap:

- the next step should be a fuller setup flow, not a second summary layer:
  - bulk apply for the strongest exact-page suggestions
  - easier edit and review for saved generic watch pages
  - a cleaner decision flow for creator-page suggestions

## Current session note (March 15, 2026)

This session turned the Library watch center into a real tracked watch surface instead of a summary-only strip.

Important changes and findings:

- `Library` now has tracked watch filters inside the existing watch center:
  - needs attention
  - confirmed updates
  - possible updates
  - unclear
  - all tracked
- the watch center now shows the actual tracked items behind those counts, not just the counts themselves
- clicking a tracked watch row opens that item in the existing Library inspector
- the backend now builds tracked watch rows from:
  - user-saved watch pages
  - built-in official pages for supported special mods
- built-in supported special mods now show up in that tracked list even before a helper latest row exists, so the UI does not depend on older saved family-state history to notice them
- the browser-preview mocks now expose the same tracked watch list shape
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `168` tests
- `npm run build` passed
- the native desktop smoke passed after it was widened to verify the tracked watch list in the real Tauri app

Important remaining gap:

- the next watch-management step should be setup and review, not another summary layer:
  - bulk setup for unwatched installed items
  - easier editing for generic saved watch sources
  - a clearer review surface for items that could be watched but are not set up yet

## Current session note (March 15, 2026)

This session made the Library watch flow more honest and easier to use without adding a crowded new screen.

Important changes and findings:

- `WatchResult` now carries where the current watch source came from:
  - built-in official page for a supported special mod
  - saved by the user
  - not saved yet
- supported special mods in `Library` no longer pretend their official page was manually saved
- custom watch-source saves for supported special mods are now blocked with a plain explanation, because SimSuite does not yet have a safe merge rule for built-in plus custom watch pages
- `Library` now has a compact watch center inside the existing table panel:
  - confirmed updates
  - possible updates
  - unclear watched items
  - automatic-check state
  - last automatic run
  - `Check watched pages now`
  - quick jump to `Settings`
- selected-item watch actions are now cleaner:
  - built-in special-mod sources do not show misleading add/change/clear buttons
  - user-saved generic sources still keep the normal save/clear flow
- the browser-preview mocks now match the real backend better for built-in versus user-saved watch sources
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `165` tests
- `npm run build` passed
- the base native desktop smoke passed again

Important remaining gap:

- the next watch step should be fuller watch management, not more low-level source-capability plumbing:
  - no watch list view yet
  - no bulk setup flow yet
  - no provider onboarding flow yet

## Current session note (March 15, 2026)

This session made the local Tauri dev loop more reliable.

Important changes and findings:

- `npm run tauri:dev` now runs through a small PowerShell wrapper
- that wrapper clears stale Vite listeners on port `1420` before launch
- the cleanup is cautious:
  - it only auto-stops stale `node`/Vite listeners
  - if some other app is using `1420`, it stops and tells the user instead of killing it
- `npm run dev:cleanup` now exists as a manual helper too
- the wrapper was proven against a real stale Vite listener and a real app start

Important remaining gap:

- the broader desktop smoke harness still needs its own cleanup and signoff polish, but the normal local `tauri:dev` loop is in much better shape now

## Current session note (March 15, 2026)

This session fixed the two startup regressions that were blocking `npm run tauri:dev`.

Important changes and findings:

- older databases now upgrade the watch-source table in the safe order:
  - add `anchor_file_id`
  - then create the `anchor_file_id` index
- a regression test now covers that older-database upgrade path
- tray creation is now lazy:
  - normal startup does not build the tray anymore
  - background mode asks for the tray only when it is actually needed
  - if Windows refuses the tray, SimSuite stays open instead of panicking during setup
- direct Rust startup now gets into normal app work again instead of dying during setup
- `tauri:dev` now gets past the old setup panic too

Important remaining gap:

- the desktop smoke and ad hoc dev checks can still leave Vite running on port `1420` when a run is interrupted, so the wrapper cleanup still needs a small follow-up

## Current session note (March 15, 2026)

This session added the first safe automatic watch-check loop and made the watch state easier to understand in the UI.

Important changes and findings:

- `Settings` now lets users:
  - turn automatic watch checks on or off
  - choose a watch-check interval
  - run `Check watched pages now`
- the Rust side now has a background watch poller for safe exact-page sources:
  - it only checks pages SimSuite already knows are safe to read directly
  - it updates `Home` and `Library` through one `watch-refresh-finished` workspace change event
  - it keeps CurseForge and other protected pages out of the automatic check path
- brand-new databases now create the watch tables correctly, so watch features no longer depend on older migration state
- `Library` now shows a clearer watch method:
  - `Check now supported`
  - `Reference only`
  - `Provider needed`
- CurseForge exact pages now land in the honest `provider needed` state instead of looking like a vague unsupported watch link

Important remaining gap:

- the desktop smoke wrapper timed out on startup in this session, so the wrapper itself still needs cleanup even though:
  - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `163` tests
  - `npm run build` passed
- the next product step should still be a fuller watch setup and management flow, plus a cleaner provider onboarding path

## Current session note (March 15, 2026)

This session finished the first real `Check now` watch path and tightened the native desktop proof so it follows the real app state instead of leaning on brittle screen guesses.

Important changes and findings:

- supported installed special mods now expose their built-in official watch page in `Library` even when there is no older saved family-state row yet
- `Library` can now do three watch actions on installed items:
  - save a watch source
  - clear a watch source
  - refresh a watch result when the source is one of the safe supported exact-page types
- the watch resolver now separates three cases more clearly:
  - `can check now`
  - `saved as a reference`
  - `provider required`
- safe check-now coverage is now real for:
  - MCCC official downloads page
  - XML Injector official page
  - GitHub releases pages such as Sims 4 Community Library
- creator pages still stay reminder-only
- CurseForge and similar protected pages still stay helper-only and cautious; they do not pretend to be auto-checkable without a proper provider
- the native Tauri smoke harness is now more truthful:
  - it starts the installed scan through the real backend command
  - it waits on the real scan state instead of using the Home `Last scan` label
  - it clicks actual Library rows instead of loose matching text
  - it now proves both `Check now` and generic save or clear watch flows in the real desktop app

Important remaining gap:

- the first installed-content watch flow is now real, but it is still small:
  - one item at a time
  - no broader watch setup surface yet
  - no watch-source editing history yet
  - no provider onboarding flow yet
- the next product step should be a fuller watch setup and management flow, not more low-level watch capability plumbing

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

The highest-value next step is now stabilization of the shared matching base before more feature growth:

1. audit whether family-hint candidate loading needs the same kind of careful widening that creator-hint loading just got
2. do more messy real-world validation on generic mod and CC matching, not just supported special mods
3. keep fixing watch bugs and watch setup edge cases until the current flow feels trustworthy
4. only after that, return to broader watch-management growth such as bulk setup, batch review, and watch history

After that hardening work is solid, the next product steps can go back to the fuller user-facing watch layer:

1. add a fuller watch-source flow for installed content now that save, clear, and check-now basics are proven
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
