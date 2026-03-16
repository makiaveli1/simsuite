# SimSuite Architecture

This repository currently runs as a Tauri desktop app with:

- `src/`: React + TypeScript desktop UI
- `src-tauri/src/`: Rust backend modules and Tauri commands
- `database/`: SQLite migrations and schema mirror
- `seed/`: starter creators, aliases, taxonomy, keyword dictionaries, and rule presets
- `seed/install_profiles.json`: built-in special mod catalog for guided installs, dependency rules, incompatibility warnings, and review-only download patterns
- `models/`: reserved for shared classification and prompt assets
- `user_data/`: reserved for user-learned data

## Current backend scope

Implemented modules:

- `scanner`
- `filename_parser`
- `bundle_detector`
- `content_versions`
- `duplicate_detector`
- `downloads_watcher`
- `install_profile_engine`
- `rule_engine`
- `validator`
- `library_index`
- `move_engine`
- `snapshot_manager`

Planned placeholders still present:

- `ai_classifier`

## Active safety pipeline

The current organization flow follows the product safety chain:

1. scanner
2. parser
3. rule engine
4. validator
5. preview
6. user approval
7. move engine

Snapshots are created before approved batch moves, and rollback is exposed through snapshot restore.

## Current UI scope

Implemented screens:

- Home
- Downloads
- Library
- Settings
- Creator Audit
- Category Audit
- Duplicates
- Organize
- Review

Not yet implemented:

- Patch Recovery
- Tools

## Current engineering note (March 16, 2026)

`Library` now has a player-facing summary layer on top of the stored local clue system, while the shared filename-classification layer has one more safe Build/Buy widening pass.

Important current behavior:

- the `Library` inspector now translates stored local clues into a simmer-friendly summary section:
  - plain-English file summary
  - file format
  - best version clue
  - useful role tags
  - in-game names
  - family hints
  - friendly version evidence
- this summary is derived from the same local-first stored evidence already used elsewhere:
  - filename classification
  - inside-file inspection
  - installed version evidence
  - watch state
- the technical inspection block still exists for deeper review, but its labels are now a little clearer
- the filename-classification layer now also recognizes a few more safe Build/Buy object terms:
  - `entryway`
  - `entrance`
  - `barback`
  - `fireplace`
- because that changes stored classification meaning again, the rebuild contract moved forward to:
  - scanner cache version -> `scanner-v15`
  - downloads assessment version -> `downloads-assessment-v9`

## Current engineering note (March 16, 2026)

The local package-classification layer now reads package context words correctly and uses them in a more careful fallback path.

Important current behavior:

- package context tokenization now splits real words before normalization, so path clues like `Strings`, `Thai`, `Integration`, and `LotPrices` are no longer mashed together into one unusable token
- support and translation packages can now still classify when they include a small helper-resource mix alongside `StringTable`, instead of only classifying the pure-string-table case
- a narrow gameplay-context fallback can now promote packages to `Gameplay` when:
  - the inside-file resource signals already lean gameplay
  - and the surrounding path or filename context clearly says add-on, module, integration, or dense lot-price style content
- this strengthened local package logic feeds the same stored-evidence layer used by:
  - `Library` installed facts
  - Inbox compare confidence
  - watch setup suggestions and watch review
- because this changed stored evidence meaning again, the rebuild contract moved forward to:
  - scanner cache version -> `scanner-v14`
  - downloads assessment version -> `downloads-assessment-v8`

## Current engineering note (March 16, 2026)

The scanner now reconciles creator signals more carefully, so harmless creator-noise does not turn into fake review work.

Important current behavior:

- nearby path hints only count when they resolve to a known creator profile
- a known folder creator can replace a weak unknown filename fallback instead of creating a fake conflict
- unknown folder names are ignored instead of being treated like creator truth
- inspection hints now compare against the full creator-hint list, not only the first hint
- a real `conflicting_creator_signals` warning is now only raised when two known creators genuinely disagree
- this behavior sits in the shared stored-evidence layer used by:
  - `Library` installed facts
  - Inbox compare confidence
  - watch setup and watch review surfaces
- because this changed stored evidence meaning, the rebuild contract was bumped again:
  - scanner cache version -> `scanner-v12`
  - downloads assessment version -> `downloads-assessment-v6`

