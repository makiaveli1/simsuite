# Library Redesign Spec

Date: 2026-03-19
Scope: `Library` screen only

## Goal

Turn `Library` into a calm collection browser that helps simmers understand what they have without pushing every warning, edit form, or deep inspection panel onto the screen at once.

The page should feel like:

- calm
- cozy
- focused
- organized

The main product question for `Library` is:

`What is this file, and do I need to care right now?`

## Direction

Use the approved direction:

- `Quiet Catalog`

This means:

- the list is the hero
- inspection supports browsing instead of competing with it
- deeper detail appears only when the user asks for it
- the page stays stable and desktop-like

`Library` is not:

- a warning center
- an update manager
- a bulk fix workspace
- a giant always-open metadata form

## Current Problems

From the current screen and the latest live captures:

- too much of the right side is open all at once
- editing and understanding are mixed together
- the left rail still feels more like a form than a calm browsing tool
- the center table is useful, but it does not feel like the clear star of the page
- the page still shows more information than many users need before they even select a file

Reference captures used during this spec pass:

- `output/playwright/library-design-casual-current.png`
- `output/playwright/library-design-seasoned-current.png`
- `output/playwright/library-design-creator-current.png`

## Main Experience

`Library` becomes the calm collection room of the app.

The page rhythm should be:

1. scan the list
2. pick one file
3. understand that file
4. open deeper detail only if needed

The page should answer three things in order:

1. what exists in the collection
2. what kind of item each file is
3. whether anything needs attention

## Page Shape

`Library` keeps a desktop workbench layout, but with a quieter balance:

1. a slim top strip
2. a light filter rail on the left
3. a dominant collection list in the middle
4. a short understanding panel on the right

The center list should own the most width.

## Top Strip

The top strip should stay small and quiet.

It should show:

- total shown
- total in library
- active filter count
- search
- `More filters`
- a small layout preset switch only if still useful after redesign

It should not:

- repeat a long page explanation
- hold many unrelated actions
- feel like a second toolbar stacked on top of the real work

## Left Filter Rail

The left rail should help narrow the collection quickly without reading like a form.

Visible by default:

- search
- type
- creator
- root or profile
- health/status
- update state

Moved behind `More filters`:

- subtype
- confidence
- narrower technical filters
- future advanced filter combinations

The rail should be:

- collapsible
- quick to scan
- lighter than it is now

## Collection List

The collection list is the heart of the page.

Every row should always show:

- mod or file name
- category or type
- one clear health signal
- one or two supporting facts

Rows should not try to show:

- long warning explanations
- full paths for normal users
- edit actions
- deep technical receipts
- too many badges

The list should feel like:

- a calm browser
- stable under filtering
- easy to scan quickly

## Right Detail Panel

The right panel should stop behaving like a full editor and instead act as a short understanding panel.

When nothing is selected:

- show a quiet empty state
- explain that selecting a file reveals what it is, how healthy it looks, and what to open next

When a file is selected, group the panel into three quiet layers:

1. `Snapshot`
   - what this file is
   - creator
   - type
   - status
   - version or update hint

2. `Care`
   - warnings
   - dependencies
   - compatibility
   - plain-language next-step guidance

3. `More`
   - buttons to open deeper detail

The right panel should not keep every editor open by default.

## Details On Demand

Important detail should move into focused on-demand surfaces.

### `Health details` sheet

Use for:

- warnings
- conflicts
- missing dependencies
- compatibility notes
- fuller reasoning behind health flags

### `Inspect file` sheet

Use for:

- full path
- hash
- parser or package signals
- inside-file clues
- richer creator-mode receipts

### `Edit details` sheet

Use for:

- creator save/fix
- type override
- related metadata correction

This keeps the main page calm while still preserving power.

## View Modes

The page structure should stay the same in all three user modes.

What changes is the amount of information shown by default and the kinds of questions the page answers first.

### Casual

Primary question:

- `Is it okay, and do I need to do anything?`

List should emphasize:

- name
- type
- active or installed state when available
- update-ready or needs-attention state

Detail panel should emphasize:

- what the file is
- whether it looks safe
- whether an update or warning matters
- one or two safe next actions

Hide by default:

- deep path info
- parser clues
- raw inspection data
- editing forms

### Seasoned

Primary question:

- `What is affecting my setup?`

List should emphasize:

- name
- creator
- type
- health signal
- version or update hint
- root or profile context when useful

Detail panel should emphasize:

- setup-relevant context
- health and version state
- what changed recently
- where to go next for deeper management

### Creator

Primary question:

- `How is this built, linked, and debugged?`

List should emphasize:

- name
- creator
- source
- version
- compatibility or health
- priority or impact hints if relevant

Detail panel should emphasize:

- creator/source/version truth
- dependency and incompatibility clues
- config/debug availability
- exact metadata when requested

Even in `Creator`, deeper inspection should still live behind organized sheets instead of staying fully open.

## Product Boundaries

`Library` should own understanding, not every other system.

The product boundary is:

- `Library` explains
- `Updates` manages tracked updates
- `Review` owns risky decisions
- `Organize` owns moving files
- `Creators` and `Types` own batch fixing

That means `Library` may link outward, but should not absorb those flows into the main page.

## Actions

Good quiet actions for `Library`:

- `Open in Updates`
- `Inspect file`
- `Edit details`
- `Open creator batches`
- `Open type batches`

Do not make these central in `Library` right now:

- enable or disable
- uninstall
- reinstall
- open logs
- export/share
- conflict resolution workflows

Those may exist later, but they should not pull the page away from its main purpose.

## Motion and Smoothness

`Library` should feel steady, soft, and desktop-like.

Animate only what helps people understand change:

- row selection
- filter narrowing
- sheet open and close
- small status updates

Avoid:

- bouncing rows
- loud hover lifts
- pulsing badges
- dramatic page movement
- busy background motion

Behavior rules:

- filtering should feel like narrowing, not rebuilding
- selected-row change should softly update the right panel
- sheets should slide in from the right with a light dim behind them
- reduced motion should fall back to simple fades or near-instant changes

## Behavior Rules

### On first open

- if a remembered selection still exists, restore it
- otherwise select the first visible file
- do not let the right panel overpower the list

### List discipline

The list should answer:

- what it is
- what type it is
- whether it needs attention

It should not answer:

- every warning detail
- raw parser metadata
- every compatibility reason

### Detail discipline

The right panel should answer:

- what this file is
- how healthy it looks
- what matters next

It should not become:

- a giant editor
- a debug console
- an update workspace
- an organize workspace

### Scrolling

The whole page should not feel like one long scroll.

Only these parts should scroll:

- filter rail
- collection list
- detail panel
- sheets and drawers

## Information By User Type

The following guidance is intentionally inspired by the user examples and the PRD, but adapted to the app's screen boundaries.

### Casual: show in the list or immediate detail

- mod name
- on/off or active state when available
- installed status
- update available
- error or warning status
- missing dependency warning
- conflict warning
- game-version compatibility when already known

### Seasoned: add in the list or immediate detail

- current version
- latest version or update hint
- load order or priority when relevant
- incompatibility warning
- profile or game instance context
- source or website hint
- last updated or installed date

### Creator: add in deeper detail or creator-mode default detail

- author
- source or website
- dependency list
- incompatibility list
- priority impact
- config/settings availability
- profile or game instance
- game version compatibility
- mod-loader compatibility
- exact health reasoning
- last updated or installed date

The strong rule is:

- beginners need `is it working and what should I do?`
- seasoned users need `what is affecting my setup?`
- creators need `how is this built, linked, and debugged?`

## Accessibility and Input

`Library` should feel like a real desktop browser:

- arrow keys move through rows
- tab order stays predictable
- Enter can open the next relevant detail layer
- Escape closes sheets
- focus states stay clear and stable

## Future-Friendly Hooks

The redesign should leave room for later improvements without cluttering version one:

- saved filter views
- pinned columns
- optional small thumbnails for some CAS or Build/Buy content
- stronger profile awareness
- custom visible fields by mode

These are future possibilities, not required for the redesign pass.

## Final Design Sentence

`Library` becomes a quiet catalog where the collection list leads, the selected file is explained clearly, and deeper editing or inspection appears only when the user asks for it.`
