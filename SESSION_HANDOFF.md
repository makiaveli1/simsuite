# Session Handoff

## Current Session (March 19, 2026 - Early Night)

- **Mode**: code
- **Focus**: full `Home` rework using the new calm-home philosophy

### Progress Made

1. **Locked the design context for the deeper page-by-page redesign**:
   - added `.impeccable.md` at the repo root
   - wrote the `Home` redesign spec in:
     - `docs/superpowers/specs/2026-03-19-home-redesign-design.md`
   - the main rules now stay explicit:
     - cosy
     - casual
     - non-confusing
     - flat desktop feel
     - personalization through theme and module choice instead of wallpapers

2. **Rebuilt `Home` away from the old command-board layout**:
   - removed the permanent right inspector
   - replaced the old workbench-style `Home` with a calmer centered landing surface
   - added one large hero area that changes its message based on:
     - library health
     - watched-page state
     - folder setup
   - kept only two fixed actions in view:
     - `Customize Home`
     - `Scan`
   - moved the rest of the page toward glanceable modules instead of navigation actions

3. **Added real details-on-demand personalization**:
   - `Customize Home` now opens as a right-side sheet
   - the sheet can change:
     - hero focus
     - information level
     - visible modules
     - theme
     - spacing density
     - ambient hero motion
   - these preferences save per user view so `Casual`, `Seasoned`, and `Creator` can each feel a little different

4. **Made the three user views actually behave differently on `Home`**:
   - `Casual` now defaults to the quietest version
   - `Seasoned` keeps a balanced middle state
   - `Creator` shows the fullest set of `Home` modules and richer detail rows
   - all three views keep the same page structure, but the amount of information shifts in a much more intentional way

5. **Verified the rebuild visually instead of trusting the code**:
   - restarted Vite at `http://127.0.0.1:1420/`
   - checked `Home` in:
     - `Casual`
     - `Seasoned`
     - `Creator`
   - also checked the `Customize Home` side sheet
   - saved fresh screenshots:
     - `output/playwright/home-pass10-casual-after.png`
     - `output/playwright/home-pass10-seasoned-verified.png`
     - `output/playwright/home-pass10-creator-after.png`
     - `output/playwright/home-pass10-customize-sheet.png`

6. **Verification**:
   - `npm run build` passed
   - checked the screen frame in the live app and confirmed the new `Home` did not overflow vertically in the checked desktop view

### What Worked

- `Home` now feels much closer to a calm browser-style landing page than a mini dashboard
- removing the always-open inspector was the right call
- the hero panel gives the page a clear center of gravity without making it feel loud
- `Casual` especially feels easier to read now
- the side-sheet customization pattern feels much more appropriate than leaving lots of controls on the page itself

### Known Problems / Gaps

- the `Customize Home` sheet is useful and works, but it is still a little long; it scrolls cleanly, though a later taste pass could tighten the theme area
- `Creator` view is intentionally denser, but it is also the closest to feeling busy again; if we want to push the calm-home idea even further later, that is the first variant to tune
- this session only reworked `Home`; the same deeper page-by-page rethink still needs to move through the other screens one at a time

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- If continuing the deeper redesign program, keep the same flow:
  - one screen per pass
  - design first
  - live screenshots
  - real cross-view checks
- Best next screen candidates:
  - `Downloads`
  - `Updates`
  - or `Library`

## Current Session (March 18, 2026 - Evening)

- **Mode**: code
- **Focus**: on-demand detail pass for `Settings` and `Updates`

### Progress Made

1. **Turned `Settings` into a real preferences window instead of one long pile of panels**:
   - added a left preferences list so only one settings group stays open at a time
   - moved the “saved here” summary into the left side so the right side can focus on the current choice
   - split the longer background/update options into a calmer two-block detail view

2. **Moved update-source editing toward details-on-demand**:
   - removed the always-open source form from the `Updates` inspector
   - added a reusable right-side sheet for source editing
   - kept the inspector shorter so status, proof, and the next action stay readable
   - moved “clear source” into the sheet instead of leaving it on screen all the time

3. **Added the shared styling for the new patterns**:
   - new workbench side-sheet chrome
   - calmer settings section buttons
   - focused settings detail panel layout
   - responsive fallback for the two-column settings detail block

4. **Checked the live app again instead of trusting the code**:
   - restarted Vite at `http://127.0.0.1:1420/`
   - visually checked:
     - `Settings` in `Creator`
     - `Settings` in `Casual`
     - `Updates` in `Creator`
     - `Updates` in `Casual`
   - saved fresh screenshots:
     - `output/playwright/pass8-settings-after.png`
     - `output/playwright/pass8-updates-after.png`

5. **Verification**:
   - `npm run build` passed twice during the pass

### What Worked

- `Settings` feels much more like a desktop preferences window now
- opening one settings section at a time cuts a lot of visual noise
- `Updates` has a cleaner inspector because editing is no longer welded into it
- the new side-sheet pattern is ready to reuse on other screens that still show too much at once

### Known Problems / Gaps

- the live fixture data only had one built-in tracked update source and zero setup items, so the new `Updates` side sheet could be verified in code and build output, but not fully opened in a real live state during this pass
- `Settings` is much calmer now, but short sections like `Experience` still leave a lot of quiet canvas below the main panel; that is cleaner than clutter, but it could still take one more taste pass later

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- If continuing the on-demand-detail pass, start with:
  - `Updates` live verification once there is editable or setup fixture data
  - another taste pass on `Settings` short-section balance
  - deciding which screen should get the next side sheet:
    - `Home`
    - `Downloads`
    - or `Library`

## Current Session (March 18, 2026 - Afternoon)

- **Mode**: code
- **Focus**: full-screen visual consistency pass across every desktop workspace

### Progress Made

1. **Quieted the app shell so the workspaces lead again**:
   - calmed the left navigation rail so it stops reading like a stack of equal-weight boxes
   - reduced the always-boxed feel on nav items and chips
   - removed the jumpy active/hover feel from rail icons

2. **Fixed the screen shell so more screens actually use the available height**:
   - changed the shared workbench page shell so the final workspace section can fill the remaining canvas
   - this helped `Review`, `Creators`, `Types`, and `Duplicates` stop leaving such large dead dark areas below the real work
   - added a stronger but still subtle surface treatment so empty space feels more intentional instead of unfinished

3. **Made the work surfaces calmer and more desktop-like**:
   - softened panel chrome and row backgrounds
   - made footer cards in `Updates` and `Review` stretch more naturally
   - gave the audit screens a real working height so the middle stage feels like a tool, not a short stack floating in space
   - tightened `Home`'s right inspector by removing the repeated action description

4. **Rechecked the whole app visually in all three experience modes**:
   - `Seasoned`
   - `Casual`
   - `Creator`
   - fresh live screenshots saved in:
     - `output/playwright/pass7-after/`
   - contact sheets saved for quick review:
     - `output/playwright/pass7-after/seasoned-sheet.png`
     - `output/playwright/pass7-after/casual-sheet.png`
     - `output/playwright/pass7-after/creator-sheet.png`

5. **Verification**:
   - `npm run build` passed

### What Worked

- the app finally looks more like one desktop product again instead of several similar dark dashboards
- the quieter rail helps the center stage stand out much more clearly
- `Review`, `Creators`, `Types`, and `Duplicates` now use vertical space better and feel less hollow
- `Home` still keeps the next action clear, but the right side is less repetitive now
- the calmer panel treatment holds up in `Casual`, `Seasoned`, and `Creator`

### Known Problems / Gaps

- `Updates` still has some quiet lower-canvas space in very short tracked lists; it is better, but a future pass could turn that area into a richer focus state
- `Home` is improved, but the lower-right stage area can still feel a bit too calm when the folder card content is short
- `Settings` is cleaner and more consistent now, but it is still one of the denser screens in the app

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- If doing one more polish pass, focus on:
  - `Updates` short-list focus state
  - `Home` lower-right balance
  - `Settings` density and grouping
- If the user wants a stronger step beyond this, move into more on-demand detail patterns:
  - side sheets for editing
  - compact inspectors by default
  - fuller focus states when one item is selected
- The latest checked screenshots are in:
  - `output/playwright/pass7-after/`

## Current Session (March 18, 2026 - Midday)

- **Mode**: code
- **Focus**: cross-view screenshot pass on `Home`, `Updates`, and `Review`

### Progress Made

1. **Rebuilt `Home` into a fuller desktop command board**:
   - replaced the old top-heavy layout with a real two-row workspace
   - added a stronger primary action card inside the main stage
   - added a dedicated tracked-pages panel so update work no longer hides inside the right inspector
   - replaced the placeholder-feeling right side with a clearer command-board inspector

2. **Reshaped `Updates` so the middle works like a real work surface**:
   - moved the selected-file story into a full-width top focus band
   - kept the list as the main stage instead of squeezing it beside another tall panel
   - added a calmer footer row for mode totals and lane guidance
   - simplified the list columns so tracked, setup, and review rows fit without the awkward horizontal squeeze

3. **Made `Review` use its center space better across views**:
   - kept the left rail and right inspector
   - added a lower center-stage pair of cards:
     - queue focus
     - best next fix lane
   - this gives tiny queues a clearer follow-up story instead of leaving one big empty center gap

4. **Checked the three experience modes on the live app instead of guessing**:
   - `Casual`
   - `Seasoned`
   - `Creator`
   - fresh screenshots saved locally for the main pass:
     - `tmp-ui-pass-6-home-casual-after.png`
     - `tmp-ui-pass-6-home-seasoned-after.png`
     - `tmp-ui-pass-6-home-creator-after.png`
     - `tmp-ui-pass-6-updates-casual-after.png`
     - `tmp-ui-pass-6-updates-seasoned-after.png`
     - `tmp-ui-pass-6-updates-creator-after.png`
     - `tmp-ui-pass-6-review-casual-after.png`
   - extra manual live checks were also saved where the fast automated capture was too easy to fool during screen transitions:
     - `tmp-ui-pass-6-review-seasoned-live.png`
     - `tmp-ui-pass-6-review-creator-live.png`
     - `tmp-ui-pass-6-updates-casual-setup-live.png`

5. **Verification**:
   - `npm run build` passed

### What Worked

