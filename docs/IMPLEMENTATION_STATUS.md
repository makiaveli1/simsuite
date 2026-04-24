# SimSuite Implementation Status

## Current session note (April 24, 2026 - Staging command wiring)

This session continued the correctness sprint by wiring the already-exposed Staging screen to its existing backend commands.

Important changes and findings:

- the frontend already called four Staging commands:
  - `get_staging_areas`
  - `cleanup_staging_areas`
  - `commit_staging_area`
  - `commit_all_staging_areas`
- the Rust command functions already existed, but they were missing from the Tauri invoke handler.
- all four commands are now registered in `src-tauri/src/lib.rs`.
- a new regression test checks the actual `generate_handler!` block so Staging commands cannot quietly become unregistered again.
- running the focused test before the fix failed on the missing command, then passed after wiring.
- `cargo fmt` also normalized existing Rust formatting in several backend files.

Checks passed:

- `cargo test --manifest-path src-tauri/Cargo.toml staging_commands_are_registered_with_tauri`
- `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml` (`216` tests)

Important remaining gap:

- Staging is registered now, but it still needs a real desktop click-through with fixture data.
- Staging still uses browser confirmation dialogs for reject actions.
- Folder view still needs backend-native contents and summaries before it is ready for huge libraries.
- Real dependency detection, missing mesh detection, recolor-to-mesh linking, and safe-delete preflight are still not implemented.

## Current session note (April 24, 2026 - Library relationship wording pass)

This session continued the Library correctness sprint by making the relationship and removal language more honest.

Important changes and findings:

- Library detail warnings now say `Check before removing` instead of implying safe-delete support.
- duplicate warnings now ask the user to compare matching files before removing either copy.
- script-mod warnings now say other mods can sometimes rely on script files, without claiming SimSuite proved a dependency.
- the Library detail sheet now presents relationship data as clues:
  - `Known file facts`
  - `Likely related clues`
  - `Possible placement clues`
- same-folder text now says it confirms shared placement, not a dependency.
- the detail sheet section is now `Related File Clues`, with a hint that hidden dependencies, missing meshes, and safe deletion are not proved yet.
- relationship badge suffixes now use plain labels like `Confirmed`, `Likely`, and `Possible` instead of raw internal words.
- new tests cover the wording so older proof-heavy copy does not slip back in.

Checks passed:

- `npm run test:unit -- src/screens/library/LibraryDetailsPanel.test.tsx src/screens/library/libraryRelationships.test.tsx`
- `npm run test:unit -- src/screens/library` (`6` files, `28` tests)
- `npm run test:unit` (`11` files, `38` tests)
- `npm run build` with the existing Vite chunk-size warning

Important remaining gap:

- Staging is still exposed but not wired through registered Tauri commands.
- Folder view still needs backend-native contents and summaries before it is ready for huge libraries.
- Real dependency detection, missing mesh detection, recolor-to-mesh linking, and safe-delete preflight are still not implemented.

## Current session note (April 24, 2026 - Library correctness sprint start)

This session began the recommended Library correctness sprint.

Important changes and findings:

- `list_library_files` no longer builds invalid SQL in either paged or unpaged mode
- Rust tests now compile again after the old `inspect_file` test calls were updated for the newer thumbnail flag
- `cargo test` now runs and passes again
- `get_folder_tree_metadata` no longer depends on unsupported SQLite `REVERSE()`
- folder metadata is now built in `library_index` from lightweight path rows, with nested folder counts covered by test
- nested folder selection in the frontend folder helper now works below the first child level
- Library relationship peer counts are less misleading:
  - same-folder counts use real parent folder paths
  - same-pack counts ignore files with no `bundle_id`
- a Downloads queue overview bug was also fixed after the full Rust suite exposed it:
  - hydrated same-version special downloads now keep the `Done` lane count
  - empty searches keep global overview counts instead of replacing them with zeroed visible rows

Checks passed:

- `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml` (`215` tests)
- `npm run test:unit` (`10` files, `36` tests)
- `npm run build` with the existing Vite chunk-size warning
- `cargo check --manifest-path src-tauri/Cargo.toml` with existing warnings
- `cargo build --manifest-path src-tauri/Cargo.toml --release` with existing warnings

Important remaining gap:

- Staging is still exposed but not wired through registered Tauri commands.
- Folder view still needs backend-native contents and summaries before it is ready for huge libraries.
- Relationship/dependency language still needs careful product cleanup because true dependency proof and safe-delete preflight do not exist yet.

## Current session note (April 24, 2026 - Library system audit)

This session was audit-only. No app behavior was intentionally changed.

New report:

- `simsuite-reports/LIBRARY_SYSTEM_AUDIT_CURRENT_STATE.md`

What the audit found:

- the Library has real backend and frontend systems, including scanner indexing, SQLite storage, list/grid/folder views, detail surfaces, duplicate detection, update-watch plumbing, review routing, and first-party thumbnail work
- the Library is not production-ready yet
- the most urgent issue is `list_library_files` SQL in `src-tauri/src/core/library_index/mod.rs`; it currently has an extra comma before `FROM files f`, which can break list/grid/folder loading in the real Tauri app
- `get_folder_tree_metadata` likely fails because it uses SQLite `REVERSE()` without a registered custom function
- folder view still depends on full row loading for useful contents, and deep nested folder selection can return empty contents
- relationship/dependency UI exists, but real dependency detection, missing mesh detection, recolor-to-mesh linking, and safe-delete preflight are not implemented
- duplicate detection is real, but cleanup actions are not implemented
- update-watch support is real but limited; CurseForge is provider-required/future work, not full automatic updating
- Staging is exposed in navigation and frontend API, but its Tauri commands are not registered
- AI infrastructure is not implemented beyond a planned-status placeholder

Checks run:

- `npm run build` passed, with a Vite chunk-size warning
- `npx tsc --noEmit` passed
- `npm run test:unit` passed: 9 files, 35 tests
- `cargo check` passed with warnings
- `cargo build --release` passed with warnings
- `cargo test` failed before tests ran because old `inspect_file` test calls are missing the new `defer_thumbnails` argument

Recommended next sprint:

1. Fix Library runtime SQL and add tests.
2. Fix `cargo test`.
3. Fix folder metadata/folder content truth.
4. Correct relationship counts before building deeper dependency or safe-delete features.

## Current session note (March 19, 2026 - late night library implementation)

This session turned the approved `Library` spec into the first real code pass for the calmer `Quiet Catalog` design.

Important changes and findings:

- the `Library` implementation plan now exists at:
  - `docs/superpowers/plans/2026-03-19-library-redesign.md`
- a tested display helper layer was added in:
  - `src/screens/library/libraryDisplay.ts`
  - `src/screens/library/libraryDisplay.test.ts`
- `Library` now has a calmer shell:
  - `src/screens/library/LibraryTopStrip.tsx`
  - `src/screens/library/LibraryFilterRail.tsx`
  - `src/screens/library/LibraryFilterRail.test.tsx`
- the top strip now owns:
  - shown count
  - total count
  - active filter count
  - search
  - `More filters`
- the left rail is lighter and now keeps only the core narrowing controls on screen
- the middle list is now a dedicated component:
  - `src/screens/library/LibraryCollectionTable.tsx`
  - `src/screens/library/LibraryCollectionTable.test.tsx`
- Library rows no longer lead with full paths
- rows now focus on:
  - file name
  - type
  - short health signal
  - a small set of clues based on the current user view
- the right side no longer keeps the creator and type editor open all the time
- a shorter understanding panel now lives on the page:
  - `src/screens/library/LibraryDetailsPanel.tsx`
  - `src/screens/library/LibraryDetailsPanel.test.tsx`
- deeper detail moved into an on-demand sheet:
  - `src/screens/library/LibraryDetailSheet.tsx`
- the side sheet now handles:
  - health details
  - inspect file
  - edit details
- `LibraryScreen.tsx` was updated to orchestrate the new pieces instead of rendering the old all-in-one inspector layout
- `src/styles/globals.css` was updated for the new Library strip, rail, row, panel, and sheet styling
- fresh live screenshots were saved for:
  - `output/playwright/library-pass2-casual.png`
  - `output/playwright/library-pass2-seasoned.png`
  - `output/playwright/library-pass2-creator.png`
  - `output/playwright/library-pass2-inspect-sheet.png`
- checks passed:
  - `npm ci`
  - `npm run test:unit -- libraryDisplay`
  - `npm run test:unit -- LibraryFilterRail`
  - `npm run test:unit -- LibraryCollectionTable`
  - `npm run test:unit -- LibraryDetailsPanel`
  - `npm run test:unit -- libraryDisplay LibraryFilterRail LibraryCollectionTable LibraryDetailsPanel`
  - `npm run build`

Important remaining gap:

- `LibraryScreen.tsx` still has more orchestration and leftover helper logic than it should after this pass
- the Library redesign is implemented on the feature branch, but it is not merged to `main` yet
- creator-mode richness is now mostly in the row clues, `More filters`, and the inspect sheet; the main page itself still stays intentionally restrained

## Current session note (March 19, 2026 - late night library design)

This session did not start coding `Library`. It completed the full design-spec phase for the page instead.

Important changes and findings:

- `Downloads` was approved, merged into `main`, and pushed to GitHub before moving to the next page
- the current `Library` screen was reviewed as both code and live UI
- the shared Sims mod manager PRD helped confirm what simmers want to see at each experience level
- the real job of `Library` is now explicitly locked:
  - a calm place to browse and understand the collection
  - not a place where every management flow stays open at once
- the approved redesign direction is:
  - `Quiet Catalog`
- the full written spec now exists at:
  - `docs/superpowers/specs/2026-03-19-library-redesign-design.md`
- the spec locks:
  - a slim top strip
  - a lighter left filter rail
  - a list-first center catalog
  - a short right understanding panel
  - on-demand sheets for:
    - health details
    - inspect file
    - edit details
  - clear page boundaries between `Library`, `Updates`, `Review`, `Organize`, `Creators`, and `Types`
  - view-specific priorities for `Casual`, `Seasoned`, and `Creator`
  - restrained motion and calmer interaction rules
- fresh live screenshots of the current page were saved for:
  - `output/playwright/library-design-casual-current.png`
  - `output/playwright/library-design-seasoned-current.png`
  - `output/playwright/library-design-creator-current.png`

Important remaining gap:

- no `Library` implementation code has been started yet
- the next required step is user review of the written spec, followed by the implementation plan
- the expected spec-review subagent loop was not cleanly available in this environment, so the spec was manually reviewed against the same design sections and constraints instead

## Current session note (March 19, 2026 - deep night follow-up)

This follow-up pass fixed one of the last visually awkward spots in `Downloads`: the special-setup action block that still felt bulky and under-organized.

Important changes and findings:

- the action card no longer repeats a long filename as both the heading and the button text
- a new helper now separates:
  - the readable card title
  - the shorter CTA label
  - the running/progress label
- new helper file:
  - `src/screens/downloads/reviewActionText.ts`
- new focused test:
  - `src/screens/downloads/reviewActionText.test.ts`
- the new test locks the intended behavior for `open_related_item`:
  - the filename stays in the card title
  - the button stays short
  - the in-progress label still reads clearly
- `DownloadsScreen.tsx` now uses the new helper in the repair CTA and the smaller action rows
- the action-card styling was tightened in `src/styles/globals.css`:
  - top-aligned content
  - calmer spacing
  - cleaner title wrapping
  - smaller CTA
  - stacked CTA on narrower widths
- live check:
  - the exact original `Use McCmdCenter...` fixture was not exposed in the current live mock data
  - but the same shorter CTA treatment was confirmed live in the MCCC blocked path
- fresh screenshot from this pass:
  - `output/playwright/downloads-pass15-action-card-tighten.png`
- checks passed:
  - `npm run test:unit -- src/screens/downloads/reviewActionText.test.ts`
  - `npm run test:unit -- src/screens/downloads/DownloadsDecisionPanel.test.tsx src/screens/downloads/DownloadsBatchCanvas.test.tsx src/screens/downloads/reviewActionText.test.ts`
  - `npm run build`

Important remaining gap:

- `DownloadsScreen.tsx` still has too much page orchestration in one file
- the exact user-reported action state was covered by unit test rather than a live screenshot because the current mock flow did not surface that action on screen

## Current session note (March 19, 2026 - deep night)

This session pushed the `Downloads` rebuild out of the awkward middle state and made it feel much closer to the approved quiet staging desk.

Important changes and findings:

- the new proof sheet and action dialog stayed in place, but the main follow-up work was deeper than that:
  - the center stage still felt too report-like
  - creator filters were still taking up too much rail space
  - some actions still fell back to browser confirmation prompts
- `Downloads` now uses more real details-on-demand behavior:
  - `More filters` opens as a floating filter panel instead of stretching the left rail
  - that filter panel now defaults to closed in every view
  - the stage detail is now tucked behind tabs instead of staying fully expanded
- `GuidedPreviewPanel` now uses a tabbed detail deck for:
  - plan
  - dependencies
  - notes
  - why
- `SpecialReviewPanel` now uses a tabbed detail deck for:
  - reason
  - dependencies
  - tracked files
- `Casual`, `Seasoned`, and `Creator` now differ more cleanly in the stage:
  - `Casual`
    - fewer summary stats
    - calmer labels
    - simpler main story before deeper tabs
  - `Seasoned`
    - a more balanced stage with the extra context tucked behind the new tabs
  - `Creator`
    - still gets the fuller proof-oriented detail, but no longer all at once
- split-stage layouts now use the left lower space more intentionally:
  - the queue column gets a small helper card instead of ending in a large dead patch
- the last browser-style confirms were removed from the inbox flow
- inbox actions now use the in-app dialog pattern for:
  - safe move
  - ignore
  - guided apply
  - review actions that need approval
- motion also got a small polish pass:
  - queue and stage scroll areas now use Motion layout-aware scrolling
  - stage tabs use an animated shared highlight
  - the filter popover fades and settles instead of snapping in
- live checks were run again on `Downloads` in:
  - `Casual`
  - `Seasoned`
  - `Creator`
- fresh screenshots from this pass were saved in:
  - `output/playwright/downloads-pass14-casual-main.png`
  - `output/playwright/downloads-pass14-seasoned-main.png`
  - `output/playwright/downloads-pass14-creator-main.png`
  - `output/playwright/downloads-pass14-creator-dialog.png`
- checks passed:
  - `npm run test:unit`
  - `npm run build`

Important remaining gap:

- the open Playwright browser kept one old React dependency-array warning in its console after hot updates; after a full reload the page behaved normally and the production build passed, so this currently looks like a stale hot-reload artifact rather than a fresh app bug
- `DownloadsScreen.tsx` still carries a lot of orchestration and could be split further later if we want the code to match the calmer UI
- the proof sheet is working well, but its section spacing and controls can still take one final taste pass later

## Current session note (March 19, 2026 - late night)

This session did not start code for `Downloads`. It completed the full design-spec phase for the page instead.

Important changes and findings:

- the current `Downloads` screen was re-read and re-checked as a real workflow instead of just a layout
- the screen's core purpose is now explicitly locked:
  - a calm staging desk for newly downloaded mods before anything reaches the game
- three redesign directions were explored:
  - `Quiet Staging Desk`
  - `Tabbed Staging Rooms`
  - `Queue First`
- the approved direction is:
  - `Quiet Staging Desk`
- the full written spec now exists at:
  - `docs/superpowers/specs/2026-03-19-downloads-redesign-design.md`
- the implementation plan now also exists at:
  - `docs/superpowers/plans/2026-03-19-downloads-redesign.md`
- the spec defines:
  - a slim utility top strip
  - a quiet left rail for watcher state, lane picking, search, and on-demand filters
  - a queue-first center workspace with a lane-aware batch canvas
  - a short right decision panel instead of a full receipt wall
  - side sheets for proof, versions, source details, and full file lists
  - a focused dialog for guided setup and major confirmation flows
  - one shared page shape across `Casual`, `Seasoned`, and `Creator`
  - restrained motion with reduced-motion-safe fallbacks
- live screenshots of the current screen were saved for all three user views:
  - `output/playwright/downloads-design-pass/downloads-casual-current.png`
  - `output/playwright/downloads-design-pass/downloads-seasoned-current.png`
  - `output/playwright/downloads-design-pass/downloads-creator-current.png`
- the implementation plan breaks the rebuild into five real slices:
  - lightweight frontend test harness and Downloads display rules
  - shell split for top strip and left rail
  - queue and lane-aware center stage rebuild
  - short decision panel plus proof sheet and setup dialog
  - final motion, empty-state, and cross-view polish

Important remaining gap:

- this session stopped at the spec gate on purpose
- the `Downloads` redesign is still not implemented yet
- the spec and implementation plan are both ready
- the next required step is to pick the execution path and begin the actual code work from the saved plan
- the brainstorming workflow expected a separate spec-review subagent loop, but that path was not cleanly available in this environment during this pass, so the spec review was done manually against the same checklist instead
- the writing-plans workflow also expected a separate plan-review subagent loop, and that was also handled manually against the same checklist because the environment still did not expose a clean dispatch path

## Current session note (March 19, 2026 - early night)

This session started the deeper screen-by-screen redesign program with a full rethink of `Home`.

Important changes and findings:

- the repo now has persistent design context in:
  - `.impeccable.md`
- the `Home` redesign spec is now written in:
  - `docs/superpowers/specs/2026-03-19-home-redesign-design.md`
- `Home` no longer behaves like a command board with a permanent right inspector
- the page is now a calmer centered landing surface with:
  - one main hero area
  - a smaller set of glance modules
  - a right-side `Customize Home` sheet
- the page stopped using navigation-style action lists as the main content
- only two direct actions stay visible:
  - `Customize Home`
  - `Scan`
- `Customize Home` now supports:
  - hero focus choice
  - show/hide home modules
  - theme
  - spacing density
  - ambient hero motion
- these settings are saved per user view, but they now stay in the right lane:
  - `Casual`, `Seasoned`, and `Creator` decide how much information `Home` shows
  - `Customize Home` only handles personal preference like focus, visible modules, theme, spacing, and motion
