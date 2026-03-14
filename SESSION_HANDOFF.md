# Session Handoff

## Current Priority

- March 14, 2026: Lumpinou Toolbox same-version handling is now confirmed in the real desktop app. The next product focus can move back to broader watch flow and careful special-mod growth.
- March 13, 2026: the shared version and update-watch foundation is now in place for all content, so the next product focus is making the watch flow more complete without slowing Inbox back down.
- March 13, 2026: keep guided install special-mod-only. Generic mods and CC can now compare against installed content, but weak matches must still stay cautious and say `unknown`.
- March 13, 2026: use `docs/SPECIAL_MOD_ONBOARDING.md` and `docs/SPECIAL_MOD_CANDIDATES.json` for future catalog growth. The external Sims mod index stays frozen and reference-only.
- March 13, 2026: keep the queue light and leave the heavier compare and evidence work on the selected item, because that is what brought live Inbox first-open back down to about `1.07s`.

## What Changed This Session

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
  - but broader setup and editing flows still need to grow carefully
- Helper-only official latest support is still intentionally narrow:
  - MCCC, GitHub release pages, and XML Injector are supported
  - Lot 51 and the CurseForge-backed sources still stay `unknown` because plain app requests hit challenge pages
- Heavy selected special-item detail is still slower than the queue in real desktop use.
- The first curated expansion wave has not started yet.
- `cargo clippy` still reports some older warnings that were not cleaned up in this checkpoint.
- The raw native check is still best as a read-only spot check unless we are deliberately running fixture-backed apply flows.

## Important Decisions

- Local installed-vs-downloaded truth stays first.
- Official latest stays helper-only.
- Weak content matches must stay cautious and return `unknown`.
- Guided install stays special-mod-only.
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
  - add the first careful user-facing watch-source flow
  - widen helper-only latest parsing only where there is a safe official endpoint
  - add the first small curated expansion wave through `docs/SPECIAL_MOD_CANDIDATES.json`
  - keep checking Inbox detail performance so the broader compare system does not make the screen feel heavy again
