# Session Handoff

## Current Priority

- March 12, 2026: verify the Inbox freeze fix against the user's real Downloads folder, not only the isolated desktop smoke fixtures.
- March 12, 2026: widen Inbox special-mod validation beyond MCCC now that the native desktop smoke lane is steady again.
- March 12, 2026: expand helper-only official latest parsing for the supported mods that still show `unknown`, while keeping local compare as the real authority.
- The highest-priority remaining gap is broader supported-mod coverage and helper-only official latest parsing for supported sources that still return `unknown`.

## What Changed This Session

- March 12, 2026: made the repo memory process official in `AGENTS.md`.
- March 12, 2026: added this rolling handoff file as the first thing to read next session.
- March 12, 2026: updated `docs/IMPLEMENTATION_STATUS.md` to record the latest Inbox findings, the real desktop test results, and the current known gaps.
- March 12, 2026: recent landed Inbox work now includes safer special-mod review links, safer archive handling for `.7z` and `.rar`, narrower watcher refreshes, lighter special-mod queue summaries, and an isolated real Tauri desktop smoke lane.
- March 12, 2026: fixed the special-mod family chooser so already-applied fuller packs stay in family comparison instead of disappearing.
- March 12, 2026: leftover partial family items now settle into a covered-already state instead of telling the user to open the wrong Inbox sibling.
- March 12, 2026: added a backend regression test for the exact post-apply MCCC family bug.
- March 12, 2026: fixed a watcher startup bug where the initial Inbox pass could fail silently and leave the watcher stuck in `processing`.
- March 12, 2026: fixed archive staging collisions so two fresh downloads in the same second do not share one staging folder.
- March 12, 2026: taught the post-apply family anchor to treat the applied full pack as the installed match instead of re-reading it as an incomplete fresh download.
- March 12, 2026: cleared stale leftover special-mod actions when a fuller family pack is already installed.
- March 12, 2026: aligned blocked leftover Inbox wording with the new covered-already backend result.
- March 12, 2026: stabilized the native Tauri smoke helper:
  - it now reads body text more safely
  - it tolerates short refresh gaps after apply
  - apply mode now skips unnecessary pre-apply detours
  - the smoke wrapper now runs `tauri build -- --debug` by default so it launches the real desktop app surface instead of a half-built Rust binary
- March 12, 2026: fixed a false same-version miss for `.ts4script` special mods by hashing the real contents inside the script archive instead of trusting the outer zip bytes.
- March 12, 2026: upgraded the native desktop smoke lane so it clicks the real Inbox row buttons by item name instead of any matching text on screen.
- March 12, 2026: extended the fixture-backed real desktop lane to prove XML Injector same-version and older-version handling alongside the MCCC checks.
- March 12, 2026: corrected the apply smoke assertion so it only flags the wrong MCCC sibling wording, not a separate XML Inbox row that can legitimately mention a fuller sibling.
- March 12, 2026: added Sims 4 Community Library to the backend version-comparison tests and the real Tauri desktop fixture lane.
- March 12, 2026: confirmed that direct app-style requests to CurseForge and Lot 51 still hit Cloudflare challenge pages, so helper-only latest expansion for those sources needs a safe official endpoint, not a bypass.
- March 12, 2026: trimmed repeated Inbox reload work in `DownloadsScreen.tsx` so one local action no longer stacks several queue reload paths on top of each other.
- March 12, 2026: replaced the old one-count skip with a short grace window after local Inbox reloads, so command events and watcher follow-up events do not immediately force another heavy Downloads reload.
- March 12, 2026: changed post-action Inbox updates so apply, ignore, and special review actions now use one main queue reload and refresh watcher status in the background instead of blocking on another extra call first.
- March 12, 2026: tightened selected-item reloads so the right panel now reloads when the selected item id or `updatedAt` changes, not every time the whole queue object is rebuilt.

## What Was Tested

- March 12, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- March 12, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- March 12, 2026: `npm run build` passed.
- March 12, 2026: `npm run tauri build -- --debug` passed.
- March 12, 2026: real Tauri desktop smoke passed with fixture folders through `npm run desktop:driver:fixtures` and `npm run desktop:smoke`.
- March 12, 2026: real desktop MCCC apply was checked in the fixture app. The update completed, preserved the `.cfg` file, and refreshed Inbox, but the post-apply family guidance was wrong.
- March 12, 2026: backend regression test `applied_full_family_pack_keeps_leftover_partial_out_of_the_waiting_lane` passed.
- March 12, 2026: real Tauri desktop base Inbox smoke passes through `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`.
- March 12, 2026: real Tauri desktop apply smoke passes through `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1 -IncludeApply`.
- March 12, 2026: full Rust suite still passes with `cargo test --manifest-path src-tauri/Cargo.toml`.
- March 12, 2026: focused XML Injector backend tests passed after the signature fix.
- March 12, 2026: real Tauri desktop base smoke now proves:
  - MCCC update item loads
  - XML Injector same-version item loads
  - XML Injector older-version item loads
  - version evidence and compare text appear in the real app