- after the first `Home` pass, `Seasoned` and `Creator` were tightened further so they no longer read like one flat group of equally weighted cards
- the denser views now use clearer bands:
  - `Seasoned`
    - snapshot + system health
    - update watch + folders
  - `Creator`
    - update watch + system health
    - snapshot + folders
    - library facts as a full-width lower strip
- live checks were run on `Home` in:
  - `Casual`
  - `Seasoned`
  - `Creator`
- the new side sheet was also checked live
- fresh screenshots were saved in:
  - `output/playwright/home-pass10-casual-after.png`
  - `output/playwright/home-pass10-seasoned-verified.png`
  - `output/playwright/home-pass10-creator-after.png`
  - `output/playwright/home-pass10-customize-sheet.png`
  - `output/playwright/home-pass10-seasoned-tightened.png`
  - `output/playwright/home-pass10-creator-tightened.png`
  - `output/playwright/home-pass11-customize-sheet.png`
- checks passed:
  - `npm run build`
  - `npm run build` after removing the conflicting extra detail-level control
  - `npm run build` after the motion-and-atmosphere polish pass

- `Home` also got a real polish pass after the structure was already in place:
  - shared motion was softened so controls, screen changes, and side sheets feel calmer
  - the `Home` hero now has two visibly different atmosphere states:
    - `Still`
      - flatter and quieter
    - `Ambient`
      - soft light ring
      - slow sweep line
      - richer hero glow
  - the old problem where `Still` and `Ambient` felt the same is now fixed
  - live checks were run again in:
    - `Casual`
    - `Seasoned`
    - `Creator`
  - fresh comparison screenshots were saved in:
    - `output/playwright/home-pass12-creator-still-v2.png`
    - `output/playwright/home-pass12-creator-ambient-v2.png`

Important remaining gap:

- the `Customize Home` sheet works and looks better now, but its theme section is still the longest part of the panel and could take one more density pass later
- the same deeper rethink still needs to be carried across the rest of the app one page at a time
- `Creator` `Home` is intentionally fuller, but it is still the first place to tune if we want even more calm without losing useful information
- the richer atmosphere polish is only on `Home` right now; other screens still use the calmer shared motion, but not the same optional atmosphere treatment

## Current session note (March 18, 2026 - evening)

This session took the redesign into a stronger “show less, mean more” direction and focused on the two screens that still kept too much open at once:

- `Settings`
- `Updates`

Important changes and findings:

- `Settings` is no longer one tall stack of always-open sections
- it now behaves more like a proper desktop preferences window:
  - left side for section choices
  - right side for the currently selected group
  - smaller saved-state summary tucked into the side
- this especially helped with the feeling that too many decisions were shouting at once
- `Background and updates` also fits the new shape:
  - close behavior stays in one block
  - watched-page automation stays in another block
  - the section is still detailed, but it is not fighting the rest of the settings page anymore
- `Updates` now uses a better details-on-demand pattern:
  - the source form is no longer pinned inside the inspector
  - source editing now opens in a right-side sheet
  - the inspector stays focused on status, proof, and the next action
  - clearing a saved source moved into the sheet so it is not always taking up space
- shared styling was added for:
  - the new workbench side sheet
  - calmer settings section buttons
  - the focused settings detail layout
- the live app was rechecked after restarting Vite at `http://127.0.0.1:1420/`
- live visual checks covered:
  - `Settings` in `Creator`
  - `Settings` in `Casual`
  - `Updates` in `Creator`
  - `Updates` in `Casual`
- fresh screenshots were saved in:
  - `output/playwright/pass8-settings-after.png`
  - `output/playwright/pass8-updates-after.png`
- checks passed:
  - `npm run build`

Important remaining gap:

- the current live data only exposed one built-in tracked update source and no setup items
- because of that, the new `Updates` side sheet could be verified through build success and code review, but not fully opened in a real live fixture during this pass
- `Settings` is much calmer now, but very short sections still leave some quiet space below the active detail panel; this is better than clutter, but could still take a final taste pass later

## Current session note (March 18, 2026 - afternoon)

This session stayed in the screenshot-driven desktop-app lane, but instead of rebuilding one or two screens, it did a full consistency pass across the whole app:

- `Home`
- `Downloads`
- `Library`
- `Updates`
- `Organize`
- `Review`
- `Creators`
- `Types`
- `Duplicates`
- `Settings`

Important changes and findings:

- the left rail was too boxy and too loud compared with the workspaces
- the rail is now calmer:
  - less constant border weight
  - less jumpy icon behavior
  - better visual priority for the active screen
- a shared screen-shell issue was still leaving several screens with too much dead lower space
- the workbench page shell now lets the last main workspace section fill the remaining canvas better
- this especially helped:
  - `Review`
  - `Creators`
  - `Types`
  - `Duplicates`
- `Home` also got one small content cleanup:
  - the right inspector no longer repeats the full next-action description again
- panel surfaces, rows, chips, and stage cards were all softened so the app reads more like one calm desktop tool instead of many similar dark boxes
- `Updates` and `Review` footer cards now stretch more naturally and feel less tacked on
- the audit screens now use a real working height so the middle stage feels fuller
- the live app was checked again in all three experience modes:
  - `Seasoned`
  - `Casual`
  - `Creator`
- fresh screenshots were saved in:
  - `output/playwright/pass7-after/`
- contact sheets were also saved for fast comparison:
  - `seasoned-sheet.png`
  - `casual-sheet.png`
  - `creator-sheet.png`
- checks passed:
  - `npm run build`

Important remaining gap:

- `Updates` still has some quiet lower-canvas space when the tracked list is very short
- `Home` still has a slightly calm lower-right area when the folders section is short
- `Settings` is improved, but it is still one of the densest screens and could take one more grouping pass later


## Current session note (March 18, 2026 - midday)

This session stayed in the screenshot-driven desktop-app lane and focused on the last three big screens that still needed cross-view cleanup:

- `Home`
- `Updates`
- `Review`

Important changes and findings:

- `Home` was rebuilt into a fuller command board:
  - stronger main-stage primary action
  - tracked-pages panel added to the stage
  - calmer but more useful right inspector
  - much less of the “top-heavy, empty lower half” feeling