- `Home` now feels much more like a proper desktop command board instead of a short stack of cards sitting at the top of a large empty page
- `Updates` is easier to read because the table gets the width back and the supporting explanation lives below it instead of fighting beside it
- `Review` now gives a clearer “what now?” answer in the center instead of making the user bounce between the queue and the far-right inspector
- the three experience modes are still using the same screen structure now, but the copy and detail level still shift the right way

### Known Problems / Gaps

- `Home` is much better, but the lower-right stage corner can still feel a little quiet when folder cards are short
- `Updates` tracked mode is cleaner now, but one-row cases can still take one more taste pass later if we want it even tighter
- `Review` is meaningfully better, but tiny queues still leave some calm empty canvas in the center stage because there are only a few rows to show

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Do one final visual consistency sweep across the cleaned desktop screens in:
  - `Casual`
  - `Seasoned`
  - `Creator`
- If there is time for one more polish round, focus on:
  - `Home` lower-stage quiet space
  - `Updates` single-row tracked density
  - `Review` tiny-queue center balance

## Current Session (March 18, 2026 - Morning)

- **Mode**: code
- **Focus**: screenshot-driven cleanup pass on `Creators`, `Duplicates`, and `Organize`

### Progress Made

1. **Calmed `Creators` so the work starts sooner**:
   - removed the repeated full-width three-step strip
   - kept the useful summary counts
   - added one small guidance note inside the left rail
   - the screen now gets to the real creator groups, sample files, and save panel much faster

2. **Rebuilt `Duplicates` into a real compare workspace**:
   - moved counts, filters, and layout presets into a dedicated left rail
   - added a proper center focus stage that shows the selected pair side by side
   - kept the deeper proof in the right inspector
   - fixed a layout issue in the lower queue so rows sit directly under the queue heading instead of drifting downward

3. **Quieted `Organize` without changing its core flow**:
   - added a short “safe path first” note in the left rail
   - tightened the summary, recommendation, preset, and issue panels
   - made the left rail scroll on its own so the page stays more desktop-like and the center preview keeps the spotlight

4. **Ran fresh live visual checks after the changes**:
   - `Creators`
   - `Duplicates`
   - `Organize`
   - screenshot files saved locally:
     - `tmp-ui-pass-5-creators-before.png`
     - `tmp-ui-pass-5-creators-after.png`
     - `tmp-ui-pass-5-duplicates-before.png`
     - `tmp-ui-pass-5-duplicates-after-4.png`
     - `tmp-ui-pass-5-organize-before.png`
     - `tmp-ui-pass-5-organize-after.png`

5. **Verification**:
   - `npm run build` passed multiple times during the pass

### What Worked

- `Creators` now feels more like `Types`: one short explanation, then straight into the work
- `Duplicates` finally reads like a desktop comparison tool instead of one long list with controls stacked on top
- `Organize` still keeps all of its safety detail, but the left side feels less overbearing and the preview gets to lead more clearly

### Known Problems / Gaps

- `Duplicates` is much stronger now, but the lower queue still leaves some quiet space when there are only a few pairs
- `Organize` is better, but the left rail could still use one more taste pass if we want it even calmer
- `Home` still needs its placeholder right side replaced
- the remaining big visual consistency targets are still:
  - `Review` small-queue density
  - `Updates` tiny-list empty space
  - `Home` right inspector

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Start with a screenshot-driven polish pass on:
  - `Home`
  - `Review`
  - `Updates`
- If those feel stable, do one lighter consistency sweep across all cleaned screens and trim any last “too many boxes” spots.

## Current Session (March 18, 2026 - Near Dawn)

- **Mode**: code
- **Focus**: screenshot-driven cleanup pass on `Library` and `Types`

### Progress Made

1. **Fixed the broken feel in `Library` by reshaping the rail and center stage**:
   - the filter rail is now a real narrow rail instead of stretching across the screen
   - the center stage now has:
     - a compact count strip
     - a selected-file focus block
     - a direct handoff button into `Updates`
   - this stopped the file table from being squeezed into the far right edge

2. **Made the `Library` rail feel like part of the tool instead of a raw form block**:
   - added a short browse note
   - added small “shown now” and “filters on” stats
   - stacked the filters vertically so the rail stays calm and narrow
   - added a reset action and quick counts for creators and type groups

3. **Calmed `Types` by removing the repeated top tutorial band**:
   - removed the three-step strip that was repeating information already shown inside the three columns
   - kept the stats row, but tightened it a bit
   - added one quieter note inside the left panel so the screen still explains itself once

4. **Ran fresh live visual checks after the changes**:
   - `Library`
   - `Types`
   - screenshot files saved locally:
     - `tmp-ui-pass-4-library-before.png`
     - `tmp-ui-pass-4-library-after.png`
     - `tmp-ui-pass-4-types-before.png`
     - `tmp-ui-pass-4-types-after.png`

5. **Verification**:
   - `npm run build` passed

### What Worked

- `Library` finally reads like a desktop browser:
  - left rail for narrowing
  - center stage for scanning the file list
  - right inspector for deeper details
- the biggest visual problem on `Library` was not just styling; the rail was effectively allowed to sprawl, which squeezed the main work area
- `Types` feels faster now because the work starts sooner and the explanation lives closer to the place where you act

### Known Problems / Gaps

- `Library` is much better, but very short result sets can still leave a lot of quiet space in the lower half of the screen
- `Types` is calmer now, but the inspector is still a little busy compared with the newer `Downloads` and `Updates` work
- `Creators`, `Duplicates`, and `Organize` should still get another visual consistency pass after these improvements

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Do one more screenshot-driven taste pass on:
  - `Library` lower-stage empty-space handling
  - `Types` inspector density
- Then continue with:
  - `Creators`
  - `Duplicates`
  - `Organize`

## Current Session (March 18, 2026 - Late Night)

- **Mode**: code
- **Focus**: screenshot-driven cleanup across the remaining desktop workbench screens

### Progress Made

1. **Fixed a shared layout bug that was distorting several screens**:
   - `Review`, `Organize`, `Duplicates`, `Creator Audit`, and `Category Audit` were all using the shared `.workbench` class directly on the page shell
   - that class was built for the split-pane `Workbench` component and quietly forced a three-column page grid
   - the result was the strange dead space and off-balance composition we kept seeing in screenshots
   - those screens now also use a new `workbench-screen` page-shell override so they keep the calmer workbench styling without inheriting the wrong outer grid

2. **Rebuilt `Review` into a fuller desktop workbench**:
   - added a real header row instead of the old top button strip
   - added a left rail with:
     - queue health
     - top reason groups
     - direct jumps into creator/type cleanup and organize
   - moved the queue into a proper center stage with layout toggles above it
   - kept the right inspector for selected detail

3. **Filled the middle of `Updates` so it stops feeling hollow**:
   - kept the left controls and right inspector
   - added a center-side spotlight panel beside the tracked/setup/review table
   - the spotlight now shows:
     - current selected-file focus
     - mode-specific counts
     - a short explanation of how to read that lane
   - this makes `Updates` feel more like a working desktop tool and less like one row floating in a large empty panel

4. **Ran fresh live visual checks after the changes**:
   - `Review`
   - `Updates`
   - `Organize`
   - `Creator Audit`
   - `Duplicates`
   - screenshot files saved locally:
     - `tmp-ui-pass-3-review.png`
     - `tmp-ui-pass-3-updates.png`
     - `tmp-ui-pass-3-organize.png`

5. **Verification**:
   - `npm run build` passed

### What Worked

- The weird left-side dead zones were not individual screen design failures after all; they were one shared page-shell bug.
- `Organize`, `Creator Audit`, `Duplicates`, and `Review` now read much more like proper desktop workspaces because their panels finally sit in normal rows and columns again.
- `Updates` feels meaningfully better because the center area now explains the selected file and the lane at the same time.

### Known Problems / Gaps

- `Updates` is much better, but when there is only one tracked row the lower part of the table area still feels visually quiet.
- `Review` is cleaner and more tool-like now, but the center stage can still feel a little empty when the queue is tiny.
- `Library` did not get a fresh visual pass this session.
- `Types` was not re-screenshotted after the shared workbench-shell fix, even though the same bug fix should help it too.

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Continue the screenshot-driven pass on:
  - `Library`
  - `Types`
  - `Downloads` quick sanity check after the newer shared cleanup
- If those look stable, do one more taste pass on:
  - `Updates` small-list empty-space handling
  - `Review` tiny-queue composition

## Current Session (March 18, 2026 - Evening)

- **Mode**: code
- **Focus**: visual cleanup pass on the new `Downloads` workbench

### Progress Made

1. **Calmed the `Downloads` rail**:
   - narrowed the rail a bit
   - changed the filter stack into a cleaner vertical form
   - turned the secondary rail actions into calmer action rows instead of more heavy buttons
   - flattened the lane summary cards so they read like a quick status list instead of a second dashboard

2. **Reduced chrome in the center stage**:
   - replaced the boxed stage stats with compact status chips
   - removed the extra corner accents and heavy shadow feel from the `Downloads` workbench panels
   - softened queue rows so the selected batch stands out without every row shouting

3. **Quieted the right inspector**:
   - compressed the signal cards into lighter inline callouts
   - made the main next-step card the clear focus instead of one more equal-weight box
   - flattened the dock sections so the inspector reads more like one tool panel and less like a stack of separate widgets

4. **Verification**:
   - `npm run build` passed

### What Worked

- The screen should now feel less like a wall of cards and more like one connected desktop workspace.
- The selected batch should read as the main story faster because the surrounding chrome is quieter.
- The rail should now support the work instead of competing with it.

### Known Problems / Gaps

- No fresh real desktop click-through or screenshot signoff was run after this visual pass.
- `Downloads` may still want one more polish pass after a real visual check, especially around spacing and how the preview feels in guided/review cases.
- `Home` still has a placeholder right inspector.
- `Review`, `Duplicates`, `Creator Audit`, and `Category Audit` still need the same cleanup treatment.

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Do a real desktop check of `Downloads` in a few real cases:
  - ready batch
  - waiting batch
  - guided setup batch
  - blocked or review batch
- If it looks solid, move to:
  - `Home` right inspector
  - `Review` or `Duplicates` workbench cleanup

## Current Session (March 18, 2026 - Late Afternoon)