## Current engineering note (March 15, 2026)

The local package-classification layer is now stronger and more honest after a deep live-library audit.

Important current behavior:

- many `.package` files were already being parsed correctly as `dbpf-package`, but the earlier category inference was too thin
- inside-file package inspection now recognizes more safe Sims resource clusters for:
  - Build/Buy surfaces
  - Build/Buy structures
  - stronger gameplay tuning groups
- package inspection can now return a confidence floor along with the inferred kind and subtype
- the scanner uses that inspection confidence floor when it applies inspection hints
- the scanner also clears stale category warnings when inspection truth confirms the final kind, so older `no_category_detected` and `conflicting_category_signals` warnings do not linger after a rebuild
- filename keyword coverage is also wider for common Build/Buy and gameplay terms, so filename parsing and inside-file inspection now reinforce each other more often instead of leaving obvious files in low-confidence buckets
- this strengthened local classification matters to more than one surface because it feeds the same stored file-evidence layer used by:
  - `Library` installed facts
  - Inbox compare confidence
  - watch setup suggestions

## Current engineering note (March 15, 2026)

The stale local-clue rebuild path now works on both installed-library data and Inbox download data.

Important current behavior:

- installed-library truth now depends on the scanner cache version as part of the stored-inspection contract:
  - when stored inspection meaning changes, the scanner cache version is bumped
  - unchanged installed files then get one real rebuild on the next scan instead of silently reusing stale rows
- Inbox truth now does the same thing through the downloads assessment version:
  - unchanged download sources no longer only rerun cached assessment logic on a version bump
  - they are reprocessed through the real ingest path so stored `files` rows refresh too
- this matters because both major trust surfaces depend on the same stored file-evidence layer:
  - `Library` installed facts and watch setup
  - `Inbox` compare and intake confidence
- the result is a more honest local-first pipeline:
  - parser changes can now propagate into stored truth
  - higher-level compare decisions are less likely to sit on stale evidence

## Current engineering note (March 15, 2026)

The file-inspection layer now filters a real ts4script noise path that showed up in the live app database.

Important current behavior:

- flat ts4script archives no longer treat raw file names such as `.pyc` entries or `_DO_NOT_UNZIP_` markers as if they were clean script namespaces
- that noise is now kept out of:
  - `script_namespaces`
  - `embedded_names`
  - very short fallback creator hints
- nested script archives still keep their real folder-based namespace clues
- this keeps the script clue layer closer to what the archive is actually telling us instead of recycling packaging noise as identity

## Current engineering note (March 15, 2026)

The file-inspection layer now pulls a little more safe creator truth out of ts4script manifests.

Important current behavior:

- ts4script manifest parsing now reads:
  - name-style fields
  - author / creator fields
  - simple string lists for authors and creators
- those values feed the same local clue pipeline as the older script hints:
  - embedded names
  - creator hints
  - family hints
- this is still additive only:
  - script mods without manifests still rely on namespaces, filenames, family hints, creator hints, and other local clues
  - manifest data is not treated as required truth

## Current engineering note (March 15, 2026)

The shared generic compare layer now uses stronger family clues during installed-candidate gathering too.

Important current behavior:

- full compare candidate gathering can now widen the installed search using inspected `family_hints`
- that widening stays cautious:
  - very short family labels are skipped
  - only the stronger normalized family clues are used to widen the installed candidate pool
- this means the generic compare layer now gathers candidates from:
  - exact hashes
  - exact filenames
  - creator assignments and inspected `creator_hints`
  - stronger inspected `family_hints`
  - filename tokens
- scoring still happens afterward, so widening the candidate pool does not bypass the later confidence checks

## Current engineering note (March 15, 2026)

The shared generic compare layer is now stricter about when it will claim that a download is simply missing from the installed library.

Important current behavior:

- generic compare now requires a medium-strength incoming local identity before it can return `not installed`
- creator plus version alone is now treated as too thin and stays `unknown`
- trusted version clues only contribute to that incoming identity when the version clue itself is at least medium-confidence
- full compare can now search installed rows using inspected `creator_hints`, not just creator assignments that were already saved into the database
- this keeps the generic compare layer cautious while still letting stronger local creator evidence help it find the right installed subject

