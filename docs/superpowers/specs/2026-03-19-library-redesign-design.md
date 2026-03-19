# Library Redesign Spec

Date: March 19, 2026
Owner: Codex
Status: draft for user review

## Summary

`Library` should become a calm, list-first collection browser.

The page should stop using a heavy left filter rail as the main structure. Instead, it should use:

- a compact filter strip at the top
- a full collection list underneath
- a steady right detail sidebar that stays open while the user moves through mods
- deeper sheets or modals only for tasks that need more room

This redesign replaces the earlier left-heavy `Quiet Catalog` layout with a flatter, cleaner `Top Strip + List + Sidebar` layout.

## Goals

- Keep the page calm, cozy, and organized.
- Let the library list own the page.
- Show only the information a user needs when they need it.
- Let users click through many mods quickly without opening and closing a popup every time.
- Keep the screen fitting cleanly inside the desktop window.
- Avoid unnecessary scrolling.

## Non-Goals

- This page should not become a full warning dashboard.
- This page should not become the main place for update workflows.
- This page should not keep editing forms open all the time.
- This page should not show every technical detail at once, even in `Creator`.

## Main User Question

When someone opens `Library`, the page should answer:

`What is in my collection, and is this item okay?`

## Core Layout

The new `Library` page should use one stable desktop shape for all three user views:

1. Top filter strip
2. Main list area
3. Right detail sidebar

The page should feel like a collection browser, not a dashboard.

## Top Filter Strip

The filter strip should sit above the list and stay visually light.

### Always visible

- search
- type
- creator
- health or status
- shown count
- total count
- active filter count
- `More filters`

### On-demand extra row

Opened by `More filters`.

Can include:

- subtype
- folder/root
- confidence
- tracked-only or warning-only toggles
- reset filters

### Strip rules

- Use a low-profile horizontal layout, not a stack of boxed controls.
- Keep counts as quiet inline chips, not dashboard cards.
- Keep the strip at the top of the page, not in a side column.
- Keep the strip height stable so the list below does not jump.

## Main List Area

The list is the hero surface.

### Shared row structure

Every row should show:

- mod name
- type or category
- basic health state

### Casual row

Show:

- mod name
- type
- one clear status
- at most one short clue

Purpose:

- answer whether the file looks okay
- avoid overload

### Seasoned row

Show:

- mod name
- type
- creator
- health state
- one or two useful clues

Examples:

- tracked
- warning
- update available
- dependency issue

Purpose:

- help users understand what affects their setup

### Creator row

Show:

- mod name
- creator
- type or subtype
- health state
- a small number of deeper clues

Examples:

- tracked
- warning
- compatibility hint
- dependency clue
- source clue

Purpose:

- give more depth without turning each row into a full technical report

### List behavior

- Clicking a row updates the right sidebar.
- The sidebar stays open while the user clicks through different rows.
- Arrow keys should move selection.
- Enter can open deeper detail when useful.
- Sticky headers should stay visible.
- Horizontal scrolling should be avoided.
- Row height should stay steady.

## Right Detail Sidebar

The sidebar is for fast understanding, not full inspection.

### What stays in the sidebar

- mod name
- creator
- type and subtype
- short health summary
- tracked or update state
- a few important warnings if any
- small action buttons

### What should not live in the sidebar all the time

- deep file inspection
- edit forms
- long warning breakdowns
- full source and path detail
- full dependency lists
- large technical note blocks

### Sidebar behavior

- stays open while the user changes selection
- updates softly and quickly
- should scroll only when the selected item genuinely has more content than the sidebar height
- should not force the whole page to scroll

## Deeper Layers

Use details-on-demand to keep the main page calm.

### Side sheets

Use a side sheet for deeper reading:

- health details
- inspect file
- source or path detail
- richer warning context

### Modals

Use a modal for focused tasks:

- edit details
- confirm a change
- other short action flows

### Layering rule

- browse in the list
- understand in the sidebar
- inspect in a sheet
- edit in a modal

## Scroll Rules

This is a hard requirement for the redesign.

### Allowed scrolling

- the list area
- the right sidebar, only if needed
- side sheets
- modals with larger forms

### Avoid

- page-level scrolling for the main `Library` screen
- horizontal scrolling in the main layout
- extra nested scroll areas inside small cards
- oversized filter areas that push the page taller than the window

### DOM fit rule

The page must fit inside the desktop window cleanly.

- The main `Library` layout should stretch to the available workspace height.
- The top filter strip should take only the height it needs.
- The list should fill the remaining height.
- The sidebar should match the same vertical working area.
- Open sheets and modals should stay fully reachable inside the window.

## Motion

Motion should be calm and useful.

### Use

- soft row selection changes
- gentle sidebar content swaps
- light list reshuffle motion after filtering
- smooth sheet slide-in
- quick modal fade and settle

### Avoid

- bouncing rows
- dramatic scaling
- loud hover transforms
- flashy page transitions
- always-moving background effects

## Information Boundaries

`Library` should focus on browsing and understanding the collection.

### Keep in Library

- what the file is
- whether it looks okay
- creator and type clues
- short update state
- short warning state

### Push to other workspaces when deeper work is needed

- updates workflow -> `Updates`
- guided fixes -> `Review`
- cleanup or relocation work -> `Organize`
- creator/type learning sweeps -> `Creators` and `Types`

## Accessibility and Input Rules

- Keyboard selection must be clear and reliable.
- Focus outlines must stay visible.
- Status colors must still read clearly without color alone.
- Reduced motion should remove sliding and heavy reshuffle effects.
- The page must remain readable at the app's normal desktop sizes without forcing unnecessary wheel scrolling.

## Final Design Statement

`Library` becomes a calm collection browser with a top filter strip, a list that owns the page, and a steady sidebar that lets users move through mods without friction.
