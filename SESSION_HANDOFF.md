# Session Handoff

## Current Priority

- March 15, 2026: the first installed-content watch flow now works end to end in the real Tauri app, including `Check now` for safe supported pages. The next product focus should be a fuller watch setup and management flow, not more backend guesswork.
- March 14, 2026: Library watch flow is now cleaner in the real app because Library queries now focus on installed content only. The next product focus should be the first fuller user-facing watch setup flow, not mixing Downloads rows into Library.
- March 14, 2026: Lumpinou Toolbox same-version handling is now confirmed in the real desktop app. The next product focus can move back to broader watch flow and careful special-mod growth.
- March 13, 2026: the shared version and update-watch foundation is now in place for all content, so the next product focus is making the watch flow more complete without slowing Inbox back down.
- March 13, 2026: keep guided install special-mod-only. Generic mods and CC can now compare against installed content, but weak matches must still stay cautious and say `unknown`.
- March 13, 2026: use `docs/SPECIAL_MOD_ONBOARDING.md` and `docs/SPECIAL_MOD_CANDIDATES.json` for future catalog growth. The external Sims mod index stays frozen and reference-only.
- March 13, 2026: keep the queue light and leave the heavier compare and evidence work on the selected item, because that is what brought live Inbox first-open back down to about `1.07s`.

## What Changed This Session

- March 15, 2026: finished the first real `Check now` watch path for installed Library items:
  - supported installed special mods now expose their built-in official page in Library even if there is no older saved family-state row yet
  - Library detail now shows whether a saved or built-in watch source can be checked right away
  - Library detail can now refresh a supported watch result with a real backend command instead of only saving or clearing the source
- March 15, 2026: improved watch-source capability handling:
  - safe supported pages such as MCCC, XML Injector, and GitHub releases now show `Check now`
  - creator pages still stay reminder-only
  - protected or blocked pages such as CurseForge and Lot 51 still stay cautious and do not pretend they are auto-checkable
- March 15, 2026: fixed the native desktop smoke harness so it follows the real app state better:
  - it now starts the installed scan through the real backend command instead of guessing from Home labels
  - it now waits on the real scan state
  - it now clicks actual Library rows instead of loose matching page text
  - it now handles webdriver click interception more safely during overlay transitions
- March 14, 2026: fixed the first real Library watch-flow gap:
  - `Library` now excludes Downloads rows and stays focused on installed content
  - saving or clearing a watch source now only works for installed Library items
  - the browser-preview mocks now match that same installed-only rule
  - the native desktop smoke now triggers a real installed scan before it tests Library watch actions
- March 14, 2026: added a generic installed fixture file to the native desktop smoke so the watch-source save and clear path is proven against a real Tauri app.
- March 14, 2026: researched CurseForge update-monitoring options from official sources:
  - CurseForge does have an official 3rd-party API path
  - it requires applying for an API key
  - project owners can block 3rd-party distribution per project
  - SimSuite should only consider a CurseForge integration through that approved API path, never through scraping or challenge bypasses
- March 14, 2026: fixed two app build blockers that were preventing a fresh native desktop verification pass:
  - `src/lib/api.ts` now imports `WatchSourceKind`
  - `src/screens/LibraryScreen.tsx` no longer shadows the watch-source label helper with a local state name
- March 14, 2026: removed the temporary live-database debug test after the live Lumpinou check was confirmed, so the repo keeps only normal regression coverage.
- March 13, 2026: added one shared version-and-match layer for all content instead of keeping version comparison mostly special-mod-only.
- March 13, 2026: `file_inspector` now stores structured `versionSignals` in file insights, while keeping `versionHints` as the short compatibility list.
- March 13, 2026: added a shared content-version resolver that:
  - builds one local subject for the download
  - finds the best installed match
  - scores how believable that match is
  - compares versions with a separate confidence result
  - stays cautious when the match is weak or the local clues disagree
- March 13, 2026: Inbox queue items can now carry generic `versionResolution` data for non-special content, while supported special mods still keep their stricter guided logic on top.
- March 13, 2026: Library detail now shows:
  - installed version summary
  - local evidence summary
  - watch status
- March 13, 2026: Home now shows broader update-watch counts without adding more dashboard boxes:
  - exact updates
  - possible updates
  - watch unknown
- March 13, 2026: the special-mod rule layer is now truly profile-driven:
  - `versionStrategy` is now read correctly from `seed/install_profiles.json`
  - the current built-in supported mods now use those rules
  - old `versionHints` can still help older indexed data through a legacy bridge while the new signal model takes over
- March 13, 2026: added long-term growth docs:
  - `docs/SPECIAL_MOD_ONBOARDING.md`
  - `docs/SPECIAL_MOD_CANDIDATES.json`
- March 13, 2026: fixed same-release handling for Lumpinou Toolbox so a same-version download with different file fingerprints is treated as a safe reinstall instead of “version unclear”, while MCCC stays strict.
- March 13, 2026: added the first user-facing watch-source flow for installed content:
  - Library detail can now save or clear per-subject watch sources (exact mod page or creator page).
  - watch sources are stored in `content_watch_sources` with user approval.
  - the watch resolver distinguishes between “no source saved” and “source saved but not yet checked”.

## What Was Tested