- **Mode**: code
- **Focus**: second desktop-workbench UI implementation slice

### Progress Made

1. **Rebuilt `Downloads` around the same desktop workbench shape**:
   - moved the stacked watcher + filter card block out of the top of the page
   - `Downloads` now uses:
     - left control rail
     - center stage
     - right inspector
   - the main stage now keeps the queue and preview together as the working surface instead of pushing them down below setup cards

2. **Made the `Downloads` rail do the quiet setup work instead of the page header**:
   - watcher path, status, filters, rule set, lane counts, and quick actions now live in the left rail
   - the center area now starts with a small status line and then goes straight into queue + preview work
   - this reduces page-level clutter and keeps the main workspace focused on the currently selected batch

3. **Tightened shared workbench shell spacing**:
   - `WorkbenchRail` and `WorkbenchInspector` were wrapping content in helper containers that had no real base styling
   - shared padding and layout rules now exist for those containers, so pane-based screens behave more consistently

4. **Verification**:
   - `npm run build` passed

### What Worked

- `Downloads` now fits the same desktop-app pattern as `Updates`, which makes the app feel more intentional already.
- The queue and preview are easier to read because they are no longer being pushed downward by a wide setup block.
- The right inspector still keeps the detailed action flow, so the redesign stayed structural without disturbing the safety logic.

### Known Problems / Gaps

- No fresh real desktop click-through or screenshot signoff was run after the `Downloads` restructure.
- `Home` still has a placeholder right inspector.
- `Review`, `Duplicates`, `Creator Audit`, and `Category Audit` still need the same desktop cleanup pass.
- The shared rail/inspector padding change needs a quick visual sanity check on `Updates` and `Library` even though the build is clean.

### Important Decisions

- `Downloads` should follow the same workbench model as `Updates`:
  - narrow control rail
  - central work surface
  - right inspector
- Workspace-level setup and filters belong in the rail, not stacked above the main working surface.
- Shared workbench container padding should come from the base layout primitives so each screen does not have to fake it separately.

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Do one real desktop UI pass for:
  - `Downloads`
  - `Updates`
  - `Library`
- If the layout feels stable, move to the next cleanup slice:
  - give `Home` a real right inspector
  - convert `Review` or `Duplicates` to the same workbench pattern
  - remove any leftover stacked dashboard bands from the remaining dense screens

## Current Session (March 18, 2026 - Afternoon)

- **Mode**: code
- **Focus**: first desktop-workbench UI implementation slice

### Progress Made

1. **Promoted `Updates` into a real workspace**:
   - rebuilt `Updates` around the new desktop workbench pattern:
     - left control rail
     - center work table
     - right inspector
   - `Updates` now owns the three watch jobs directly:
     - tracked items
     - setup items
     - review items
   - the selected file can now be opened straight into `Updates` from `Library`

2. **Cut the leftover watch-center clutter out of `Library`**:
   - removed the old hidden watch-center state and dead helper code from `LibraryScreen.tsx`
   - `Library` is back to being a browse-and-inspect screen
   - the inspector now gives a small handoff into `Updates` instead of trying to host watch management itself

3. **Fixed shared workbench shell problems**:
   - `Workbench`, `WorkbenchRail`, and `WorkbenchInspector` were building broken class names because the class strings were being joined incorrectly
   - that is now fixed, so the shared desktop layout styles can apply consistently

4. **App shell wiring now treats `Updates` like a first-class workspace**:
   - `updates` is now included in workspace version invalidation
   - Home actions route straight into the right `Updates` mode
   - update-related changes can now refresh `Home` and `Updates` together

5. **Verification**:
   - `npm run build` passed

### What Worked

- The current redesign direction is much clearer now:
  - `Home` is the command board
  - `Library` is the file browser
  - `Updates` is the watch and update workspace
- The new `Updates` screen now feels like a real desktop pane-based tool instead of a stacked utility page.
- The `Library` file got much smaller and easier to reason about once the old watch-center leftovers were removed.

### Known Problems / Gaps

- This session only covered the first big UI slice:
  - `Updates` is now in much better shape
  - `Library` is cleaner
  - but other dense screens still need the same workbench cleanup
- No real desktop click-through or fresh screenshot pass was run yet after this redesign slice.
- `Home` still has a placeholder right inspector.
- The new `Updates` workspace still needs a second polish pass for:
  - stronger mode counts
  - a little more visual refinement
  - possible batch setup helpers later

### Important Decisions

- `Updates` is now the real owner of tracked/setup/review watch work.
- `Library` should not host the old watch center anymore.
- Shared workbench components should stay the base for new dense desktop screens instead of each screen inventing its own shell.

### Next Session Start Here

- Read this file first.
- Then read `docs/IMPLEMENTATION_STATUS.md`.
- Do one real desktop UI check for:
  - `Home` -> `Updates`
  - `Library` -> selected file -> `Open in Updates`
  - save source
  - clear source
  - check selected source
- Then move to the next redesign slice:
  - tighten `Downloads` into the same workbench pattern
  - reduce stacked summary clutter on the other dense screens
  - give `Home` a proper right inspector instead of the placeholder

## Current Session (March 17, 2026 - Evening)

- **Mode**: code
- **Focus**: Desktop-First Workbench Redesign implementation

### Progress Made

1. **Fixed TypeScript errors in LibraryScreen.tsx**:
   - Added missing state variables for legacy watch center functionality: `setQueuedWatchCenterAction`, `setPendingWatchIntent`, `setWatchCenterMessage`
   - These are now placeholder setters since watch functionality has moved to Updates screen

2. **Fixed import errors in HomeScreen.tsx**:
   - Changed from barrel import `'../components/layout'` to individual imports
   - Now imports Workbench, WorkbenchStage, WorkbenchInspector separately

3. **Build verification**:
   - `npm run build` now succeeds
   - `npm run tauri dev` starts successfully

### Key Changes in Progress

- HomeScreen and LibraryScreen now use Workbench layout components
- Updates navigation link from Home now points to 'updates' instead of 'library'
- Watch-related functionality references in Library are now legacy placeholders

---

## Current Priority

- March 16, 2026: a research-only product audit was done before any more player-facing `Library` detail work:
  - important honesty call:
    - SimSuite is **not** ready to claim 100% accuracy across every simmer's library
    - the right production standard is:
      - confirmed facts as facts
      - strong clues marked carefully
      - true unknowns left unknown
  - outside research was checked against:
    - the linked `r/thesimscc` organizer discussion
    - broken-CC and missing-mesh threads on `r/sims4cc`
    - Scarlet's mod list help pages
    - TS4 Mod Hound
    - Sims 4 Mod Manager / Overwolf feature pages
    - SimSweep feature notes
  - the strongest repeated player needs were:
    - visual preview / thumbnail
    - creator name
    - one clear "what is this?" label
    - broken / outdated / unknown / duplicate / conflict status
    - missing mesh or missing requirement clues
    - quick path to the file so it can be removed
    - "what sim or lot is using this?" style usage tracing
    - update tracking and notes
  - current product direction from this audit:
    - beginner and seasoned views should not grow into debug panels again
    - `type + subtype + file format` should likely collapse into one stronger player-facing summary line
    - the next valuable simmer-facing fields are probably:
      - preview
      - status
      - needs / dependency / missing-mesh hints
      - used-by tracing later through tray analysis
      - source / creator / open-folder actions
  - next best step after the current confidence freeze:
    - decide how to handle `unsafe_script_depth`
    - then do the postponed watch bug sweep
    - after that, do a focused player-facing info pass guided by this audit instead of adding raw parser detail
- March 16, 2026: the next deep trust pass stayed in feature freeze and tightened the last real category edge cases instead of adding anything new:
  - two narrow package-inspection fixes were added:
    - `uicheats`-style helper packages can now use one more context-only gameplay resource clue
    - lean CAS appearance packages can now use a narrow inside-file CAS fallback when they look like simple appearance/default-replacement content instead of mixed gameplay or Build/Buy packages
  - one narrow filename-confidence cleanup also stayed in place for common real-world patterns that were already clearly named:
    - override/default replacement packages
    - pose packs
    - childbirth / pregnancy packages
  - this keeps those files out of low-confidence review when the filename evidence is already strong enough to trust
  - important guardrail:
    - the new gameplay clue is still context-only
    - a file like `Colorful_Var_Pink.package` does **not** get forced into a category just because it contains one weak resource type
  - regression coverage was added for all three cases:
    - the `UI Cheats` helper path now promotes safely
    - lean CAS appearance packages now classify safely
    - a single ambiguous resource with no other context still stays unknown
  - rebuild versions moved forward again because stored meaning changed:
    - library scan cache -> `scanner-v17`
    - downloads assessment -> `downloads-assessment-v11`
  - real live verification now passed on the user profile again, not just fixtures:
    - the real user-profile database now reports `scanner-v17`
    - a real full startup-triggered scan completed as `sessionId = 41`
    - `filesScanned = 13010`
    - scan duration was about 9 minutes
    - open installed review rows are now:
      - `unsafe_script_depth = 20`
      - `low_confidence_parse = 1`
      - `no_category_detected = 1`
      - `conflicting_category_signals = 0`
    - true installed `Unknown` rows are now down to `1`
    - the last honest unknown is:
      - `Colorful_Var_Pink.package`
      - it still only exposes one weak compressed package resource and no safe creator, version, or in-game-name clues
      - it should stay unknown until we can prove more from real inside-file evidence
  - important truth about the remaining review lane:
    - the last `20` `unsafe_script_depth` rows are not parser mistakes
    - they are real deep-installed script mods and should stay flagged unless we explicitly redesign that rule
  - checks passed:
    - `cargo check --manifest-path src-tauri/Cargo.toml`
    - `cargo build --manifest-path src-tauri/Cargo.toml`
    - `cargo test --manifest-path src-tauri/Cargo.toml` with `209` tests
    - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
    - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
    - `npm run build`
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`
    - real desktop launch against the live profile, with a completed `scanner-v17` full scan
  - next best step:
    - keep the feature freeze
    - decide whether `unsafe_script_depth` should stay as a hard review flag or move to a calmer but still visible warning
    - then move into the postponed watch-system bug sweep on top of this stronger local-truth base
- March 16, 2026: the stale live-rescan gap is now closed with real app proof:
  - `Home` now gets one simple truth flag from the backend:
    - whether the stored library facts were built with the current scan rules or not
  - the app shell now uses that flag to start one automatic library refresh per app session when all of these are true:
    - game folders are configured
    - no scan is already running
    - the stored scan fingerprint is stale
  - `Home` now shows that state in calmer player language instead of hiding it:
    - `Library check` / `Library facts`
    - a single compact refresh banner when the stored library facts are stale
  - this fixes the earlier trust hole where the app could quietly show old library facts as if they were current after a scan-rule change
  - regression coverage was added for the stale-scan helper:
    - a brand-new empty database does not nag for a refresh
    - existing indexed data with an older fingerprint is marked stale
  - real live verification now passed, not just fixture smoke:
    - the real user-profile database now reports `scanner-v15`
    - a real full startup-triggered scan completed as `sessionId = 39`
    - `filesScanned = 13010`
    - scan duration was about 9 minutes
    - true installed `Unknown` rows are now down to `3`
    - open review rows are now:
      - `unsafe_script_depth = 20`
      - `low_confidence_parse = 11`
      - `no_category_detected = 3`
      - `conflicting_category_signals = 2`
  - checks passed:
    - `cargo test --manifest-path src-tauri/Cargo.toml` with `200` tests
    - `npm run build`
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`
    - real desktop launch against the live profile, with a completed `scanner-v15` full scan
  - next best step:
    - stay in feature freeze
    - target the remaining `Unknown`, `low_confidence_parse`, and watch-bug edge cases with the same live-data method