## Current engineering note (March 15, 2026)

The shared matching and watch-setup foundation is now slightly stricter and less guessy.

Important current behavior:

- `Library` watch setup no longer shortlists candidates by looking for raw JSON field names inside stored `insights`
- the shortlist now parses the stored `FileInsights` payload and checks for real populated clue values before a row can enter the watch-setup scan
- exact-page setup suggestions now require stronger local clue combinations instead of letting weak version-only hints through
- shared subject matching now uses both:
  - saved creator matches from the database
  - inspected `creator_hints` found inside files
- ts4script manifest names can now add optional identity hints when they exist
  - this is additive only
  - script mods without manifests still rely on filenames, namespaces, family hints, creator hints, and other local clues

## Current engineering note (March 15, 2026)

The watch system now has a fuller compact flow inside `Library` without branching into another management screen.

Important current watch-center behavior:

- the watch center now has three compact follow-up lanes inside the same `Library` table area:
  - tracked watched items
  - setup suggestions for unwatched installed items
  - review queue items for saved reminder-only or provider-needed links
- strongest exact-page setup suggestions are split into their own small bulk strip above the normal setup list so the easiest exact-page wins stay visible together
- `Home` now gets both setup-count and review-count truth from the same backend watch data, so the summary layer and `Library` stay aligned
- pending watch setup or review intent is now applied only after the target file detail opens:
  - this avoids losing the handoff when the `Library` inspector starts empty
  - it keeps the existing detail panel as the single editor for both setup and review

## Current engineering note (March 15, 2026)

The watch system now has a cleaner cross-screen handoff and a more complete follow-up loop without adding another management surface.

Important current watch-center behavior:

- `Home` watch summary rows can now open `Library` with a focused watch intent instead of only navigating to the generic Library screen
- `Library` can respond to that focused intent by:
  - switching to the right tracked-watch filter when needed
  - highlighting and scrolling the right section into view
  - showing a plain message about what lane the user is seeing
- the watch center now has small “start from here” actions inside the same surface:
  - setup flow can start from the strongest setup suggestion
  - review flow can start from the first tracked item that still needs human follow-up
- review flow now reuses the same queue-like follow-up pattern as setup:
  - save / clear / refresh can move forward to the next review item when the current one is resolved
  - review mode also has a skip action
- this keeps the watch system inside the current `Library` table area and detail panel:
  - no second watch-management screen
  - no separate wizard surface
  - no guessed URLs

## Current engineering note (March 15, 2026)

The Library watch center now has a more complete follow-up loop without turning into a second management screen.

Important current follow-up behavior:

- tracked watch rows can now expose a direct review action for saved user-managed watch links that still need human follow-up
- the same existing Library detail panel now serves two follow-up modes:
  - setup mode for unwatched installed items from the shortlist
  - review mode for saved generic watch links from the tracked list
- setup mode can continue to the next strong suggestion after a save, so the user does not have to keep bouncing back into the shortlist between each item
- setup mode also lets the user:
  - skip one suggestion for now
  - stop the current setup queue cleanly
- this keeps watch management flat and compact:
  - no second watch-management screen
  - no guessed URLs
  - no separate wizard state outside the current Library surface

## Current engineering note (March 15, 2026)

Library now follows the same command-threading rule that stabilized Inbox:

- the heavier Library-facing commands now run in background workers instead of on the window thread
- this includes:
  - Home overview loading when Library asks for watch counts
  - Library facets
  - Library list rows
  - Library tracked-watch rows
  - selected-item detail
  - save and clear watch actions
  - creator-learning and category-override saves
- this change is about responsiveness, not changing Library truth:
  - the backend work still happens
  - it just stops blocking the whole desktop window while it runs

## Current engineering note (March 15, 2026)

The Library watch center now has two compact lanes instead of one:

- a tracked-items lane for pages that are already being watched
- a setup-suggestions lane for installed items that look watch-worthy but are not set up yet

Important current setup-lane behavior:

- the setup lane stays inside the existing Library table panel
- it only uses local installed-file clues to shortlist candidates
- it skips anything that already has:
  - a saved watch page
  - a built-in supported special-mod watch page
