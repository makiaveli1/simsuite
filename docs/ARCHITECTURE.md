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

- Tray
- Patch Recovery
- Tools

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
- `Library` now shows installed-version facts and watch status in the detail panel only
- `Home` now rolls up:
  - exact updates
  - possible updates
  - unknown watch status
- watch data is stored separately from local compare truth:
  - local compare still decides Inbox version truth
  - watch results stay helper-only
- guided install is still special-mod-only
- community lists and third-party indexes are still reference material only, not runtime truth

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