- March 16, 2026: the next stabilization pass stayed on `Library` polish and tightened the player-facing detail rules:
  - beginner and seasoned `Library` inspector views were simplified on purpose
  - the goal of this pass was:
    - show regular simmers only the information they actually need
    - stop surfacing heuristic labels that can sound more certain than they are
    - keep the heavier receipts and correction tools in creator mode only
  - beginner and seasoned views now show only calmer, player-facing sections:
    - creator
    - type
    - subtype when it really exists
    - file format
    - filtered in-game names when they look human-readable
    - installed version and update state
    - grouped-file and safety notes only when they matter
  - beginner and seasoned views no longer surface the heavy debug-feeling sections by default:
    - no inside-file evidence section
    - no creator-learning tool
    - no type-override tool
    - no raw path panel
    - no local version evidence / watch evidence dumps
    - no heuristic summary tags like add-on, core helper, or texture recolor
  - creator mode still keeps the deeper proof and correction tools
  - important trust rule change:
    - regular simmer views now only surface direct facts or carefully filtered exact file clues
    - if an installed version is not strong enough to trust, the UI now says it is not confirmed yet instead of presenting the value like a real confirmed version
  - checks passed:
    - `cargo test --manifest-path src-tauri/Cargo.toml` with `198` tests
    - `npm run build`
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`
  - outside research this pass reinforced the same user needs:
    - creator name
    - update state
    - easy broken-CC identification
    - clear file type
    - thumbnails or in-game names when available
  - next best step:
    - do the real live in-app rescan trigger cleanup so `scanner-v15` actually refreshes the user-profile database
    - then keep tightening the last true unknown files and watch bugs
- March 16, 2026: the next stabilization pass focused on two things at once:
  - a few last obvious Build/Buy filenames
  - making `Library` show file details in a way simmers actually care about
  - the `Library` inspector now has a new plain-English summary layer:
    - `At a glance` / `Simmer summary` explains what a file seems to be, not just how the parser saw it
    - it now surfaces:
      - a plain-English file summary such as Build/Buy object, texture/recolor package, add-on/module, or text-support package
      - file format
      - the best version clue
      - useful tags
      - in-game names
      - related family hints
      - friendly version evidence lines
    - the older technical inspection section is still there, but some labels are now clearer:
      - `Creator hints` -> `Creator names found`
      - `Version hints` -> `Version numbers found`
      - `Resources` -> `Package contents`
      - `Namespaces` -> `Script folders`
      - `Embedded names` -> `In-game names`
  - filename keyword coverage was widened again for a small safe Build/Buy cluster:
    - `entryway`
    - `entrance`
    - `barback`
    - `fireplace`
  - subtype mapping now treats those names more usefully:
    - entryway / entrance -> `Build Surfaces`
    - barback / fireplace -> `Furniture`
  - rebuild versions were bumped again so old stored meaning cannot linger:
    - library scan cache -> `scanner-v15`
    - downloads assessment -> `downloads-assessment-v9`
  - checks passed:
    - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
    - `cargo test --manifest-path src-tauri/Cargo.toml` with `198` tests
    - `npm run build`
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`
  - important open gap:
    - the real app database still shows the old `scanner-v14` fingerprint
    - a headless launch of the built app did not trigger a live-library rebuild against the user profile
    - because of that, the real stored `Unknown` count is still sitting at the old `10` rows until a deliberate in-app rescan path is triggered
    - this needs to be treated as unfinished verification, not as proof that the classifier change failed
  - outside research check for simmer-facing details matched the product instinct:
    - simmers mostly care about what a file is, who made it, whether it is an add-on or standalone file, whether it has version/update clues, and whether it is likely support text / recolor / helper content
  - the next best step should stay in stabilization:
    - trigger a real in-app rescan against the live profile and confirm the small Build/Buy cluster actually drops
    - then keep tightening the remaining true unknowns and watch bugs before any new feature work
- March 16, 2026: the live low-confidence and no-category stabilization pass closed another big chunk of noisy review work with real app proof:
  - a real bug was hiding obvious package context words:
    - package path token parsing was normalizing names too early and smashing words together
    - that meant clear names like `..._Strings.package` were not being read as separate clues
  - package inspection is now a little better at safe context-based classification:
    - support and translation packages can now stay classifiable even when they include a small helper-resource mix instead of only pure `StringTable`
    - add-on, module, integration, and dense lot-price packages can now promote to `Gameplay` when their inside-file signals already lean that way
  - rebuild versions were bumped again so old stored meaning could not linger:
    - library scan cache -> `scanner-v14`
    - downloads assessment -> `downloads-assessment-v8`
  - real live desktop validation on the app database showed:
    - full `Library` rebuild completed cleanly with `sessionId = 37`, `filesScanned = 13010`, `reusedFiles = 0`, `updatedFiles = 13010`
    - full-scan review work dropped again from `80` to `50`
    - open review rows are now down to:
      - `unsafe_script_depth = 20`
      - `low_confidence_parse = 18`
      - `no_category_detected = 10`
      - `conflicting_category_signals = 2`
    - remaining true `Unknown` rows are down to `10`
    - live Inbox refresh still settled cleanly at `6 ready / 1 review`
  - the remaining weak bucket is much smaller and clearer now:
    - `4` CountryCrafter texture packages
    - `2` Bistro Expanded barback packages
    - `1` likely unreadable fireplace package with no parsed DBPF format
    - `3` isolated gameplay-ish edge cases
  - the next product focus should stay on stabilization:
    - target the remaining Build/Buy texture and object package cluster with the same live-data method
    - keep watch bug cleanup behind this trust pass
    - keep validating against the real app database, not just fixtures
- March 16, 2026: the creator-conflict stabilization pass closed the biggest remaining review-noise source with real live proof:
  - the earlier `1856` installed `conflicting_creator_signals` rows were mostly not real creator disagreements
  - the main root causes were:
    - weak filename fallback names like `ESTATE` or `SOHO` being treated like real creators
    - unknown nearby folder names like `Strings` or `Nightwork` being treated like creator truth
    - co-author script-mod hints being over-penalized when the current creator was already present in the full hint list
  - the scanner now handles creator signals more carefully:
    - nearby path hints only count when they resolve to a known creator profile
    - a known folder creator can replace a weak unknown filename fallback
    - a real conflict is only raised when two known creators truly disagree
    - inspection hints now look at the full creator-hint list, not only the first hint
  - rebuild versions were bumped again so old creator-noise rows could not linger:
    - library scan cache -> `scanner-v12`
    - downloads assessment -> `downloads-assessment-v6`
  - real live desktop validation on the app database showed:
    - full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, `updatedFiles = 13010`, `sessionId = 34`
    - installed `conflicting_creator_signals` dropped from `1856` to `0`
    - download-side `conflicting_creator_signals` also settled at `0`
    - open review rows are now down to:
      - `low_confidence_parse = 119`
      - `no_category_detected = 81`
      - `unsafe_script_depth = 20`
      - `conflicting_category_signals = 7`
    - live Inbox refresh finished cleanly and settled at `6 ready / 1 review`
  - the next product focus should stay on stabilization:
    - audit the remaining `low_confidence_parse` and `no_category_detected` rows with the same live-data approach
    - keep fixing watch bugs before any new watch features land
    - keep validating against the real app database instead of trusting fixtures alone
- March 15, 2026: the first deep live package-and-CC confidence sweep found a real foundation gap and tightened it with live proof:
  - the real issue was not that many `.package` files were unreadable
  - many low-confidence installed rows were already being parsed as real `dbpf-package` files, but the inside-file resource inference and filename keyword coverage were too thin to confidently classify them
  - package inspection now recognizes more safe Sims package patterns for:
    - Build/Buy surfaces and structures
    - stronger gameplay tuning clusters
  - filename keyword coverage is now wider for common real-world terms such as:
    - walls, floors, tiles, foundation, stairs, fence, railing, spandrel
    - aspirations, careers, recipes, interactions, lot traits, lot challenges, cheats
  - the scanner now clears stale category warnings when inside-file inspection confirms the final kind, so old `no_category_detected` and `conflicting_category_signals` warnings stop lingering after a true rebuild
  - real live desktop validation on the app database showed:
    - installed low-confidence `Unknown` rows dropped from `304` to `81`
    - installed low-confidence `BuildBuy` rows dropped from `104` to `0`
    - installed low-confidence `CAS` rows dropped from `643` to `1`
    - installed low-confidence `Gameplay` rows dropped from `7` to `4`
    - installed review-item join count dropped from about `3788` to `2082`
  - the next product focus should stay on stabilization:
    - audit `conflicting_creator_signals`, now the biggest remaining review reason by far
    - keep doing live validation on ugly real mod and CC libraries instead of trusting fixtures alone
    - keep watch bug cleanup behind this same confidence-hardening pass
