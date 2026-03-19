# Home Redesign Spec

Date: 2026-03-19
Scope: `Home` screen only

## Goal

Turn `Home` into a calm personal landing surface instead of a mini command center. It should feel closer to opening a clean browser start page than opening a dashboard full of actions.

## Direction

- Use the approved direction:
  - the calmer structure from `Minimal Widget Shelf`
  - the softer atmosphere from `Ambient Canvas`
- Keep the page flat and desktop-like
- Remove the permanent right inspector
- Keep navigation work in the sidebar, not on the `Home` page itself
- Use a side sheet for personalization instead of always-visible controls

## Main Experience

### Page shape

`Home` becomes a single centered surface with:

1. A quiet top row
   - status tone
   - theme chip
   - `Customize Home` button
   - scan button kept available because it is a core system action, not a navigation shortcut

2. A main hero panel
   - short summary of current library state
   - one strongest system message
   - a compact status rail underneath

3. A small set of glance modules
   - system health
   - update watch
   - folder readiness
   - library facts

4. A right-side customization sheet
   - open only when needed
   - controls theme, density, focus card, and visible modules

### What should not be on Home

- no permanent right inspector
- no long list of navigation buttons to other screens
- no repeated explanations that restate headings
- no wallpaper system
- no always-open advanced edit forms

## View Modes

### Casual

- calmest default
- fewer visible modules
- simpler words
- more summary, less evidence
- folder help stays visible sooner if setup is incomplete

### Seasoned

- balanced default
- three or four glance modules visible
- more exact wording
- system health and watch status stay equally visible

### Creator

- densest version of the same page
- same structure, but richer fact rows
- more receipts in the lower modules
- module defaults lean toward watch status and library facts

## Personalization

`Customize Home` side sheet should support:

- show or hide each module
- pick the main hero focus:
  - `Library health`
  - `Update watch`
  - `Folder setup`
- choose density:
  - `Calm`
  - `Balanced`
  - `Detailed`
- pick any existing app theme
- toggle ambient motion:
  - `Still`
  - `Ambient`

This should save per user view, so `Casual` can stay simpler while `Creator` keeps more detail.

## Motion

- page content should fade and rise in gently on load
- hero and modules should stagger in lightly
- customization sheet should slide in from the right
- ambient background motion should be very subtle and respect reduced motion

## Layout rules

- page should fit a 1440x960 desktop view without main-page scrolling
- modules should use clean grid alignment with generous breathing room
- the hero should be the visual anchor
- lower modules should never feel like a wall of equal-weight cards

## Data rules

- keep using existing home overview data
- do not add new backend requirements for this pass
- use display logic to reshape what is shown per mode

## Verification

- check `Home` in:
  - `Casual`
  - `Seasoned`
  - `Creator`
- test both:
  - complete folder setup state
  - incomplete folder setup state, if available
- verify reduced-motion-safe behavior
- run `npm run build`

## Success test

The page should feel calm enough that a new user can read it in a few seconds, while a heavier user still feels like it is personal, useful, and worth opening first.