- `Updates` now gives the main list the width back:
  - selected-file story moved into a top focus band
  - the extra explanation moved into a lower footer row
  - list columns were simplified so tracked/setup/review rows fit more naturally
- `Review` now uses its center better:
  - queue focus card added below the list
  - best-next-fix card added beside it
  - this helps tiny queues still feel like a real workspace instead of a sparse queue floating in space
- the live app was checked in all three experience modes:
  - `Casual`
  - `Seasoned`
  - `Creator`
- extra manual live screenshots were needed for some `Review` captures because the quick automated capture could land mid-transition after tall screens like `Settings`; the actual rendered screen was checked manually after that
- checks passed:
  - `npm run build`

Important remaining gap:

- `Home` now feels much better, but the lower-right stage area can still go a little quiet when the folder block is short
- `Updates` tracked mode is cleaner, but one-row cases could still take one more density pass later
- `Review` is much improved, but very tiny queues still leave some calm empty space in the center stage

## Current session note (March 18, 2026 - morning)

This session kept the redesign in the screenshot-driven desktop-app lane and focused on three screens that still carried too much clutter:

- `Creators`
- `Duplicates`
- `Organize`

Important changes and findings:

- `Creators` no longer uses the repeated top teaching strip
- the screen now keeps the useful counts, adds one quieter note in the left rail, and lets the actual group list, sample files, and save panel do the talking
- `Duplicates` got the biggest structural cleanup:
  - left rail for counts, filters, and layout presets
  - center comparison stage for the selected pair
  - right inspector for deeper proof
- `Duplicates` also had one last layout bug fixed during visual checking:
  - the lower queue rows were sitting too low inside their panel
  - the panel now stacks its heading, note, and queue naturally so the rows start where users expect
- `Organize` was tightened instead of rebuilt:
  - added a short “safe path first” note
  - compressed the left rail cards
  - made the left rail scroll on its own so the full page stays more desktop-like
- fresh live visual checks were run after the changes for:
  - `Creators`
  - `Duplicates`
  - `Organize`
- checks passed:
  - `npm run build`

Important remaining gap:

- `Duplicates` now feels like a real compare desk, but tiny result sets still leave some quiet space in the lower queue region
- `Organize` is calmer, but there is still room for one more taste-level density pass
- `Home` still needs the placeholder right side replaced
- the next best screenshot-driven cleanup targets are:
  - `Home`
  - `Review`
  - `Updates`

This document maps the current implementation to the active product requirements.

## Current session note (March 18, 2026 - near dawn)

This session stayed in the desktop-first cleanup lane and focused on two screens that still felt visually off even after the bigger workbench move:

- `Library`
- `Types`

Important changes and findings:

- `Library` was not just plain; its left filter rail was effectively sprawling across the screen
- that was making the middle file table feel squeezed into the far right side and was a big reason the page looked hollow
- `Library` now has:
  - a real narrow filter rail
  - a stronger center-stage header with counts and selected-file focus
  - a direct `Open in Updates` handoff in the stage
  - stacked rail filters instead of a wide horizontal form row
- `Types` now starts the work sooner:
  - the repeated top three-step strip was removed
  - a smaller guidance note now lives inside the left panel instead
  - the summary strip is still there, but tighter
- fresh live visual checks were run after the changes for:
  - `Library`
  - `Types`
- checks passed:
  - `npm run build`

Important remaining gap:

- `Library` still leaves some quiet lower-stage space when the result list is very short
- `Types` is cleaner, but the right inspector is still a little busier than the newest screens
- the same screenshot-driven cleanup should keep moving into:
  - `Creators`
  - `Duplicates`
  - `Organize`

## Current session note (March 18, 2026 - later night)

This session kept the desktop-first redesign moving, but the most important result was finding and fixing one shared page-shell layout bug that was quietly making several screens look much stranger than their actual screen structure.

Important changes and findings:

- `Review`, `Organize`, `Duplicates`, `Creator Audit`, and `Category Audit` were using the shared `.workbench` class directly on the page shell
- that class belongs to the split-pane `Workbench` component and was forcing a three-column outer page grid
- that bug was a big reason those screens still looked awkward in screenshots:
  - dead left space
  - off-balance headers
  - panels appearing shifted or top-heavy
- a new `workbench-screen` override now lets those page shells keep the quieter workbench look without inheriting the wrong outer grid
- `Review` also got a real desktop cleanup:
  - proper header row
  - left rail with queue health and reason groups
  - central queue stage
  - right inspector kept for selected detail
- `Updates` got a stronger center stage:
  - selected-file spotlight
  - mode-specific counts
  - short lane guidance
  - this reduces the “one row in a huge empty panel” feeling
- fresh live visual checks were run after the changes for:
  - `Review`
  - `Updates`
  - `Organize`
  - `Creator Audit`
  - `Duplicates`
- checks passed:
  - `npm run build`

Important remaining gap:

- `Updates` and `Review` are much better, but both can still feel a little visually quiet when their lists are very short
- `Library` still needs a fresh screenshot-driven pass in the newer desktop shell
- `Types` should be visually rechecked after the shared page-shell fix even though the structural bug fix should already help it

## Current session note (March 18, 2026 - visual cleanup)

This session did not change ownership or flow. It was a visual cleanup pass on the new `Downloads` workbench so the screen stops feeling like a stack of equal-weight cards.

Important changes and findings:

- the `Downloads` rail was quieted:
  - slightly narrower
  - vertical filter stack
  - calmer secondary actions
  - lane summaries flattened into a quicker status list
- the center stage was cleaned up:
  - boxed stage stats became compact chips
  - extra corner accents and heavier panel feel were removed from the `Downloads` workbench surfaces
  - queue rows were softened so the selected batch can lead the screen
- the right inspector was simplified:
  - signal cards are lighter
  - the main next-step card is more clearly the primary focus
  - docked detail sections read more like one inspector and less like many separate mini-panels
- checks passed:
  - `npm run build`

Important remaining gap:

- no fresh real desktop click-through or screenshot signoff was run after this visual pass
- `Downloads` still needs a real-eye check in guided/review states
- the same cleanup still needs to reach `Home`, `Review`, `Duplicates`, and the audit screens