- March 15, 2026: the stale ts4script clue cleanup is now proven end to end in the real app data:
  - `Library` scan cache now bumps when stored inspection meaning changes, so unchanged installed files get one true rebuild instead of silently keeping stale clues
  - the downloads Inbox assessment path now does the same kind of one-time rebuild for unchanged download items when the assessment version changes
  - real desktop validation on the live app data showed:
    - a full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, and `updatedFiles = 13010`
    - the live Inbox refresh then rebuilt unchanged download items under the newer rules
    - strict JSON-array checks now show `0` bad `.pyc` / `_DO_NOT_UNZIP_` namespace values, `0` bad embedded-name marker values, and `0` weak `mc` creator hints in stored ts4script clue fields for both `mods` and `downloads`
  - the next product focus should stay on stabilization:
    - keep doing live validation on ugly real mod and CC libraries instead of trusting fixtures alone
    - keep tightening generic compare confidence before adding new feature surface area
    - keep fixing watch bugs before resuming watch feature growth
- March 15, 2026: real live-library validation found a noisy ts4script clue path and it is now tightened:
  - a read-only check of the live app database showed at least `132` ts4script rows carrying filename-style namespace noise such as raw `.pyc` names or `_DO_NOT_UNZIP_`
  - flat script archives now keep that noise out of:
    - `script_namespaces`
    - `embedded_names`
    - fallback creator hints
  - the next product focus should stay on stabilization:
    - keep doing real live-library validation instead of guessing from fixtures alone
    - keep trimming noisy local clues before they can weaken Inbox and watch confidence
    - keep fixing watch and Inbox trust gaps before new features
- March 15, 2026: safe inside-file extraction is a little stronger now for script mods that ship manifests:
  - manifest parsing now also reads safe author and creator fields, including simple string lists
  - this can improve creator matching when the mod already names its author inside the file
  - this is additive only:
    - script mods without manifests still use the older clue paths
    - manifests are still not required truth
  - the next product focus should stay on stabilization:
    - do more messy live-library validation on generic mods and CC
    - keep looking for more safe inside-file clues that help confidence without widening weak guesses
    - keep fixing watch and Inbox trust gaps before new features
- March 15, 2026: generic Inbox matching now has the next missing candidate-search bridge too:
  - full compare can now search installed rows using inspected `family_hints`, not just hashes, filenames, saved creators, or inspected creator hints
  - that family-based widening is still kept cautious:
    - very short family labels are skipped
    - only the stronger normalized family clues are used to widen the installed candidate pool
  - the next product focus should stay on stabilization:
    - do more messy live-library validation on generic mods and CC
    - audit whether more safe inside-file identity clues can help matching without making it more guessy
    - keep fixing watch and Inbox trust gaps before new features
- March 15, 2026: generic Inbox matching is now in the middle of a confidence-hardening pass too:
  - generic compare now only says `not installed` when the incoming local identity is genuinely stronger
  - a creator clue plus a version clue by themselves now stay `unknown`
  - full compare can now use inspected `creator_hints` to find installed candidates even when no saved creator match exists yet
  - the next product focus should stay on this stabilization track:
    - audit whether family-hint candidate loading needs the same kind of careful tightening or widening
    - do more messy live-library validation on generic mods and CC
    - keep fixing watch and Inbox trust gaps before new features
- March 15, 2026: feature growth should pause until the shared matching and watch-confidence base feels trustworthy:
  - `Library` watch setup is now stricter about what counts as a good setup candidate
  - weak version-only guesses no longer get pushed toward exact-page setup
  - inspected creator hints now feed the shared subject match layer instead of being left on the table
  - ts4script manifest names can now add identity clues when they exist, but manifest files are still optional and not treated as required truth
  - the next product focus should stay on stabilization:
    - audit generic Inbox match thresholds with the same caution pass
    - do more messy real-library validation on mod and CC matching
    - keep fixing watch bugs before adding new watch features
- March 15, 2026: `Library` now has the first compact bulk watch-management layer inside the existing watch center:
  - strongest exact-page candidates can be filled together in one exact-page strip
  - saved reminder-only and provider-needed links now have a real review queue lane
  - the next product focus should stay on finishing that flow cleanly:
    - stronger multi-item setup for many exact-page rows
    - a cleaner batch review lane for many reminder/provider-needed items
    - watch history and source audit only after setup and review feel solid
- March 15, 2026: the watch system now feels more connected across `Home` and `Library`:
  - `Home` watch rows can land on the right Library watch lane
  - `Library` can highlight the right watch section and keep setup/review moving forward
  - the next product focus should be the real next layer after that:
    - stronger bulk setup for exact-page candidates
    - a cleaner batch review lane for reminder-only and provider-needed links
    - watch history and source audit after the setup/review flow feels complete
- March 15, 2026: Library watch follow-up is smoother now:
  - setup suggestions can move straight into the next suggestion after a save
  - saved generic reminder or provider-needed links can be reviewed straight from the tracked list
  - the next product focus should be broadening this into a stronger bulk setup flow without adding another crowded management screen
- March 15, 2026: Library no longer runs its main load and save commands on the window thread. The next product focus should be checking the real app again and then trimming any Library work that is still slow now that the freezing path is gone.
- March 15, 2026: Library now has a first real watch-setup shortlist for installed items that are strong enough to watch but are not set up yet. The next product focus should be turning that shortlist into a fuller setup flow:
  - bulk setup for the strongest exact-page candidates first
  - easier edit and review for saved generic watch sources
  - keep `Home`, `Library`, and automatic watch polling in sync without adding more screen clutter
- March 15, 2026: Library now has a real tracked watch list inside the existing watch center, and the native desktop smoke now proves that list is visible in the real app. The next product focus should be better watch management, especially bulk setup and a clearer way to review items that still do not have a watch source.
- March 15, 2026: Library watch flow is more honest now. Built-in supported special-mod pages are shown as built-in, not as if the user saved them. The next product focus should be the fuller watch-management flow, not more backend watch guesswork.
- March 15, 2026: the compact Library watch center is now in place and the base native desktop smoke passed again. The next product focus can move to broader watch management and provider planning.
- March 15, 2026: the local dev loop is steadier now because `npm run tauri:dev` clears stale Vite listeners on port `1420` before it starts. The next product focus can go back to the fuller watch setup and provider flow.
- March 15, 2026: `tauri:dev` startup no longer dies on the old watch-schema migration or on tray setup during normal launch. The next product focus can go back to the fuller watch setup and provider flow, but the desktop smoke wrapper still needs cleanup so it does not leave Vite running on port `1420`.
- March 15, 2026: safe automatic watch checks now exist, but the next product focus should still be a fuller watch setup and provider flow. The current desktop smoke wrapper also timed out on startup in this session, so that harness needs another cleanup pass before it is treated as perfect signoff.
- March 15, 2026: the first installed-content watch flow now works end to end in the real Tauri app, including `Check now` for safe supported pages. The next product focus should be a fuller watch setup and management flow, not more backend guesswork.
- March 14, 2026: Library watch flow is now cleaner in the real app because Library queries now focus on installed content only. The next product focus should be the first fuller user-facing watch setup flow, not mixing Downloads rows into Library.
- March 14, 2026: Lumpinou Toolbox same-version handling is now confirmed in the real desktop app. The next product focus can move back to broader watch flow and careful special-mod growth.
- March 13, 2026: the shared version and update-watch foundation is now in place for all content, so the next product focus is making the watch flow more complete without slowing Inbox back down.
- March 13, 2026: keep guided install special-mod-only. Generic mods and CC can now compare against installed content, but weak matches must still stay cautious and say `unknown`.
- March 13, 2026: use `docs/SPECIAL_MOD_ONBOARDING.md` and `docs/SPECIAL_MOD_CANDIDATES.json` for future catalog growth. The external Sims mod index stays frozen and reference-only.
- March 13, 2026: keep the queue light and leave the heavier compare and evidence work on the selected item, because that is what brought live Inbox first-open back down to about `1.07s`.

## What Changed This Session

