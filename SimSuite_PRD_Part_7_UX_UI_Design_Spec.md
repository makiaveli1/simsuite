# Sims Mod Suite — PRD Part 7: UX / UI Design Spec

## Status
Draft v1

## Purpose
This document defines how the app should feel, look, and communicate.
It is meant to guide design and front-end implementation so the product
feels playful and Sims-like, while still being clear, trustworthy, and easy to use.

This document covers:

- visual identity
- layout rules
- component behavior
- interaction patterns
- motion and feedback
- accessibility basics
- tone of interface writing

---

# Current UX note (March 11, 2026)

The app now has richer Inbox logic than earlier drafts assumed, but the
Inbox surface is still the weakest UX area in real use.

Current UX priorities:

- keep Inbox calm, direct, and fast
- avoid stacked boxes that repeat the same information
- keep the right panel and decision panel ordered and easy to scan
- show guidance only where it is needed; do not flood the screen with
  hints
- keep Casual, Seasoned, and Creator different by emphasis and default
  detail, not by making Casual feel more crowded

Current known UX issue:

- Inbox can still feel heavy and unstable when the user first opens it
  or selects items, especially for special mods

The next UX pass should simplify and speed up Inbox before adding more
UI complexity.

---

# 1. Experience Goals

The app should feel:

- playful
- friendly
- tidy
- modern
- safe
- a little game-like
- never childish
- never cluttered

The emotional goal is:

**“This feels like a polished Sims companion tool that understands my library and helps me stay in control.”**

---

# 2. Design Principles

## 2.1 Safe first
The design should make risky actions feel deliberate.
Dangerous operations must be visually separated from routine actions.

## 2.2 Clear hierarchy
The user should always know:
- where they are
- what the app found
- what the app wants to do
- what will happen next

## 2.3 Friendly without fluff
The app can feel warm and expressive, but should not get in the way of productivity.

## 2.4 Visual calm
Many users will already feel stressed by messy mod folders.
The interface should reduce that stress, not add to it.

## 2.5 Layered depth
Beginner users should see simple actions first.
Advanced controls should be available but not overwhelming.

---

# 3. Visual Identity

## 3.1 Style direction
Use a visual language inspired by Sims-adjacent ideas:

- gem-like highlights
- bright accents
- rounded panels
- soft shadows
- clean cards
- subtle game dashboard energy

Do not imitate official Sims UI directly.
The app should feel inspired, not copied.

## 3.2 Suggested visual traits
- rounded corners
- airy spacing
- soft surfaces
- clear icon-led navigation
- light decorative color accents
- structured card layouts

---

# 4. Color System

## 4.1 Core palette behavior
The palette should feel lively but controlled.

Recommended roles:

- Primary accent: plumbob-like green
- Secondary accent: sky blue or aqua
- Support accent: warm yellow
- Error: coral red
- Warning: amber
- Success: green
- Neutral surfaces: soft off-white, cool gray, charcoal

## 4.2 Usage rules
Primary accent should be used for:
- active nav item
- main action buttons
- progress highlights
- selected filters

Warning colors should be used for:
- risky paths
- suspicious files
- review alerts

Error colors should be used only for:
- failed actions
- blocked actions
- missing bundle problems
- unrecoverable issues

## 4.3 Dark and light mode
Support both light and dark themes later.
Design the system from tokens so either mode can be added without redesigning every screen.

---

# 5. Typography

## 5.1 Tone
Typography should feel clean and friendly.

## 5.2 Roles
Use a small type scale:

- Display: welcome and major headers
- Heading: page titles
- Subheading: section labels
- Body: normal text
- Caption: metadata and helper text
- Label: filters, pills, tags, buttons

## 5.3 Rules
- Avoid very small text for important metadata
- Use bold selectively for file names, creators, and warnings
- Keep paragraph text short and readable
- Use sentence case, not all caps, for most interface text

---

# 6. Layout System

## 6.1 App shell
The desktop app should use a three-part layout:

- left sidebar navigation
- main content area
- optional right-side detail panel

## 6.2 Sidebar
Sidebar includes:
- logo / app name
- main navigation
- scan button or scan status
- storage health / sync style summary later if needed

The sidebar should stay stable across the app.

## 6.3 Content area
The main area should use:
- large page header
- summary strip if helpful
- primary content blocks in cards
- clear spacing between sections

## 6.4 Right detail panel
This should appear when the user selects:
- a mod
- a tray bundle
- a duplicate group
- a review item

It should show detailed info without forcing page navigation.

---

# 7. Core Screen Design Direction

## 7.1 Home
Home should feel warm and instantly informative.

Top section:
- greeting / welcome
- current mode
- last scan status

Main cards:
- total mods
- total tray items
- duplicates
- review queue
- unsafe paths
- new downloads

Lower section:
- recent activity
- quick actions
- suggested next steps

## 7.2 Library
Library should feel like a clean content browser.

Needs:
- search
- filter bar
- content cards or rows
- quick status chips
- selection state

Each row/card should show:
- item name
- creator
- type
- location
- confidence or status

