# Session Handoff

## Current Priority

- March 12, 2026: finish Inbox special-mod correctness before widening helper-only latest checks.
- The highest-priority remaining gap is the real desktop apply smoke harness. The backend family-ranking bug is fixed, but the native apply smoke still needs a steadier end-to-end check.

## What Changed This Session

- March 12, 2026: made the repo memory process official in `AGENTS.md`.
- March 12, 2026: added this rolling handoff file as the first thing to read next session.
- March 12, 2026: updated `docs/IMPLEMENTATION_STATUS.md` to record the latest Inbox findings, the real desktop test results, and the current known gaps.
- March 12, 2026: recent landed Inbox work now includes safer special-mod review links, safer archive handling for `.7z` and `.rar`, narrower watcher refreshes, lighter special-mod queue summaries, and an isolated real Tauri desktop smoke lane.
- March 12, 2026: fixed the special-mod family chooser so already-applied fuller packs stay in family comparison instead of disappearing.
- March 12, 2026: leftover partial family items now settle into a covered-already state instead of telling the user to open the wrong Inbox sibling.
- March 12, 2026: added a backend regression test for the exact post-apply MCCC family bug.
- March 12, 2026: added temporary body-text debug output to the native apply smoke so future harness failures are easier to understand.

## What Was Tested

- March 12, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- March 12, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed.
- March 12, 2026: `npm run build` passed.
- March 12, 2026: `npm run tauri build -- --debug` passed.
- March 12, 2026: real Tauri desktop smoke passed with fixture folders through `npm run desktop:driver:fixtures` and `npm run desktop:smoke`.
- March 12, 2026: real desktop MCCC apply was checked in the fixture app. The update completed, preserved the `.cfg` file, and refreshed Inbox, but the post-apply family guidance was wrong.
- March 12, 2026: backend regression test `applied_full_family_pack_keeps_leftover_partial_out_of_the_waiting_lane` passed.
- March 12, 2026: real Tauri desktop base Inbox smoke still passes after the family fix.
- March 12, 2026: real Tauri desktop apply smoke is still flaky. It timed out in the helper lane, and one debug run showed stale or mixed fixture state.

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
- Backend family truth is now better:
  - a fuller applied special-mod pack can still anchor the family after install
  - a weaker leftover sibling no longer gets the wrong “open that item first” action in the backend decision path
- Local installed-vs-downloaded comparison is still the trustworthy decision path.
- Official latest stays helper-only and does not block the local decision flow.

## Known Problems / Gaps

- The original post-apply family-ranking bug is fixed in backend logic and tests, but the native apply smoke has not yet proven the full desktop path cleanly enough.
- The native apply smoke helper is still unstable:
  - some runs time out before the apply action appears
  - one debug run showed stale or mixed fixture state instead of a clean fresh fixture app
- Helper-only official latest coverage is still too narrow:
  - MCCC and GitHub release pages are supported
  - Lot 51 and several CurseForge-backed supported mods still show `unknown` even though their official pages are readable today
- Real desktop fixture coverage is still strongest for MCCC. The other supported special mods still need the same end-to-end fixture checks.
- `.7z` and `.rar` are safely held for review right now, but there is not yet a safe supported extraction path for them.

## Important Decisions

- Keep local installed-vs-downloaded truth as the real authority for Inbox decisions.
- Keep official latest as helper-only extra context.
- Keep `SESSION_HANDOFF.md` as the main cross-session baton-pass file.
- Keep `docs/IMPLEMENTATION_STATUS.md` as the broader project memory.
- Keep the Sims 4 index file as reference material only.
- Keep `.7z` and `.rar` blocked for now instead of unpacking them automatically.

## Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Start by stabilizing the real Tauri apply smoke harness so it always launches a clean fixture app and can reliably reach the apply action.
- After the harness is steady, re-check the post-apply MCCC flow in the real fixture app and confirm the UI now matches the fixed backend family logic.
- After that, widen helper-only official latest parsers for the supported readable official sources:
  - Lot 51 Core Library
  - XML Injector
  - Lumpinou Toolbox
  - Smart Core Script
- Then expand the real desktop fixture lane beyond MCCC so every supported special mod has:
  - update flow
  - same-version flow
  - older-version flow
  - partial-pack flow