- March 16, 2026: finished the live low-confidence and no-category audit with another real package-classification hardening pass:
  - started from the real rebuilt app database instead of guessing from fixtures
  - confirmed the remaining weak bucket had become small enough to inspect file-by-file
  - found a real path-token bug in package inspection:
    - package context words were being normalized before splitting
    - that hid clear support words like `Strings`, `Thai`, and `LotPrices`
  - changed the file inspector so package context tokens are now read as real separate words
  - widened the safe support-package fallback:
    - pure `StringTable` packages still classify
    - support and translation packages with a small helper-resource mix can now classify too
  - added a narrow gameplay-context fallback for packages that already have gameplay-style inside-file signals and clear add-on/module/integration or dense lot-price context
  - bumped rebuild contracts again so the new parser meaning forced one real rebuild:
    - `scanner-v14`
    - `downloads-assessment-v8`
  - real live desktop validation on the app database showed:
    - full `Library` rebuild completed with `sessionId = 37`, `filesScanned = 13010`, `reusedFiles = 0`, `updatedFiles = 13010`
    - full-scan review work improved from `80` to `50`
    - review reasons are now:
      - `unsafe_script_depth = 20`
      - `low_confidence_parse = 18`
      - `no_category_detected = 10`
      - `conflicting_category_signals = 2`
    - true `Unknown` installed rows are now only `10`
    - live Inbox refresh still settled at `6 ready / 1 review`
  - the leftover unknown rows are now concentrated enough to target directly next:
    - a small Build/Buy texture/object cluster
    - one unreadable package
    - a few isolated gameplay edge cases
  - checks run:
    - `cargo fmt --manifest-path src-tauri/Cargo.toml --all`
    - `cargo test --manifest-path src-tauri/Cargo.toml` with `197` tests
    - `cargo build --manifest-path src-tauri/Cargo.toml`
    - `npm run build`
    - `npm run tauri:build -- --debug`
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1`
- March 16, 2026: finished the live creator-conflict audit and hardened the shared creator-signal rules with real app proof:
  - started with a read-only live-database audit instead of assuming the remaining review noise was watch-related
  - confirmed the `1856` installed creator conflicts were almost entirely fake disagreements from the scanner layer
  - found three real causes:
    - weak filename fallback labels being treated like creator truth
    - unknown folder names being treated like creator truth
    - inspection only looking at the first creator hint instead of the whole hint list
  - changed the scanner so creator signals now behave more like a ranked identity check:
    - known path creators can help
    - unknown path names are ignored
    - weak unknown creator guesses do not fight stronger known creators
    - only two known creators can create a real creator-conflict warning
    - co-author or shared-code cases no longer false-flag when the current creator is already in the hint list
  - bumped rebuild versions again so the real app had to refresh stored creator meaning:
    - library scan cache -> `scanner-v12`
    - downloads assessment -> `downloads-assessment-v6`
  - added direct regression tests proving:
    - a known folder creator can replace a weak filename creator
    - unknown nearest-folder names are skipped
    - two known creators still keep a real conflict
    - inspection hints do not raise a conflict when the current creator already exists in the hint list
  - real live desktop validation on the app data then showed:
    - full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, `updatedFiles = 13010`, `sessionId = 34`
    - installed creator conflicts dropped from `1856` to `0`
    - download-side creator conflicts also settled at `0`
    - the live Inbox refresh completed and stored `downloads-assessment-v6`
    - Inbox finished at `6 ready / 1 review`
    - open review rows are now:
      - `low_confidence_parse 119`
      - `no_category_detected 81`
      - `unsafe_script_depth 20`
      - `conflicting_category_signals 7`
  - full checks passed again after the code change:
    - `cargo check --manifest-path src-tauri/Cargo.toml` passed
    - `cargo build --manifest-path src-tauri/Cargo.toml` passed
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `191` tests
    - `cargo fmt --manifest-path src-tauri/Cargo.toml --all` passed
    - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features` passed with older warnings only
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: finished the first deep live package-and-CC confidence sweep and used it to harden the shared local-classification base:
  - started with a read-only live-database audit instead of guessing from fixtures
  - confirmed many weak `.package` rows were already being parsed as real `dbpf-package` files with resource summaries
  - found the real classification gaps:
    - too few inside-file DBPF resource patterns for Build/Buy and gameplay packages
    - too-thin filename keyword coverage for common real-world Build/Buy and gameplay labels
    - stale category warnings lingering even after the final category had become known
  - widened package inspection to recognize more safe resource clusters for:
    - Build surfaces
    - Build structures
    - stronger gameplay tuning groups
  - widened keyword and subtype coverage for common plural and structural names such as:
    - walls, floors, tiles, paint, foundation, terrain, roof, stairs, fence, railing, spandrel, frieze, trim, ceiling, paneling
    - careers, aspirations, recipes, interactions, lot traits, lot challenges, cheats
  - the scanner now:
    - accepts inspection-driven `Gameplay` promotions
    - raises confidence using inspection confidence floors
    - clears stale `no_category_detected` and `conflicting_category_signals` warnings when inspection truth wins
  - bumped rebuild versions again so the live app truly refreshed stored meaning:
    - library scan cache -> `scanner-v10`
    - downloads assessment -> `downloads-assessment-v4`
  - real live desktop validation on the app data then showed:
    - full `Library` rebuild completed with `scanMode = full`, `reusedFiles = 0`, `updatedFiles = 13010`, `sessionId = 31`
    - live Inbox refresh completed and settled at `6 ready / 1 review`
    - installed low-confidence rows improved:
      - `Unknown 304 -> 81`
      - `BuildBuy 104 -> 0`
      - `CAS 643 -> 1`
      - `Gameplay 7 -> 4`
    - installed review queue join count improved from about `3788` to `2082`
    - the current dominant remaining installed review reason is now clearly `conflicting_creator_signals` at `1856`
  - full checks passed again after the code change:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `187` tests
    - `cargo fmt --manifest-path src-tauri/Cargo.toml --all` passed
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: finished the live stale-clue rebuild pass for both `Library` and `Inbox`:
  - bumped the library scan cache version so a real parser change forces one true installed-library rebuild instead of reusing stale rows
  - confirmed the first blocking full-scan command path was a bad fit for the desktop driver, then switched the live scan to the safer background `start_scan` path
  - a real live-library rebuild completed successfully:
    - `scanMode = full`
    - `reusedFiles = 0`
    - `updatedFiles = 13010`
    - `sessionId = 30`
  - confirmed the next stale-data gap was on the Inbox side:
    - unchanged download items were being reassessed with newer rules
    - but their stored file rows were not being rebuilt
  - fixed that by changing the Inbox version-bump path so unchanged download sources are reprocessed through the real ingest path, not only the cached assessment path
  - bumped the downloads assessment version again so the live app would actually rerun that deeper rebuild after the first partial pass
  - real desktop validation on the live app data then showed:
    - the downloads assessment version moved to `downloads-assessment-v3`
    - Inbox state improved from `5 ready / 2 review` to `6 ready / 1 review`
    - strict JSON-array checks now show `0` bad `.pyc` / `_DO_NOT_UNZIP_` namespace values, `0` bad embedded-name marker values, and `0` weak `mc` creator hints in stored ts4script clue fields for both `mods` and `downloads`
  - full checks passed again after the code change:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `181` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: continued the real-data cleanup pass for ts4script clue quality:
  - a read-only query against the live app database found at least `132` ts4script rows carrying filename-style namespace noise such as raw `.pyc` names or `_DO_NOT_UNZIP_`
  - flat script archives now keep that noise out of:
    - `script_namespaces`
    - `embedded_names`
    - fallback creator hints
  - added direct regression tests proving:
    - flat script archives now skip filename noise in script clues
    - nested script archives still keep the real namespace and creator path working
  - full checks passed again:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `181` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: continued the safe inside-file extraction pass for script-mod manifests:
  - ts4script manifest parsing now also reads safe author and creator fields, including simple string lists in JSON and YAML-style manifest files
  - that means creator clues can now come from clean manifest author data, not only names, namespaces, stems, or saved creator learning
  - this is additive only:
    - script mods without manifests still use the older local clue paths
    - manifests are still optional and not treated as required truth
  - added direct regression tests proving:
    - JSON manifest author lists feed creator hints
    - YAML-style manifest author lists feed creator hints
  - full checks passed again:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `180` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: continued the shared confidence-hardening pass with a careful family-hint candidate-search fix:
  - full compare can now search installed rows using inspected `family_hints`, so real local family clues can actually pull likely installed matches into the scoring pass
  - that widening stays cautious:
    - very short family labels are skipped
    - only the stronger normalized family clues are used to widen the installed candidate pool
  - added direct regression tests proving:
    - the family-hint shortlist itself stays picky about short values
    - family hints can now pull in the right installed match during full compare even when hashes, filenames, and creators do not line up first
  - full checks passed again:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `178` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: continued the shared confidence-hardening pass with the next generic compare fix:
  - generic compare now requires a medium-strength incoming identity before it will say `not installed`
  - trusted version clues now count toward incoming identity only when the version confidence is at least medium
  - full compare can now search installed rows using inspected `creator_hints`, not only saved creator assignments
  - added direct regression tests proving:
    - creator plus version alone now stays `unknown`
    - creator plus family plus version can still report `not installed`
    - creator hints can pull in the right installed match during full compare
  - full checks passed again:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `176` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: started the first real confidence-hardening pass instead of adding more watch surface area:
  - the watch-setup shortlist now checks real parsed clue data instead of only looking for JSON field names in stored `insights`
  - weak version-only rows now stay out of watch setup suggestions instead of being nudged toward exact-page setup
  - inspected `creator_hints` now feed the shared subject match tokens, so local matching can use creator clues already found inside the files
  - ts4script manifest names now add optional identity hints when they exist, but script mods without manifests still use the older local clue paths
  - full checks passed again:
    - `cargo test --manifest-path src-tauri/Cargo.toml` passed with `173` tests
    - `npm run build` passed
    - `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed
- March 15, 2026: the `Library` watch center now carries the next real watch-management layer without adding another management screen:
  - strongest exact-page suggestions are split into a compact bulk exact-page strip
  - saved reminder-only and provider-needed links now get their own compact review queue lane
  - `Home` now gets a real review count from backend truth instead of a frontend guess
- March 15, 2026: fixed a real Library handoff bug:
  - starting watch setup or review while the inspector was empty could drop the pending handoff before the file detail opened
  - `Library` now opens the target file first and then applies the pending watch intent, so setup/review can survive that empty-inspector state
- March 15, 2026: the generic watch smoke path had to be adjusted for the real desktop driver:
  - the Wry webdriver can still miss `Library` row selection clicks even though the file detail exists
  - the smoke now proves the generic save/review/clear flow through the live Tauri command bridge plus the real `Library` UI reaction
  - this keeps a real desktop proof lane in place while leaving row-click behavior available for later manual follow-up
- March 15, 2026: `Home` watch rows now open `Library` with a real watch focus request instead of only opening the generic Library screen:
  - `Watch setup` lands on the setup suggestions lane
  - `Exact updates` / `Updates ready` lands on the tracked confirmed-updates lane
  - the watch center now shows a clear focus message and highlights the right section
- March 15, 2026: `Library` review flow now behaves more like the setup flow:
  - review mode can move to the next saved review item after save, clear, or refresh when the current item no longer needs review
  - review mode now has its own skip action so the user can keep moving without closing the editor and manually reopening the next row
- March 15, 2026: the watch center now has direct “start from here” actions inside the existing surface:
  - `Work through setup` / `Set up watched pages`
  - `Work through review` / `Review watched pages`
  - these reuse the current watch editor and do not create another management screen
- March 15, 2026: the native desktop smoke now proves the wider app flow too:
  - `Home` -> `Watch setup` lands on the setup lane
  - `Home` -> `Exact updates` lands on the tracked confirmed-updates lane
  - the earlier real Tauri watch save/clear flow still passes after these additions
- March 15, 2026: Library watch follow-up now behaves more like a guided queue:
  - `Set up` / `Start setup` still opens the existing watch editor
  - after a save, SimSuite can move straight to the next strong setup suggestion instead of making the user go back and click around again
  - setup mode now has `Skip for now` and `Stop setup` so the user can keep moving or leave the queue cleanly
- March 15, 2026: tracked watch rows can now show a direct `Review` / `Review source` action for saved generic watch pages that still need human follow-up:
  - reminder-only creator pages
  - provider-needed exact pages such as CurseForge links
  - other saved user links that still sit in an unclear state
- March 15, 2026: the existing Library detail panel now handles both flows without adding a second watch-management screen:
  - setup flow opens the editor with the suggested source type and label
  - review flow opens the same editor with the saved watch source already loaded
- March 15, 2026: the native desktop smoke now proves the new follow-up path in the real Tauri app:
  - starts setup from the shortlist
  - saves the watch page
  - switches to `All tracked`
  - opens `Review` for the saved generic watch source
  - closes review and clears the watch source again
- March 15, 2026: moved the heaviest Library command path onto background workers:
  - `get_home_overview`
  - `get_library_facets`
  - `list_library_files`
  - `list_library_watch_items`
  - `get_file_detail`
  - `save_watch_source_for_file`
  - `clear_watch_source_for_file`
  - `save_creator_learning`
  - `save_category_override`
- March 15, 2026: this mirrors the earlier Inbox threading fix, so Library actions should stop freezing the whole app just because Rust is busy.
- March 15, 2026: added the first real watch-setup shortlist in `Library`:
  - the existing watch center now includes a compact `Ready to set up` / `Setup suggestions` block
  - it shows installed items with strong local clues but no saved or built-in watch source yet
  - each suggestion can open the existing Library inspector instead of sending the user to a new management screen
- March 15, 2026: setup suggestions are now more actionable:
  - each suggestion row still opens the existing Library inspector
  - `Set up` / `Start setup` now opens the same inspector with the watch editor already prefilled to the suggested source type
  - SimSuite still does not guess URLs; it only saves the suggested watch type and label so the user can finish the setup safely
- March 15, 2026: `Home` now shows a matching `Watch setup` count from real backend truth instead of relying on a separate frontend guess.
- March 15, 2026: `Home` watch rows now jump straight to `Library`, so users can move from summary counts into real watch follow-up in one click.
- March 15, 2026: the backend now builds watch-setup suggestions from installed files that have enough local clues:
  - creator clues
  - version clues
  - script clues
  - strong filename or embedded-name clues
- March 15, 2026: fixed a real watch-setup filter bug:
  - the setup scan now treats both `.package` / `.ts4script` and `package` / `ts4script` extension styles as valid
  - this keeps valid installed items from being skipped just because older rows store extensions a little differently
- March 15, 2026: the browser-preview mocks now include the same watch-setup response shape so preview work stays closer to the real app.
- March 15, 2026: the native desktop smoke now proves the new setup block is visible in the real Tauri app before it continues with the older watch checks.
- March 15, 2026: added the first real tracked watch list in `Library`:
  - the watch center now includes filter chips for:
    - needs attention
    - confirmed updates
    - possible updates
    - unclear
    - all tracked
  - the watch list now shows the actual tracked items behind those counts instead of only showing summary numbers
  - clicking a tracked watch row opens that item in the existing Library inspector
- March 15, 2026: the backend now builds tracked watch rows from two honest sources:
  - user-saved watch pages
  - built-in supported special-mod official pages
- March 15, 2026: built-in supported special mods now appear in the tracked watch list even before a helper latest-check row exists, so the watch list does not depend on older saved family-state history to notice them
- March 15, 2026: the browser-preview mocks now expose the same tracked watch list shape as the real app
- March 15, 2026: the native desktop smoke now checks the new watch list itself:
  - it switches Library to `All tracked`
  - it confirms tracked file names are visible there before it continues with the older detail checks
- March 15, 2026: tightened the watch-source truth layer:
  - `WatchResult` now says whether the source is:
    - built in for a supported special mod
    - saved by the user
    - not saved
  - supported special mods now keep their built-in official page in Library without pretending it was saved manually
  - custom watch pages for supported special mods are now rejected with a plain message instead of being saved and quietly ignored later
- March 15, 2026: added a compact Library watch center:
  - confirmed update count
  - possible update count
  - unclear watched-item count
  - automatic-check state
  - last automatic run
  - `Check watched pages now`
  - quick jump to `Settings`
- March 15, 2026: cleaned up Library watch actions so they match reality:
  - built-in supported special mods no longer show misleading `Add watch source`, `Change watch source`, or `Clear watch source` buttons
  - built-in sources now explain that SimSuite is using the official page already
  - `Check now` still appears where it is genuinely safe
- March 15, 2026: updated the browser-preview mocks to match the new watch behavior:
  - mock built-in supported mods now carry built-in source origin
  - generic saved pages still show as user-saved
  - mock Home watch counts now match the real backend better and no longer treat every not-yet-watched item as an unknown watch result
- March 15, 2026: cleaned up the local dev loop:
  - `npm run tauri:dev` now uses a wrapper that clears stale Vite listeners on port `1420` before launch
  - the cleanup only auto-stops stale `node`/Vite listeners, not random apps
  - `npm run dev:cleanup` now exists as a manual cleanup helper too
- March 15, 2026: fixed two real startup regressions:
  - older databases now add `anchor_file_id` before creating the watch-source index, so migrated apps no longer crash during setup
  - tray creation is now lazy, so normal app startup does not depend on Windows accepting the tray icon right away
- March 15, 2026: background mode now creates the tray only when it is actually needed:
  - normal launches skip tray setup
  - turning background mode on still prepares the tray
  - close-to-tray still works when the tray can be created
  - if Windows refuses the tray, the app stays open instead of panicking on launch
- March 15, 2026: added the first safe automatic watch-check loop:
  - `Settings` now has automatic watch checks and a check interval
  - users can run `Check watched pages now`
  - the Rust side now polls only safe exact-page sources while the app is open or hidden in the tray
  - Home and Library update through one `watch-refresh-finished` workspace change event after a watch pass completes
- March 15, 2026: tightened watch-source truth and messaging:
  - brand-new databases now create the watch tables correctly
  - `Library` now shows whether a source is `Check now supported`, `Reference only`, or `Provider needed`
  - CurseForge exact pages now save as `provider required` instead of looking like vague unsupported links
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

- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests after the new bulk setup, review queue, and watch-intent handoff changes.
- March 15, 2026: `npm run build` passed after the `Library` watch-center, handoff, and smoke updates.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the real Tauri smoke was widened to prove:
  - the broader watch-center flow still works in the real app
  - a generic creator-page watch source can surface into the review queue
  - clearing that watch source returns the item to setup suggestions
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests after the Home-to-Library watch focus and review-queue changes.
- March 15, 2026: `npm run build` passed after the App, Home, Library, style, and desktop-smoke updates.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the native smoke was widened to prove:
  - `Home` watch rows can land on the right Library watch lane
  - the wider watch-center flow still works in the real Tauri app
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests after the Library watch follow-up changes.
- March 15, 2026: `npm run build` passed after the new setup-queue and review actions were added to `Library`.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the native smoke was widened to prove:
  - setup can start from the shortlist
  - a generic watch page can be saved
  - the saved generic watch row exposes `Review`
  - the watch source can still be cleared afterward
- March 15, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed after the Library command threading fix.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests after moving the Library hot path off the UI thread.
- March 15, 2026: `npm run build` passed after the command-signature changes.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the Library threading fix.
- March 15, 2026: `cargo fmt --manifest-path src-tauri/Cargo.toml` passed after the watch-setup shortlist backend changes.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `170` tests after the watch-setup shortlist and extension-normalization fix.
- March 15, 2026: `npm run build` passed after the Home and Library watch-setup UI changes.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the smoke flow was widened to wait for the new setup section in the real Tauri app.
- March 15, 2026: `npm run build` passed again after the `Home` jump links and the prefilled `Start setup` flow were added.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed again after the wider-app watch follow-up wiring.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `168` tests after the tracked watch-list work.
- March 15, 2026: `npm run build` passed after the Library watch-list UI and mock updates.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed after the smoke script was widened to check the new Library tracked watch list in the real Tauri app.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `165` tests after the watch-source-origin and Library watch-center changes.
- March 15, 2026: `npm run build` passed after the Library watch-center and watch-origin UI changes.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` passed again:
  - the real Tauri app launched
  - the base native smoke completed successfully
  - the watch-management changes did not break the current desktop proof lane
