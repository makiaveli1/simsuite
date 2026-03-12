# Session Handoff

## Current Priority

- March 12, 2026: widen Inbox special-mod validation beyond MCCC now that the native desktop smoke lane is steady again.
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

## Known Problems / Gaps

- Helper-only official latest coverage is still too narrow:
  - MCCC and GitHub release pages are supported
  - Lot 51 and several CurseForge-backed supported mods still show `unknown` even though their official pages are readable today
- Real desktop fixture coverage is still strongest for MCCC. The other supported special mods still need the same end-to-end fixture checks.
- `.7z` and `.rar` are safely held for review right now, but there is not yet a safe supported extraction path for them.
- The native smoke lane is now stable for the current MCCC fixture flow, but it still does not cover the other supported special mods yet.
- Rust still has a small set of older unused-field and unused-helper warnings that were not cleaned up in this pass.

## Important Decisions

- Keep local installed-vs-downloaded truth as the real authority for Inbox decisions.
- Keep official latest as helper-only extra context.
- Keep `SESSION_HANDOFF.md` as the main cross-session baton-pass file.
- Keep `docs/IMPLEMENTATION_STATUS.md` as the broader project memory.
- Keep the Sims 4 index file as reference material only.
- Keep `.7z` and `.rar` blocked for now instead of unpacking them automatically.
- Treat the Tauri desktop smoke wrapper as the preferred real-app Inbox check, not the browser preview.

## Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Start by widening helper-only official latest parsers for the supported readable official sources:
  - Lot 51 Core Library
  - XML Injector
  - Lumpinou Toolbox
  - Smart Core Script
- Then expand the real desktop fixture lane beyond MCCC so every supported special mod has:
  - update flow
  - same-version flow
  - older-version flow
  - partial-pack flow
- After that, use the real Tauri smoke lane to prove those supported-mod flows one by one.