- it can suggest:
  - exact page
  - creator page
- suggestion rows can now do two safe things:
  - open the existing Library inspector
  - open the existing watch editor already prefilled to the suggested source type and label
- `Home` now gets a matching `watch_setup_items` count from the same backend truth, so the wider app can point users to unfinished watch setup without separate frontend counting logic
- `Home` watch summary rows now jump straight to `Library`, so the summary layer can hand users off to the real watch-management surface without adding another screen
- the backend setup scan now normalizes stored file extensions before filtering, so older rows with `.package` / `.ts4script` do not get dropped from the shortlist

## Current engineering note (March 15, 2026)

Startup is now safer in two important ways:

- database upgrades add the `content_watch_sources.anchor_file_id` column before they create the matching index, so older databases no longer crash during setup
- tray creation is now lazy:
  - normal startup skips tray setup
  - background mode creates the tray only when it actually needs it
  - if Windows refuses the tray icon, the app stays open instead of panicking during launch

## Current engineering note (March 15, 2026)

The Library watch center is no longer summary-only. It now doubles as the first real watch-management surface for tracked items.

Important current watch-center behavior:

- the watch center still stays inside the existing Library table panel instead of opening a new management screen
- it now has filter chips for:
  - needs attention
  - confirmed updates
  - possible updates
  - unclear
  - all tracked
- tracked rows can come from:
  - user-saved watch pages
  - built-in official pages for supported special mods
- built-in supported special mods do not need an older helper latest row before they can appear in that tracked list
- clicking a tracked row reuses the existing Library inspector instead of branching into a second watch-details surface
- this keeps watch management flat and compact while still making the summary counts actionable

## Current engineering note (March 15, 2026)

Library watch sources now have a clearer product split so the app stops blurring built-in special-mod pages with user-saved pages.

Important current watch-source behavior:

- `WatchResult` now records where the source came from:
  - built-in official page for a supported special mod
  - user-saved page
  - no saved page yet
- supported special mods now show their built-in official page in `Library` without pretending the user saved it by hand
- custom watch-page saves for supported special mods are intentionally blocked for now because there is no honest merge rule yet between:
  - the built-in official page
  - a user override page
- `Library` now has a compact watch center inside the existing table area that shows:
  - confirmed update count
  - possible update count
  - unclear watched-item count
  - automatic-check state
  - last run state
  - one manual `Check watched pages now` action
- that watch center stays summary-first so the screen does not turn into a second dashboard or a crowded settings page

## Current engineering note (March 15, 2026)

The watch system now has a safe automatic polling layer on top of the earlier shared version-and-watch foundation.

Important current watch behavior:

- automatic watch checks are controlled from `Settings`
- the background poller only runs for:
  - safe exact pages SimSuite already knows how to read directly
  - future approved providers
- protected pages are still stored, but they stay outside the automatic polling path
- the watch result now carries a clearer product state:
  - `can check now`
  - `saved as a reference`
  - `provider required`
- `Library` now shows that watch method directly in the detail panel so the user can tell whether:
  - SimSuite can check the page now
  - the link is only a reminder
  - a provider such as CurseForge still needs an approved integration path
- `Home` and `Library` stay in sync through a `watch-refresh-finished` workspace change event after watch polling completes
- the watch schema now exists in both fresh and migrated databases, so new installs and test databases behave the same way

## Current engineering note (March 13, 2026)

The current app is no longer just a scan-and-sort shell. It already has a real Downloads Inbox, a real guided special-mod pipeline, real snapshot-backed apply flows, and one shared version-and-watch foundation for all content.

Important current behavior:

- `Downloads` is the intake desk for new files, archives, guided special-mod installs, blocked items, and review-only batches
- `Organize` is the safe sorter for broader library cleanup and preset-based moves
- `Library`, `Creator Audit`, and `Category Audit` feed learned data back into later scans and suggestions
- `Review` is the hold queue for files that still need a human decision
- `Duplicates` is inspection-only right now
- `Home` can now summarize broader update-watch status without turning into another busy dashboard

Important current version-and-watch architecture:

- `file_inspector` now collects structured local `versionSignals` for all content
- `FileInsights.versionHints` stays as the short compatibility summary, but it is no longer the whole version story
- `content_versions` builds one local subject for a download or installed item and then:
  - scores the best installed match
  - compares versions
  - returns a separate confidence result
  - stays cautious when the match is weak or the local clues disagree
- generic downloads can now compare against installed content when the local match is strong enough
- weak generic matches stay `unknown` instead of pretending to know
- `Library` now focuses on installed content only:
  - Downloads rows stay in `Inbox`
  - Library queries exclude `source_location = 'downloads'`
  - watch actions only attach to installed Library items
- `Library` now shows installed-version facts and watch status in the detail panel only
- `Home` now rolls up:
  - exact updates
  - possible updates
  - unknown watch status
- watch data is stored separately from local compare truth:
  - local compare still decides Inbox version truth
  - watch results stay helper-only
- supported installed special mods can now expose a built-in exact-page watch source in `Library` even before the user saves anything manually
- watch capabilities are now split into three product states:
  - can check now
  - saved as a reference
  - provider required
- the first live `Check now` path currently covers:
  - MCCC official downloads page
  - XML Injector official page
  - safe GitHub releases pages such as Sims 4 Community Library
- creator pages are still reminder-only
- guided install is still special-mod-only
- community lists and third-party indexes are still reference material only, not runtime truth
- future provider integrations such as CurseForge should only use official approved APIs and user-approved sources; scraping protected pages is out of scope

Important current special-mod architecture:

- the app has a built-in special-mod catalog in `seed/install_profiles.json`
- special mods are handled with per-mod rules, not one shared MCCC rule
- special-mod profiles can now declare `versionStrategy` rules in seed data
- the repeatable parts of special-mod handling should stay shared:
  - common compare logic
  - common evidence building
  - common smoke-test helpers
  - keep per-mod differences in seed data or small strategy hooks so the catalog can grow without copy-paste logic
- update decisions are local-first:
  - compare the downloaded pack
  - compare the installed files on disk
  - use saved family state and file fingerprints only when needed
  - use official latest checks only as extra guidance
- selected-item special-mod actions should trust the fuller loaded special-mod decision and guided plan over stale queue-row intake flags, so same-version safe reinstalls do not disappear just because the queue row was older
- if the installed side is missing a saved package hash, the compare flow can fall back to hashing the real installed file from disk so same-version support libraries do not false-flag as unknown
- internal file inspection now helps special-mod identity and version checks
- community sources and third-party indexes are not part of runtime install authority
- helper-only official latest checks should only use safe readable official endpoints; challenge bypasses and brittle scraping workarounds are out of scope

Current integration and performance work already in place:

- workspace change events now refresh only the parts of the app that actually changed
- Downloads queue and selected-item loading were split so Inbox can stay lighter
- Downloads queue rows now stay on lightweight lane and summary data, while full special-mod compare work stays on the selected item panel
- the app now lazy-loads major screens
- hot-path indexes and slow-command timing logs were added
- Inbox startup now boots from a real Downloads watcher state instead of guessing

Current biggest known gap:

- Inbox is much healthier in real desktop use now, but richer selected-item special-mod detail can still feel heavier than the rest of the screen
- broader watch-source setup and management still needs careful product work
- the remaining work should focus on:
  - trimming that selected-item detail path
  - widening safe helper-only latest sources
  - extending deeper apply or blocked-flow validation before growing the catalog further

## Known gaps versus the full product plan

- scanner is incremental and backgrounded now, but deeper prioritization and scheduling controls are still missing
- duplicate detection covers exact, filename, and version cases, but safe duplicate cleanup actions are still missing
- tray bundles are grouped; broader mod-set bundle detection is not implemented
- the special mod catalog covers the first curated wave only; broader curated profile, dependency, and incompatibility coverage is still pending
- the shared version foundation is in place for all content, but broader user-facing watch management and later Library watch filters are still pending
- option-pack and manual-step archives are detected and routed to review, but SimSuite does not yet guide users through choosing install variants
- AI classification is not wired to Ollama or llama.cpp yet
- patch recovery tooling is still pending
- user-facing Tools and Tray surfaces are still missing