- March 15, 2026: `npm run dev:cleanup` successfully stopped a real stale Vite `node` process that was still listening on port `1420`.
- March 15, 2026: `npm run tauri:dev` now launches through the new wrapper, starts Vite cleanly, and reaches `simsuite.exe` without the old port-conflict failure.
- March 15, 2026: `npm run dev:cleanup` reports `status=free` after the wrapper start check, so the cleanup path is working.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `164` tests after the schema-order and lazy-tray startup fix.
- March 15, 2026: `npm run build` passed after the lazy-tray startup fix.
- March 15, 2026: direct `cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --color always --` reached normal app startup and Downloads watcher work without the old database or tray panic.
- March 15, 2026: `npm run tauri:dev` now gets through Vite and launches `simsuite.exe` without the old setup panic. The wrapper still needs cleanup because interrupted runs can leave port `1420` busy.
- March 15, 2026: `cargo test --manifest-path src-tauri/Cargo.toml` passed with `163` tests after the automatic-watch schema and provider-state work.
- March 15, 2026: `npm run build` passed after the Library and Settings watch-state updates.
- March 15, 2026: `pwsh -NoProfile -File scripts/desktop/run-tauri-smoke.ps1` timed out waiting for startup text, so the desktop smoke wrapper still needs more work before it can be treated as steady signoff for this new watch checkpoint.
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

- The creator-conflict audit turned out to be a real shared-foundation fix, not a watch-only cleanup:
  - installed creator-conflict review noise dropped from `1856` to `0`
  - download-side creator conflicts also dropped to `0`
  - Inbox and Library both benefited because they share the same stored creator clues
- Creator signal handling is more trustworthy now:
  - known creators can reinforce each other even when their text casing or formatting differs
  - weak fallback labels no longer override stronger known creator evidence
  - co-author script-mod cases no longer create fake creator conflicts just because the first hint is not the saved creator
- The first deep live package-and-CC audit turned into a real foundation win instead of another surface tweak:
  - many real installed `.package` rows now land on stronger, cleaner categories
  - stale category-warning noise dropped sharply after the true rebuild
  - live Inbox and Library both benefited because they share the same stored local clue base