## Current session note (March 18, 2026 - later)

This session pushed the desktop-first redesign into `Downloads`, which was still one of the most crowded workspaces.

Important changes and findings:

- `Downloads` now follows the same pane-based workbench pattern as the newer `Updates` screen:
  - left control rail
  - center queue + preview stage
  - right inspector
- watcher status, search, filters, tidy style, lane counts, and quick actions moved into the left rail instead of sitting above the main work surface
- the center stage now opens with a small status line and then goes straight into the working area:
  - queue
  - preview
- the deeper selected-batch detail flow stayed in the right inspector, so the structural cleanup did not disturb apply / ignore / review behavior
- shared rail and inspector content wrappers now have real base padding and layout rules, which should help the newer workbench screens stay visually consistent
- checks passed:
  - `npm run build`

Important remaining gap:

- no fresh real desktop click-through or screenshot signoff was run after this `Downloads` pass
- `Home` still needs a real inspector
- `Review`, `Duplicates`, and the audit screens still need the same desktop cleanup
- the shared rail/inspector spacing change should be visually checked on `Library` and `Updates`

## Current session note (March 18, 2026)

This session began the first real desktop-first UI restructure instead of only planning it.

Important changes and findings:

- `Updates` is now a real dedicated workspace in the app shell, not just leftover watch logic hanging off `Library`
- the `Updates` screen now uses the new pane-based workbench layout:
  - left control rail
  - central table
  - right inspector
- `Library` no longer carries the old hidden watch-center state and dead follow-up helpers
- `Library` now hands the selected file off into `Updates` instead of trying to manage tracking inside the browser view
- workspace refresh wiring now includes `updates`, so update-related changes can refresh the right surfaces together
- the shared workbench shell also had a real bug fixed:
  - class names in `Workbench`, `WorkbenchRail`, and `WorkbenchInspector` were being joined incorrectly
  - this could quietly break density and layout styling across the new workbench screens
- checks passed:
  - `npm run build`

Important remaining gap:

- this was the first big UI slice, not the full redesign
- `Downloads`, `Review`, `Duplicates`, and the audit screens still need the same desktop-workbench cleanup
- no fresh real desktop click-through or screenshot signoff was run yet after this slice

## Current session note (March 16, 2026)

This session did not add features. It was a research-backed product audit for player-facing mod information, so later `Library` polish stays useful instead of turning back into a debug screen.

Important findings:

- SimSuite should not claim universal 100% accuracy yet across all simmer libraries
- the right production rule is:
  - confirmed facts as facts
  - strong clues marked carefully
  - true unknowns left unknown
- outside research was checked against:
  - the linked `r/thesimscc` organizer discussion
  - broken-CC and missing-mesh help threads on `r/sims4cc`
  - Scarlet's mod list help pages
  - TS4 Mod Hound
  - Sims 4 Mod Manager / Overwolf feature pages
  - SimSweep feature notes

What players most consistently seemed to want:

- preview thumbnails
- creator name
- one clear "what is this?" summary
- broken / outdated / unknown / duplicate / conflict status
- missing mesh / missing requirement clues
- quick file location and removal path
- "used by this Sim / lot" style tracing
- update tracking and notes

Resulting product direction:

- beginner and seasoned `Library` views should stay tightly player-facing
- raw resource and parser detail should remain hidden in creator mode or deeper receipts
- `type`, `subtype`, and `file format` likely need to become one clearer player-facing summary instead of three separate labels
- the next useful simmer-facing info fields are likely:
  - preview
  - status
  - needs / dependency / missing-mesh hints
  - source / creator / open-folder actions
  - later, tray-based used-by tracing

Important remaining gap:

- this audit did not change app behavior yet
- the next implementation pass should come only after:
  - the `unsafe_script_depth` review decision
  - the postponed watch bug sweep

## Current session note (March 16, 2026)

This session stayed in feature freeze and finished another live-data trust pass on the last real installed-content category edge cases.

Important changes and findings:

- package inspection now has two more narrow fallback paths:
  - `uicheats`-style helper packages can use one more context-only gameplay resource clue
  - lean CAS appearance/default-replacement style packages can use a narrow CAS resource fallback when their inside-file mix stays small and clean
- filename confidence also has one more narrow cleanup layer for already-clear real filenames:
  - override/default replacements
  - pose packs
  - childbirth / pregnancy packages
- important guardrail:
  - ambiguous one-resource packages still stay unknown when there is no safe supporting context
  - `Colorful_Var_Pink.package` is the proof case for that rule
- rebuild versions were bumped again so old stored meaning could not linger:
  - `scanner-v17`
  - `downloads-assessment-v11`
- checks passed:
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `209` tests
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
  - `npm run build`
  - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`

Real live validation also passed again:

- the real user-profile database now reports `scanner-v17`
- the app completed a real automatic full scan as `sessionId 41`
- `filesScanned 13010`
- scan time was about `9` minutes
- true installed `Unknown` rows are now down to `1`
- open installed review rows are now:
  - `unsafe_script_depth 20`
  - `low_confidence_parse 1`
  - `no_category_detected 1`
  - `conflicting_category_signals 0`
- the only remaining real unknown is `Colorful_Var_Pink.package`
  - it still has only one weak package resource clue and no safe supporting creator, version, or human-readable inside-file evidence
  - keeping it `Unknown` is currently the honest outcome, not a missed easy rule

Important remaining gap:

- the remaining installed review lane is now mostly real placement safety, not category uncertainty:
  - `20` rows are `unsafe_script_depth`
  - only `1` file is still truly unknown
- before more feature work, the next decision should be whether deep script installs should:
  - keep living in `Review` as a hard safety flag
  - or move to a calmer visible warning without pretending the risk is gone
- after that, the next stabilization pass should move back to the postponed watch-system bug sweep

## Current session note (March 16, 2026)

This session stayed in feature freeze and closed the stale live-rescan gap that was leaving real library facts behind after scan-rule changes.

Important changes and findings:

- the backend now tells `Home` whether the stored library facts are stale under the current scan rules
- the app now starts one automatic library refresh per app session when:
  - library folders are configured
  - no scan is already running
  - the stored scan fingerprint is older than the current scan fingerprint
- `Home` now surfaces that state in a calm player-facing way:
  - `Library check` / `Library facts`
  - one compact refresh banner
- this closes the earlier trust gap where old indexed library facts could still be shown after a scan-rule change without any clear warning
- a small regression test pass was added for the stale-flag helper:
  - empty fresh database -> no stale warning
  - existing indexed data with an older fingerprint -> stale warning
- checks passed:
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `200` tests
  - `npm run build`
  - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`