## 7.3 Organize
This should feel like a control room for filing rules.

Needs:
- current detected structure
- rule preset cards
- editable templates
- preview before apply
- automation level setting

## 7.4 Review
This is the trust screen.

It must make uncertainty understandable.

Each review card should show:
- file or bundle name
- why it needs review
- suggested destination
- confidence
- actions: accept, edit, ignore, always do this

## 7.5 Duplicates
Should feel investigative, not scary.

Needs:
- grouped duplicate sets
- why they are considered duplicates
- newest / oldest indicators
- safe actions

## 7.6 Tray
Should feel separate from Mods.

Use bundle-based visuals:
- household
- lot
- room

Show related file pieces clearly.

## 7.7 Patch Recovery
This should feel serious and stable.

Use stronger visual cues:
- snapshots
- before / after comparisons
- grouped creators
- hold areas
- rollback actions

## 7.8 Settings
Should be simple and grouped:

- folders
- safety
- AI
- automation
- appearance
- future mobile settings placeholder

---

# 8. Component Library

## 8.1 Cards
Cards are the main building block.
Use them for:
- summary metrics
- content groups
- alerts
- review items
- duplicate sets

Cards should support:
- title
- metadata
- status chip
- actions

## 8.2 Chips / tags
Use for:
- CAS
- BuildBuy
- Gameplay
- Script
- Tray
- Duplicate
- Needs Review
- Unsafe Path

Chips should be visually distinct but not too loud.

## 8.3 Buttons
Button hierarchy:

Primary:
- main forward action

Secondary:
- support action

Tertiary / ghost:
- low-emphasis action

Danger:
- destructive action

Examples:
- Apply Suggestions
- Review Items
- Create Snapshot
- Move to Archive
- Undo

## 8.4 Tables and lists
Use when the user needs precision and volume.
Examples:
- file lists
- scan results
- duplicate details

## 8.5 Empty states
Every major screen needs a designed empty state.

Examples:
- no duplicates found
- no review items
- no new downloads
- no tray bundles

These should feel encouraging, not dead.

## 8.6 Modals
Use modals only for:
- dangerous confirmations
- snapshot restore
- folder selection
- automation changes

Avoid putting long workflows inside modals.

---

# 9. Navigation Patterns

## 9.1 Main navigation
Always visible in sidebar.

## 9.2 Local navigation
Some screens may need tabs.

Examples:
- Library: Mods / Scripts / Overrides / Unknown
- Tray: Households / Lots / Rooms
- Patch Recovery: Snapshots / Holds / Suggested Checks

## 9.3 Breadcrumbs
Use only where it helps with complex screens.
Not required everywhere.

---

# 10. Search and Filtering

Search is critical.

## 10.1 Search behavior
Should support:
- file name
- creator
- folder
- set name
- bundle name

## 10.2 Filters
Should be visible and removable.
Use filter chips with clear reset actions.

Useful filters:
- category
- subtype
- creator
- content area
- review status
- duplicate type
- confidence level

---

# 11. Motion and Feedback

## 11.1 Motion style
Motion should be:
- soft
- quick
- supportive
- never flashy

## 11.2 Good uses of motion
- showing selected state
- expanding detail panel
- confirming successful actions
- loading scan progress
- highlighting changed items after a move

## 11.3 Avoid
- long animations
- bouncing effects
- excessive movement on dense data screens

---

# 12. States and Status Language

Important states should always be visible.

Examples:
- safe
- needs review
- duplicate
- low confidence
- missing files
- unsafe script path
- ready to move
- waiting for approval

These states should use both:
- text
- color or icon

Never rely on color alone.

---

# 13. Accessibility Basics

The app should support:

- keyboard-friendly navigation
- readable contrast
- large enough touch targets for future mobile adaptation
- icons with labels or tooltips
- color-independent warnings
- reduced motion support later

---

# 14. Tone of Voice

The app should speak in plain, warm English.

Good examples:
- “We found 3 files that may be in the wrong place.”
- “This script mod is deeper than recommended.”
- “We can fix this after you approve it.”
- “Your current folder system was detected and saved.”

Avoid:
- heavy technical jargon
- robotic phrasing
- overly cute language
- blameful wording

---

# 15. Suggested Design Tokens

Codex or design implementation should define reusable tokens for:

- colors
- radius sizes
- spacing scale
- shadow levels
- font sizes
- icon sizes
- transition timing

This prevents inconsistent UI later.

---

# 16. Visual Priority Rules

When multiple signals appear on screen, show them in this order:

1. blocked or dangerous issue
2. action needed now
3. useful summary
4. optional detail
5. decorative element

Safety and clarity always beat style.

---

# 17. First Release UI Priorities

For v1, the most important screens to polish first are:

1. Home
2. Library
3. Review
4. Organize
5. Tray

These will shape trust and first impressions.

---

# 18. Handoff Notes for Codex

Codex should implement the interface as:

- a reusable design system
- token-based theming
- shared cards, chips, buttons, and panels
- consistent page headers and spacing
- friendly copy and status language

The UI should feel like a real product, not a developer tool with a coat of paint.