- package classification is now better at recognizing real Build/Buy and gameplay files from what is inside the package, not only from filenames
- common Build/Buy and gameplay names now classify more honestly even when creators use plural or structural terms
- `Home` and `Library` now feel more like one watch workflow instead of separate summary and detail pockets.
- The watch center can now guide both setup work and saved review work without adding another crowded manager page.
- Review flow now has the same “keep moving” feel that setup flow already had.
- The real desktop smoke now proves the wider Home-to-Library watch handoff, not just single-item Library actions.
- The watch center can now carry a user through more than one step without forcing them back out into the list after every save.
- Saved generic watch links that still need a human look now have a direct review path from the tracked list instead of making the user hunt through the detail panel first.
- The real desktop smoke still passed after this wider follow-up flow was added, so the new actions are proven in the actual Tauri app instead of only in preview.
- Library now uses the same background-command pattern that already helped Inbox, so the app should stay responsive while Library work is running.
- The command move did not break the real desktop smoke lane or the current watch-management flow.
- The new watch-setup shortlist now gives `Library` a clear “not watched yet” lane without creating a second watch-management screen.
- `Home` and `Library` now agree on the `Watch setup` count because they both use the same backend truth.
- `Home` can now send users straight into `Library` when a watch summary row needs follow-up.
- setup suggestions now shave off an extra step because the existing watch editor can open already prefilled with the suggested watch type and label.
- Installed items are no longer skipped from the setup shortlist just because their stored extension includes a leading dot.
- The new setup block stayed compatible with the real desktop smoke flow, so the wider app integration is still intact.
- The Library watch center now points to real items, not just counts.
- Built-in supported special mods now appear in the tracked watch list even when no helper latest row has been written yet.
- The tracked watch list stays inside the existing Library surface, so the screen is still flat and compact instead of growing a separate management page too early.
- Library now tells the truth about where a watch source came from:
  - built-in official page
  - saved by you
  - not saved
- Built-in supported special mods no longer pretend their official page was manually saved.
- Library now has a compact watch summary area without turning into a cluttered dashboard.
- Global `Check watched pages now` works from Library and refreshes the summary plus the selected item.
- the local dev loop now clears stale Vite listeners automatically before `tauri:dev` starts
- there is now a safe manual cleanup helper for port `1420`
- old databases with watch-source rows now upgrade cleanly instead of crashing on `anchor_file_id`
- normal app startup no longer depends on creating the tray icon first
- background mode still has a tray path, but that tray is created only when needed
- The app can now poll safe saved watch pages automatically while it is running.
- The watch result now says more clearly what kind of source it is:
  - can check now
  - reference only
  - provider needed
- CurseForge links now land in the honest `provider needed` state instead of looking like a generic unclear failure.
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

- The biggest remaining installed-library trust problems are now the lower-confidence rows that are still honestly unresolved:
  - open review rows are now `119 low_confidence_parse`, `81 no_category_detected`, `20 unsafe_script_depth`, and `7 conflicting_category_signals`
  - creator-conflict noise is no longer the main blocker
  - the next stabilization pass should audit the remaining unknown and low-confidence package rows with the same live-data method
- Generic package confidence is much better now, but it is still not at the final trust goal:
  - `81` installed rows still sit in low-confidence `Unknown`
  - some gameplay packages are still only medium-confidence because their inside-file clues are weaker or noisier
  - more live validation is still needed on messy mixed CC libraries, not just script-heavy mods
- Generic compare is now stricter, but it is still not at the final confidence goal yet:
  - family-hint loading is still narrower than the new creator-hint loading
  - some generic CC and mod cases still need live validation outside fixture tests
  - the queue summary wording may need a follow-up pass once more real-world generic compare cases are checked
- The first confidence-hardening pass is in, but the broader generic matching layer still needs the same audit treatment:
  - Inbox and Library watch setup still depend on the same shared subject match and version-confidence rules
  - this session tightened watch setup first because it was the safest user-facing place to start
  - the next stabilization step should audit generic installed-match thresholds before more watch features land
- ts4script manifest names are now helpful extra evidence, but they are not universal:
  - many script mods do not ship a manifest at all
  - SimSuite still has to rely on filenames, namespaces, creator hints, family hints, and other local clues for those mods
- The real desktop smoke passes again, but there is still one test-harness caveat:
  - the Wry webdriver does not reliably select `Library` rows in this watch flow
  - the current generic watch smoke now uses the live Tauri command bridge to save and clear the source, then checks the real `Library` UI reaction
  - if future work changes `Library` row selection or the detail panel, do one manual desktop click-through too
- The watch system is much smoother now, but it is still not true bulk setup:
  - users still add one real watch URL at a time
  - SimSuite still intentionally avoids inventing or guessing watch URLs
- There is still no true batch review lane for many reminder-only or provider-needed links at once.
- There is still no watch history or source audit trail yet.
- There is still no true multi-save bulk setup flow yet:
  - the app can move to the next strong suggestion after a save
  - but it still relies on the user to paste or confirm each watch URL one at a time
- Saved provider-needed and reminder-only watch pages are easier to review now, but there is still no dedicated batch review lane for many of them at once.
- The freeze path should be fixed, but Library may still have real slow work left underneath:
  - `get_home_overview` still computes watch counts and setup counts
  - `get_file_detail` still does deeper version and watch resolution
  - if the real app still feels slow after this threading fix, the next step is trimming those code paths instead of moving more commands around
- Watch management is better, but still not complete:
  - there is still no bulk setup flow for unwatched installed items
  - there is still no dedicated review surface for items that could be watched but are not set up yet
  - there is still no edit history or source audit trail
- The next missing layer is fuller watch management:
  - no easy bulk setup flow yet
  - no edit history or source audit trail yet
  - no provider onboarding flow yet
- Built-in supported special mods now use their own official page in Library, but custom override pages for those built-ins are intentionally blocked for now because there is no honest merge rule yet.
- The watch system is readable now, but the user-facing management flow is still thin:
  - watch results can be shown
  - generic watch sources are stored in the database
  - Library can now save, clear, and sometimes check a watch source for installed items
  - but broader setup, editing, batch setup, provider setup, and polling flows still need to grow carefully
- The safe automatic watch loop exists now, but it still needs a fuller management story:
  - no watch history view yet
  - no provider onboarding UI yet
  - no clear watch list screen yet
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
- The deeper native watch smoke still needs widening:
  - the base smoke passed this session
  - but richer Library watch scenarios should be added before we rely on it as the final signoff path for all watch features

## Important Decisions

- Treat inside-file DBPF resource clusters as stronger category evidence for `.package` files when they match safe known Sims patterns.
- If inspection truth confirms the final file kind, stale category warnings should be cleared instead of left behind as fake review noise.
- Keep feature freeze in place until creator-conflict noise is audited, because that is now the biggest remaining foundation problem.
- Generic compare should prefer `unknown` over `not installed` unless the incoming local identity is at least medium-strength.
- Inspected `creator_hints` are allowed to help candidate search during full compare because they come from local file inspection, not from network guesswork.
- Stop adding new watch features until the shared matching and confidence base is tighter.
- Treat ts4script manifest names as optional helper evidence only, not as required truth for script mods.
- Keep watch follow-up inside the current `Library` watch center and detail panel instead of creating a separate watch-management screen.
- `Home` watch rows should open `Library` with intent, not just with navigation.
- Setup and review should both behave like guided follow-up queues where that reduces repeated clicks, but SimSuite still must not guess URLs or silently save links.
- The fuller watch-management flow should keep reusing the existing Library detail panel instead of creating a separate watch-management screen.
- Setup follow-up should reduce repeated clicks, but SimSuite still must not guess or invent watch URLs.
- Review actions should be shown only for the saved user-managed watch sources that actually need follow-up, so the tracked list stays compact.
- The first fuller watch-management step should stay inside the current Library screen:
  - summary counts
  - tracked watch list
  - existing detail panel
  - not a new heavy dashboard page
- Built-in supported special-mod pages and user-saved watch pages are different product states and must stay visibly different in Library.
- SimSuite should block misleading custom watch-page saves for supported special mods until there is a real rule for how built-in and custom sources should coexist.
- tray creation should be lazy:
  - normal startup must not fail just because the tray icon is unhappy
  - background mode can request the tray later when it actually needs it
- Local installed-vs-downloaded truth stays first.
- Official latest stays helper-only.
- Automatic watch checks should only touch safe exact-page sources and approved providers.
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
- Start from stabilization, not feature growth:
  - audit the remaining `low_confidence_parse` and `no_category_detected` rows first, because creator-conflict noise is now cleared
  - compare the remaining weak `.package` and `.ts4script` rows against their stored `resourceSummary`, creator clues, and family clues to see what safe signal is still missing
  - keep an eye on `unsafe_script_depth` and `conflicting_category_signals`, but do not let those distract from the much larger low-confidence bucket
  - re-check generic Inbox queue summaries against the stronger package classification results
  - keep watch setup suggestions cautious unless the local clues are genuinely strong
- Check the wider watch flow again in the desktop app:
  - `Home` -> `Watch setup`
  - `Home` -> `Exact updates`
  - start setup from the shortlist
  - save a generic watch page
  - review a saved generic watch source
  - clear the watch source
- If the next session touches `Library` row selection or the smoke again, do one manual real-app click-through too because the current Wry webdriver is still shaky there.
- Then move to the next watch-management gap:
  - stronger bulk setup for exact-page candidates
  - a cleaner batch review lane for saved reminder/provider-needed sources
  - then watch history / source audit after that
- Then use `docs/SPECIAL_MOD_ONBOARDING.md` before adding any new supported special mod.
- Next best product steps:
  - build the next watch-management step on top of the new tracked watch list:
    - bulk setup for unwatched installed items
    - easier source editing for generic watched items
    - a better way to review items that still need a watch source
  - widen the native Library watch smoke beyond the current base lane
  - decide whether SimSuite should add provider adapters after that, starting with a CurseForge feasibility check against their API terms and key requirements
  - widen helper-only latest parsing only where there is a safe official endpoint
  - add the first small curated expansion wave through `docs/SPECIAL_MOD_CANDIDATES.json`
  - keep checking Inbox detail performance so the broader compare system does not make the screen feel heavy again