Real live validation also passed:

- the real user-profile database now reports `scanner-v15`
- the app completed a real automatic full scan as `sessionId 39`
- `filesScanned 13010`
- scan time was about `9` minutes
- true installed `Unknown` rows are now down to `3`
- open review rows are now:
  - `unsafe_script_depth 20`
  - `low_confidence_parse 11`
  - `no_category_detected 3`
  - `conflicting_category_signals 2`

Important remaining gap:

- the stale-scan foundation problem is fixed, but stabilization is not done yet:
  - the last real `Unknown` cluster still needs targeted inspection
  - the remaining low-confidence parse rows still need another live-data pass
  - watch-system bugs still need cleanup before feature growth resumes

## Current session note (March 16, 2026)

This session stayed in feature freeze and tightened the `Library` inspector so regular simmers see calmer, more trustworthy file details instead of a debug-style panel.

Important changes and findings:

- beginner and seasoned `Library` views were simplified on purpose
- those views now focus on direct player-facing facts only:
  - creator
  - type
  - subtype when present
  - file format
  - filtered in-game names when they look human-readable
  - installed version and update state
  - safety notes and grouped-file info only when they matter
- beginner and seasoned views no longer show the heavier internal sections:
  - inside-file evidence
  - creator-learning tools
  - type-override tools
  - raw path panel
  - local version evidence dumps
  - watch evidence dumps
  - heuristic summary tags like add-on, core helper, or texture recolor
- creator mode still keeps the deeper receipts and correction tools
- one important trust rule changed in the UI:
  - if an installed version is not strong enough to trust, the player-facing view now says it is not confirmed yet
  - the uncertain value is no longer surfaced like a confirmed version in beginner or seasoned views
- outside research aligned with the same priorities:
  - creator name
  - update status
  - easy broken-CC identification
  - clear mod / CC type
  - thumbnails or in-game names when available
- checks passed:
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `198` tests
  - `npm run build`
  - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`

Important remaining gap:

- the player-facing cleanup is in place, and the later stale-profile rebuild gap has now been closed
- the next stabilization work should stay focused on the last unknown files and watch bugs

## Current session note (March 16, 2026)

This session stayed in feature freeze and focused on two stabilization goals: better player-facing mod details in `Library`, and one more safe cleanup pass for the last obvious Build/Buy filename cluster.

Important changes and findings:

- `Library` now has a new plain-English summary layer for installed files:
  - it explains what the file seems to be in simmer-friendly language instead of only showing parser-style fields
  - it now surfaces:
    - a plain-English summary
    - file format
    - best version clue
    - useful role tags
    - in-game names
    - related family hints
    - friendlier version evidence lines
- the older technical inspection block is still available, but several labels are now clearer for humans:
  - `Creator hints` -> `Creator names found`
  - `Version hints` -> `Version numbers found`
  - `Resources` -> `Package contents`
  - `Namespaces` -> `Script folders`
  - `Embedded names` -> `In-game names`
- filename classification was widened again for a small safe Build/Buy cluster:
  - added `entryway`, `entrance`, `barback`, and `fireplace`
  - subtype mapping now treats those as:
    - `Build Surfaces` for entryway / entrance
    - `Furniture` for barback / fireplace
- rebuild versions were bumped again so old stored meaning cannot linger:
  - `scanner-v15`
  - `downloads-assessment-v9`
- checks passed again:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `198` tests
  - `npm run build`
  - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`

Important remaining gap:

- real live verification is not complete yet for this exact pass:
  - the user-profile database still reports the older `scanner-v14` fingerprint
  - a headless app launch did not trigger the fresh live-library rebuild path
  - because of that, the stored live `Unknown` count is still unchanged at `10`
  - the next session should trigger a deliberate in-app rescan path and then re-measure the remaining Build/Buy object cluster

## Current session note (March 16, 2026)

This session stayed in feature freeze and finished the next deep live-data trust pass on the remaining low-confidence and no-category rows.

Important changes and findings:

- a real file-inspection bug was found and fixed:
  - package context words were being normalized too early, which merged words together and hid obvious support clues like `Strings`, `Thai`, and `LotPrices`
- package inspection is now a little stronger in three careful ways:
  - package path and filename context words are split correctly
  - support and translation packages can still classify when they contain a small helper-resource mix instead of only pure `StringTable`
  - add-on, module, integration, and dense lot-price packages can promote to `Gameplay` when their inside-file resource signals already lean that way
- rebuild versions were bumped again so the real app had to refresh stored meaning:
  - `scanner-v14`
  - `downloads-assessment-v8`
- real live validation on the app data showed:
  - full `Library` rebuild completed with `sessionId 37`, `filesScanned 13010`, `reusedFiles 0`, and `updatedFiles 13010`
  - full-scan review work improved from `80` to `50`
  - open review rows are now:
    - `unsafe_script_depth 20`
    - `low_confidence_parse 18`
    - `no_category_detected 10`
    - `conflicting_category_signals 2`
  - true `Unknown` installed rows are now down to `10`
  - the live Inbox refresh still completed cleanly at `6 ready / 1 review`
- the remaining weak rows are now concentrated instead of scattered:
  - `4` CountryCrafter texture packages
  - `2` Bistro Expanded barback packages
  - `1` unreadable fireplace package with no parsed DBPF format
  - `3` isolated gameplay-style edge cases
- checks passed again:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `197` tests
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `npm run build`
  - `npm run tauri:build -- --debug`
  - the native desktop smoke

Important remaining gap:

- the trust pass is much closer now, but it is still not done:
  - the remaining unknown bucket is now mostly a small Build/Buy object-and-texture cluster
  - one unreadable package still needs a safer fallback story
  - watch bugs still need cleanup before feature work resumes

