# Session Handoff

## Current Priority

- March 12, 2026: finish Inbox special-mod correctness before widening helper-only latest checks.
- The highest-priority known bug is a real Inbox truth bug after a safe MCCC apply: the family view can wrongly point back to a partial blocked sibling as the "better" pack.

## What Changed This Session

- March 12, 2026: made the repo memory process official in `AGENTS.md`.
- March 12, 2026: added this rolling handoff file as the first thing to read next session.
- March 12, 2026: updated `docs/IMPLEMENTATION_STATUS.md` to record the latest Inbox findings, the real desktop test results, and the current known gaps.
- March 12, 2026: recent landed Inbox work now includes safer special-mod review links, safer archive handling for `.7z` and `.rar`, narrower watcher refreshes, lighter special-mod queue summaries, and an isolated real Tauri desktop smoke lane.

## What Was Tested

- March 12, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- March 12, 2026: `npm run build` passed.
- March 12, 2026: `npm run tauri build -- --debug` passed.
- March 12, 2026: real Tauri desktop smoke passed with fixture folders through `npm run desktop:driver:fixtures` and `npm run desktop:smoke`.
- March 12, 2026: real desktop MCCC apply was checked in the fixture app. The update completed, preserved the `.cfg` file, and refreshed Inbox, but the post-apply family guidance was wrong.

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
- Local installed-vs-downloaded comparison is still the trustworthy decision path.
- Official latest stays helper-only and does not block the local decision flow.

## Known Problems / Gaps

- Real correctness bug: after a safe MCCC apply, Inbox can rank a partial blocked sibling above the full applied pack and suggest the wrong next move.
- Helper-only official latest coverage is still too narrow:
  - MCCC and GitHub release pages are supported
  - Lot 51 and several CurseForge-backed supported mods still show `unknown` even though their official pages are readable today
- Real desktop fixture coverage is still strongest for MCCC. The other supported special mods still need the same end-to-end fixture checks.
- The apply smoke helper script still needs a small follow-up so it can drive the confirmation prompt and post-apply wording automatically.
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
- Reproduce and fix the post-apply family-ranking bug in the real fixture-based desktop app.
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