- March 15, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- March 15, 2026: `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- March 15, 2026: `cargo build --manifest-path src-tauri/Cargo.toml` passed.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `158` tests.
- March 15, 2026: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features` passed with warnings only.
- March 15, 2026: `npm run build` passed.
- March 15, 2026: real native desktop fixture smoke passed after the watch-capability and harness fixes:
  - the app launched in Tauri
  - the installed scan was triggered through the real backend command
  - Library opened on real installed content
  - a supported installed Library item showed `Check now`
  - the watch result refreshed in the real app
  - the generic installed fixture file could still save and clear a watch source
- March 14, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- March 14, 2026: `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- March 14, 2026: `cargo build --manifest-path src-tauri/Cargo.toml` passed.
- March 14, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `153` tests.
- March 14, 2026: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features` passed with warnings only.
- March 14, 2026: `npm run build` passed.
- March 14, 2026: real native desktop fixture smoke passed after the Library watch-flow fix:
  - the app launched in Tauri
  - a real installed scan was triggered
  - Library opened on installed content
  - the generic installed fixture file could save a watch source
  - the same file could clear that watch source again
- March 14, 2026: `npm run build` passed after the small app fixes.
- March 14, 2026: `npm run tauri:build -- --debug` passed.
- March 14, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `146` tests.
- March 14, 2026: real native desktop read-only check against the live app data passed for Lumpinou Toolbox:
  - Inbox queue row showed `Installed and incoming match`
  - selected detail showed `Already current`
  - the primary action showed `Reinstall guided copy`
  - local versions showed `Installed 1.179.6` and `Incoming 1.179.6`
- March 13, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `142` tests.
- March 13, 2026: `cargo build --manifest-path src-tauri/Cargo.toml` passed.
- March 13, 2026: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features` passed with warnings only.
- March 13, 2026: `npm run build` passed.
- March 13, 2026: `npm run desktop:smoke:fixtures` passed.

## What Worked

- Built-in supported Library items no longer need an older saved family-state row before SimSuite knows their official page.
- The first real `Check now` path works in the desktop app for safe supported pages.
- The native desktop smoke is more honest now because it waits on the real scan state and clicks actual Library rows.
- The real Library watch flow now works on the right kind of content:
  - installed files only
  - not Inbox or Downloads rows
- The native desktop smoke now proves the save-watch and clear-watch path in the real app instead of only proving that the UI renders.
- The live desktop app now confirms the Lumpinou Toolbox fix in the real Inbox, not just in backend tests.
- The real queue row and the selected item panel agree for the current live Lumpinou same-version case.
- Generic downloads now have a shared local compare path instead of special-case-only version checks.
- Supported special mods still keep:
  - guided install
  - reinstall rules
  - dependency checks
  - blocked-flow rules
  - rollback-backed apply
- Library now has local installed-version awareness without turning into another Inbox.
- Home can now summarize broader update-watch state without crowding the UI.
- The current six built-in supported special mods still work on the stricter profile-driven rule path:
  - MCCC
  - XML Injector
  - Lot 51 Core Library
  - Sims 4 Community Library
  - Lumpinou Toolbox
  - Smart Core Script
- The real fixture-backed desktop smoke still passes after the shared foundation work.

## Known Problems / Gaps

- The watch system is readable now, but the user-facing management flow is still thin:
  - watch results can be shown
  - generic watch sources are stored in the database
  - Library can now save, clear, and sometimes check a watch source for installed items
  - but broader setup, editing, batch setup, provider setup, and polling flows still need to grow carefully
- Helper-only official latest support is still intentionally narrow:
  - MCCC, GitHub release pages, and XML Injector are supported
  - Lot 51 and the CurseForge-backed sources still stay `unknown` because plain app requests hit challenge pages
- CurseForge is promising as a future provider, but it is not a drop-in shortcut:
  - it requires an approved API key
  - project distribution can be turned off by the author
  - the official terms place real limits on how 3rd-party apps can use and cache API data
- Heavy selected special-item detail is still slower than the queue in real desktop use.
- The first curated expansion wave has not started yet.
- `cargo clippy` still reports some older warnings that were not cleaned up in this checkpoint.
- The raw native check is still best as a read-only spot check unless we are deliberately running fixture-backed apply flows.

## Important Decisions

- Local installed-vs-downloaded truth stays first.
- Official latest stays helper-only.
- Weak content matches must stay cautious and return `unknown`.
- Guided install stays special-mod-only.
- Library watch actions should only attach to installed Library items, not Downloads rows.
- Any CurseForge integration must use the official approved API path. No scraping, no challenge bypasses, and no trying to sneak around author distribution settings.
- The external Sims mod index stays frozen and reference-only.
- Future growth should be data-driven:
  - shared version signals
  - shared subject matching
  - shared compare logic
  - shared onboarding docs
  - per-mod rules in profile data

## Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Then use `docs/SPECIAL_MOD_ONBOARDING.md` before adding any new supported special mod.
- Next best product steps:
  - design the first fuller user-facing watch-source flow for installed content now that save, clear, and check-now basics are real
  - decide whether SimSuite should add provider adapters after that, starting with a CurseForge feasibility check against their API terms and key requirements
  - widen helper-only latest parsing only where there is a safe official endpoint
  - add the first small curated expansion wave through `docs/SPECIAL_MOD_CANDIDATES.json`
  - keep checking Inbox detail performance so the broader compare system does not make the screen feel heavy again