## Current session note (March 16, 2026)

This session stayed in feature freeze and finished the live creator-conflict audit against the real app database.

Important changes and findings:

- the earlier `1856` installed `conflicting_creator_signals` rows were mostly fake disagreements, not real creator uncertainty
- the scanner now handles creator signals more carefully:
  - path-based creator hints only count when they resolve to a known creator profile
  - a known folder creator can replace a weak unknown filename fallback
  - unknown folder names no longer create fake creator conflicts
  - inspection creator hints now look at the full hint list, so co-author cases do not false-flag when the current creator is already present
- rebuild versions were bumped again so the real app had to refresh stored creator meaning:
  - `scanner-v12`
  - `downloads-assessment-v6`
- real live validation on the app data showed:
  - full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, and `updatedFiles = 13010`
  - installed creator conflicts dropped from `1856` to `0`
  - download-side creator conflicts also settled at `0`
  - open review rows are now:
    - `low_confidence_parse 119`
    - `no_category_detected 81`
    - `unsafe_script_depth 20`
    - `conflicting_category_signals 7`
  - the live Inbox refresh completed and settled at `6 ready / 1 review`
- full checks passed again:
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `cargo build --manifest-path src-tauri/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml` with `191` tests
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features` with older warnings only
  - `npm run build`
  - the native desktop smoke

Important remaining gap:

- creator-conflict noise is no longer the main trust blocker, but the broader stabilization pass is still not done:
  - the next live audit target should be the remaining `low_confidence_parse` and `no_category_detected` rows
  - watch bugs still need cleanup before feature work resumes
  - the old Clippy warning backlog still exists and should be cleaned up separately from product-behavior stabilization

## Current session note (March 15, 2026)

This session used the live app database to do a deeper package-and-CC trust audit and fixed a real classification gap in the shared local foundation.

Important changes and findings:

- the weak-confidence problem was not mainly unreadable `.package` files
- many weak installed rows were already being parsed as real `dbpf-package` files with resource summaries, but the category inference was too narrow
- package inspection now recognizes more safe inside-file Sims patterns for:
  - Build/Buy surfaces
  - Build/Buy structures
  - stronger gameplay tuning clusters
- filename and subtype keyword coverage is now wider for common real-world terms such as:
  - walls, floors, tiles, foundation, stairs, fence, railing, spandrel
  - aspirations, careers, recipes, interactions, lot traits, lot challenges, cheats
- the scanner now:
  - promotes unknown rows to `Gameplay` when inspection truth is strong enough
  - raises confidence from inspection confidence floors
  - clears stale `no_category_detected` and `conflicting_category_signals` warnings when inspection confirms the final category
- rebuild versions were bumped again so the live app truly refreshed stored meaning:
  - `scanner-v10`
  - `downloads-assessment-v4`
- real live validation on the app data showed:
  - full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, and `updatedFiles = 13010`
  - installed low-confidence rows improved from:
    - `Unknown 304 -> 81`
    - `BuildBuy 104 -> 0`
    - `CAS 643 -> 1`
    - `Gameplay 7 -> 4`
  - installed review-item join count improved from about `3788` to `2082`
  - the biggest remaining installed review reason is now clearly `conflicting_creator_signals` at `1856`
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `187` tests
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all` passed
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- this was a real foundation fix, but the trust pass is still not done:
  - creator-conflict noise is now the next biggest target by far
  - more live validation is still needed on mixed real-world mods and CC
  - watch bugs still need cleanup before feature work resumes

## Current session note (March 15, 2026)

This session finished the stale ts4script clue rebuild path across both installed library data and Inbox download data.

Important changes and findings:

- the library scan cache now bumps when stored inspection meaning changes, so unchanged installed files get one true rebuild instead of silently keeping old clue rows
- the downloads assessment path now does the same kind of one-time rebuild for unchanged Inbox items when the assessment version changes
- real live-app validation was completed instead of relying on fixtures only:
  - a full live `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, and `updatedFiles = 13010`
  - the live Inbox refresh then rebuilt unchanged download items under the newer rules
  - Inbox state improved from `5 ready / 2 review` to `6 ready / 1 review`
- strict JSON-array checks on the live app database now show:
  - `0` bad `.pyc` / `_DO_NOT_UNZIP_` namespace values in stored ts4script clue fields for `mods`
  - `0` bad `.pyc` / `_DO_NOT_UNZIP_` namespace values in stored ts4script clue fields for `downloads`
  - `0` `_DO_NOT_UNZIP_` embedded-name marker values in those stored clue fields
  - `0` weak `mc` creator hints in those stored clue fields
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `181` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- this closes a real stale-clue trust problem, but the wider stabilization pass still is not done:
  - more live validation is still needed on generic mods and CC that do not have as many strong script clues
  - more watch bugs still need cleanup before feature work resumes
  - deeper long-run Inbox validation is still needed on messy real Downloads folders

## Current session note (March 15, 2026)

This session used the live app database to find and tighten a real ts4script clue-quality problem.

Important changes and findings:

- a read-only query against the live app database found at least `132` ts4script rows carrying filename-style namespace noise such as raw `.pyc` names or `_DO_NOT_UNZIP_`
- flat ts4script archives now keep that noise out of:
  - `script_namespaces`
  - `embedded_names`
  - short fallback creator hints
- this should make local matching less noisy for script mods that ship flat archive layouts
- added direct regression tests for:
  - flat script archives skipping filename noise in script clues
  - nested script archives still keeping the real namespace and creator path
- `cargo test --manifest-path src-tauri/Cargo.toml` passed with `181` tests
- `npm run build` passed
- the native desktop smoke passed again

Important remaining gap:

- this removes one real script-noise path, but the wider stabilization pass still is not done:
  - more live-library validation is still needed on mixed generic mods and CC
  - there are probably more noisy local clue patterns hiding in real libraries
  - watch bugs still need cleanup before feature work resumes

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

The highest-value next step is still stabilization of the shared matching base before more feature growth:

1. audit the remaining `low_confidence_parse` and `no_category_detected` rows with the same live-database method used for creator conflicts
2. do more messy real-world validation on generic mod and CC matching, especially the unresolved `.package` rows
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