- March 12, 2026: real Tauri desktop apply smoke still passes after the XML fixture and smoke-harness upgrades.
- March 12, 2026: focused Sims 4 Community Library backend tests passed.
- March 12, 2026: real Tauri desktop base smoke now also proves:
  - Sims 4 Community Library same-version item loads
  - Sims 4 Community Library older-version item loads
  - version evidence and compare text appear in the real app
- March 12, 2026: after the Inbox reload cleanup, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run tauri:build -- --debug`, and `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1 -IncludeApply` all passed again.

## What Worked

- Real desktop Inbox first-open, refresh, and selection worked in the fixture app.
- Real desktop MCCC version evidence was clear and correct before apply:
  - installed `2025.9.0`
  - incoming `2026.1.1`
  - compare result `Incoming pack is newer`
- Real desktop safe MCCC apply worked in the fixture app:
  - restore-point confirmation appeared
  - 3 files were moved in
  - 3 older files were replaced
  - 1 settings file was kept
- Real desktop post-apply Inbox state now works better:
  - the full MCCC pack settles into the done lane instead of falling back into an incomplete waiting state
  - the leftover partial sibling settles into a covered-already done state
  - the leftover no longer offers a misleading “download missing files” action once the fuller family pack is already installed
- Backend family truth is now better:
  - a fuller applied special-mod pack can still anchor the family after install
  - a weaker leftover sibling no longer gets the wrong “open that item first” action in the backend decision path
- Local installed-vs-downloaded comparison is still the trustworthy decision path.
- Official latest stays helper-only and does not block the local decision flow.
- Real desktop XML Injector same-version now behaves correctly in the fixture app:
  - installed `4.0`
  - incoming `4.0`
  - compare result `Installed and incoming match`
  - the decision no longer falls back to `Version could not be compared` just because the two `.ts4script` zip wrappers differ
- Real desktop Sims 4 Community Library now behaves correctly in the fixture app:
  - same-version downloads settle into the already-current path
  - older downloads stay out of the update path
  - the version panel shows local compare clues in the real app
- The real desktop smoke still passes after the Inbox refresh cleanup:
  - Inbox open still works
  - refresh still works
  - MCCC apply still works
  - the lighter reload path did not break the real desktop special-mod flow

## Known Problems / Gaps

- The freeze fix is proven against the real desktop fixture app, but it still needs a real live-folder check against the user's own heavier Downloads state.
- Helper-only official latest coverage is still too narrow:
  - MCCC and GitHub release pages are supported
  - Lot 51 and several CurseForge-backed supported mods still show `unknown` even though their official pages are readable today
- Direct non-browser requests to CurseForge and Lot 51 still hit Cloudflare challenge pages, so helper-only latest expansion for those sources is blocked unless we find a safe official machine-readable path.
- Real desktop coverage now includes MCCC, XML Injector, and Sims 4 Community Library, but the other supported special-mod families still need fixture-backed desktop flows.
- `.7z` and `.rar` are safely held for review right now, but there is not yet a safe supported extraction path for them.
- The native smoke lane is now stable for the current MCCC, XML Injector, and Sims 4 Community Library flows, but it still does not cover the other supported special mods yet.
- Rust still has a small set of older unused-field and unused-helper warnings that were not cleaned up in this pass.
- XML Injector older-version wording is functionally correct in the fixture app, but it may still be worth simplifying later because the queue currently explains it through the “better sibling already in Inbox” family lens.
- The Downloads refresh cleanup currently uses a short local grace window to swallow duplicate post-action reloads. That is much lighter than before, but it may still need tuning if real watcher traffic stays noisy in a large live Downloads folder.

## Important Decisions

- Keep local installed-vs-downloaded truth as the real authority for Inbox decisions.
- Keep official latest as helper-only extra context.
- Keep `SESSION_HANDOFF.md` as the main cross-session baton-pass file.
- Keep `docs/IMPLEMENTATION_STATUS.md` as the broader project memory.
- Keep the Sims 4 index file as reference material only.
- Keep `.7z` and `.rar` blocked for now instead of unpacking them automatically.
- Treat the Tauri desktop smoke wrapper as the preferred real-app Inbox check, not the browser preview.
- Treat normalized inner `.ts4script` content as the stronger same-version fingerprint for supported script-based special mods.

## Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Before widening more special-mod coverage, check Inbox in the user's real desktop setup again and confirm whether the freeze/over-refresh problem is now actually better in live use.
- Start by checking whether the remaining helper-only official latest sources have a safe official endpoint that the app can fetch without fighting Cloudflare.
- Keep MCCC and GitHub release parsing as the known-good online helpers.
- Then expand the real desktop fixture lane beyond MCCC, XML Injector, and Sims 4 Community Library so every supported special mod has:
  - update flow
  - same-version flow
  - older-version flow
  - partial-pack flow
- After that, use the real Tauri smoke lane to prove those supported-mod flows one by one.
