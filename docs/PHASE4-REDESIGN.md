# SimSort Phase 4 — UI/UX Redesign Plan
## Ariadne (Studio) — Senior UI/UX Design Document

---

## Executive Summary

Phase 4 takes the solid Phase 3 foundation — theming, motion, view modes, the sheet overlay — and closes the gap between a functional library and one that actually *thinks like a simmer*. The core problems to solve:

1. **Filter rail is underpowered** — dropdown-only, no subtype chips, no counts, no color coding
2. **Mod types all look identical in rows** — the kind palette exists in CSS but is barely visible in the table
3. **More Details sheet is generic** — it shows the same sections regardless of whether you're looking at a CAS CC or a script mod; the content depth is wasted
4. **View modes don't feel meaningfully different** — Casual/Seasoned/Creator differ in density but not in *intent*

---

## 1. Filter Redesign

### 1.1 Current State
- Type filter: bare `<select>` dropdown — one at a time, no chips
- Creator filter: dropdown only, no search
- Subtype: exists in facets but is hidden behind "More Filters" popover
- Watch status: pill chips in the toolbar strip row 2 — good
- No counts on any filter
- No color coding on type filters

### 1.2 Decisions

**Layout: Hybrid — Type chips in rail + dropdown refinements in More Filters**

Rationale: Simmers think in types first. Having CAS, Script, Gameplay visible as **color-coded chips** at the top of the rail (not hidden in a dropdown) is the single highest-impact change. Subtype filters go in a secondary chip row. Creator and confidence go in grouped dropdowns.

**Type chips (primary filter row)**
- Rendered as `library-kind-chip` — a custom chip element with a colored left-border and type icon
- `library-kind-chip` gets `border-left: 3px solid var(--kind-{type})` and a matching tinted background
- Counts appear as a badge: `library-kind-chip-count` in the top-right corner
- Active state: fills with `var(--kind-{type})` at 12% opacity, border brightens
- Inactive chips are muted (40% opacity text) so unselected types don't compete visually

Existing CSS type colors to use directly:
```css
--kind-cas: #e07bd3;      /* CAS — purple-pink */
--kind-script: #f0a500;   /* ScriptMods — amber */
--kind-gameplay: #4caf82; /* Gameplay — green */
--kind-buildbuy: #5b9bd5;  /* BuildBuy — blue */
--kind-override: #e05c5c;  /* OverridesAndDefaults — red */
--kind-poses: #9b7fe8;     /* PosesAndAnimation — violet */
--kind-presets: #4db6ac;   /* PresetsAndSliders — teal */
--kind-tray: #78909c;      /* Tray — blue-grey */
--kind-unknown: #607d8b;   /* Unknown — grey */
```

**New class: `library-kind-chip`**
```css
.library-kind-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.28rem 0.6rem 0.28rem 0.4rem;
  border: 1px solid var(--line);
  border-left-width: 3px;
  background: var(--surface-4);
  color: var(--text-soft);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.14s ease;
  position: relative;
}
.library-kind-chip:hover {
  background: var(--surface-hover);
  color: var(--text);
}
.library-kind-chip.is-active {
  color: var(--text);
  /* per-type color via CSS custom property on the element */
}
.library-kind-chip-count {
  font-size: 0.62rem;
  font-weight: 700;
  color: var(--text-dim);
  margin-left: auto;
}
.library-kind-chip.is-active .library-kind-chip-count {
  color: var(--text-soft);
}
```

Apply type-specific left-border and active fill via CSS attribute selector or utility classes:
```css
.library-kind-chip[data-kind="CAS"]     { border-left-color: var(--kind-cas); }
.library-kind-chip[data-kind="ScriptMods"] { border-left-color: var(--kind-script); }
.library-kind-chip[data-kind="Gameplay"]  { border-left-color: var(--kind-gameplay); }
.library-kind-chip[data-kind="BuildBuy"]  { border-left-color: var(--kind-buildbuy); }
.library-kind-chip[data-kind="OverridesAndDefaults"] { border-left-color: var(--kind-override); }
.library-kind-chip[data-kind="PosesAndAnimation"]    { border-left-color: var(--kind-poses); }
.library-kind-chip[data-kind="PresetsAndSliders"]   { border-left-color: var(--kind-presets); }
.library-kind-chip[data-kind="Tray*"]    { border-left-color: var(--kind-tray); }
.library-kind-chip[data-kind="Unknown"]  { border-left-color: var(--kind-unknown); }

.library-kind-chip.is-active[data-kind="CAS"]    { background: rgba(224, 123, 211, 0.12); border-color: var(--kind-cas); }
.library-kind-chip.is-active[data-kind="ScriptMods"] { background: rgba(240, 165, 0, 0.12); border-color: var(--kind-script); }
/* etc. */
```

**Subtype chips (secondary filter row)**
- Only visible when a primary type is active (e.g., when CAS is selected, show Hair, Clothing, Accessories, etc.)
- Rendered as `library-subtype-chip` — smaller than kind chips, no border-left color, pill shape
- Count badges: yes
- Horizontal scrollable row if many subtypes

**Creator filter: searchable dropdown**
- Replace plain `<select>` with a custom `library-creator-select` component
- Uses existing `facets.creators[]` array
- Inline search: as user types, list filters in place (no server round-trip)
- No search icon — just type-to-filter behavior in the dropdown input
- "All creators" remains at top
- Selected creator shows in the trigger button

**Confidence filter: segmented control (already exists, strengthen it)**
- Already implemented as `confidence-segmented` — fine, but move it more prominently (currently hidden in More Filters)
- In Creator view: show confidence segmented inline in the rail body
- In Casual view: hide entirely

**Watch status filter: already good, improve labeling**
- Current: `["All", "Has Updates", "Needs review", "Not Tracked", "Duplicates"]`
- Rename: "Has Updates" → "⚡ Has Updates", "Needs review" → "⚑ Needs review"
- Add count badges to each pill in the `LibraryFilterRail` watch row

**Duplicate filter: move to primary chip row**
- Currently buried in watchFilter — "duplicates" is a kind-agnostic filter
- Add a `library-dup-chip` pill next to type chips in the rail
- `library-dup-chip`: styled as `library-kind-chip` but with amber coloring

**Active filter indication**
- Each active filter chip gets `is-active` class + per-type color fill
- Rail header shows `N filters active` count (already exists, just reinforce)
- Toolbar strip: amber "Reset" button with count badge (already exists)
- Active filter chips in the rail are never hidden — they remain visible with full color

**Layout in LibraryFilterRail:**
```
┌─────────────────────────────────┐
│ Narrow Library    [×]           │
├─────────────────────────────────┤
│  247 shown    3 filters on      │
├─────────────────────────────────┤
│ STATUS                          │
│ [All] [⚡Updates] [⚑Review]     │
│ [Not Tracked] [Dupes]           │
├─────────────────────────────────┤
│ TYPE                       [▼]  │  ← dropdown for "Other"
│ [■ CAS 42] [■ Script 18]        │
│ [■ Gameplay 27] [■ B/B 12]      │
│ [■ Override 5] [■ Presets 3]    │
├─────────────────────────────────┤
│ SUBTYPE (shown when CAS active)  │
│ [Hair 12] [Clothing 28] [⚙×]    │
├─────────────────────────────────┤
│ CREATOR                         │
│ [___search creator___ ▼]        │
├─────────────────────────────────┤
│ CONFIDENCE (creator+)            │
│ [Any] [High] [Med] [Low]        │
├─────────────────────────────────┤
│ [Reset filters]  14 creators     │
└─────────────────────────────────┘
```

---

## 2. Mod-Type Differentiation

### 2.1 Row Presentation

The table already has a `type-accent-col` with a colored bar (`type-accent--{typeColor}`). The problem: it's a 4px left border in the first column and easy to miss. The differentiation needs to be stronger and more type-specific.

**Changes to `LibraryCollectionTable.tsx`:**

1. **Increase the type accent bar width**: 4px → 6px, increase opacity
2. **Add a type-color background tint to the entire row on hover**: `row:hover { background: color-mix(in srgb, var(--kind-{type}) 5%, var(--surface-hover)) }`
3. **Add a type-specific icon in the type pill**: use SVG icons per type instead of just the type label text
4. **In Creator view**: add a "type fact" column — `library-row-type-fact` — showing the most diagnostic single fact per type

**New supporting facts per type (Creator view):**

| Type | Fact shown | Format |
|---|---|---|
| CAS | Subtype + body category | `"Hair / Long"` |
| ScriptMods | Script namespace (short) | `"kybus.c Willow.WillowMod"` |
| Gameplay | Family tag | `"EP11 · Household"` |
| BuildBuy | Object type | `"Object / Lighting"` |
| OverridesAndDefaults | Override type | `"Tuning Override"` |
| PosesAndAnimation | Pose type | `"CAS Pose Pack"` |
| PresetsAndSliders | Preset type | `"Body Preset"` |
| TrayHousehold | Household name | `"The Pancakes 2.0"` |
| TrayLot | Lot type + name | `"Household · Pancakes"` |
| Unknown | Raw clue | `"[dbpf-package]"` |

**Badge treatments per type:**

| Type | Primary badge | Secondary badge | Color treatment |
|---|---|---|---|
| CAS | `type-pill` + subtype | confidence dot | Pink/violet tones |
| ScriptMods | version badge | `⚠` if low confidence | Amber/orange |
| Gameplay | EP/tags pill | health pill | Green |
| BuildBuy | source pill | health pill | Blue |
| Override | ⚑ warning badge | confidence dot | Red |
| Poses | pose type badge | — | Violet |
| Presets | preset type badge | — | Teal |
| Tray | "Tray" chip | "Disabled" if inactive | Blue-grey |
| Unknown | `?` badge | — | Grey |

### 2.2 Sidebar Summary Differentiation

`LibraryDetailsPanel.tsx` already has per-type data but shows everything generically. Changes:

**In the header** — the type pill (`library-type-pill`) should be larger in Creator view (add `is-prominent` modifier) and show the type-specific icon alongside the label.

**In the Snapshot section** — per-type fact emphasis:
- CAS: show `subtype` prominently, then `creator`
- ScriptMods: show `version` + `namespace count` prominently, then creator
- Gameplay: show `family hints` + `EP tags` prominently
- Tray: show `tray identity` (household/lot name) prominently, mark as "inactive" if disabled

**New class: `library-detail-type-hero`**
```css
.library-detail-type-hero {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.65rem;
  align-items: start;
  padding: 0.75rem;
  border: 1px solid var(--line);
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--kind-{type}) 8%, transparent), transparent 65%),
    var(--surface-3);
}
.library-detail-type-hero-icon {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid var(--line);
  background: var(--surface-4);
}
.library-detail-type-hero-label {
  font-size: 0.9rem;
  font-weight: 700;
}
.library-detail-type-hero-sublabel {
  font-size: 0.72rem;
  color: var(--text-soft);
  margin-top: 0.1rem;
}
```

### 2.3 Color Coding Map

The existing `--kind-*` palette in `globals.css` should be used as the canonical mapping, extended slightly:

```css
/* Primary type colors (left-border, pill bg, row tint) */
--kind-cas:        #e07bd3;  /* CAS */
--kind-script:     #f0a500;  /* ScriptMods */
--kind-gameplay:   #4caf82;  /* Gameplay */
--kind-buildbuy:   #5b9bd5;  /* BuildBuy */
--kind-override:   #e05c5c;  /* OverridesAndDefaults */
--kind-poses:      #9b7fe8;  /* PosesAndAnimation */
--kind-presets:    #4db6ac;  /* PresetsAndSliders */
--kind-tray:       #78909c;  /* Tray* */
--kind-unknown:    #607d8b;  /* Unknown */

/* Secondary (pill text, active chip text) */
--kind-cas-text:        #f0b8e8;
--kind-script-text:     #ffcc50;
--kind-gameplay-text:    #7dd4a8;
--kind-buildbuy-text:    #8bb8e0;
--kind-override-text:    #f08080;
--kind-poses-text:       #c4b0f0;
--kind-presets-text:     #80d4c8;
--kind-tray-text:        #90a8b3;
--kind-unknown-text:     #8090a0;

/* Health/attention overlay colors (not type colors) */
--health-good:     var(--accent);       /* #78f0a1 — green */
--health-warn:     var(--amber);         /* #f2c47b — amber */
--health-danger:   var(--danger);        /* #ff8484 — red */
--health-muted:    var(--text-dim);      /* grey */
```

Row tint on hover (type-specific):
```css
.library-table tbody tr:hover[data-kind="CAS"] {
  background: rgba(224, 123, 211, 0.06);
}
.library-table tbody tr:hover[data-kind="ScriptMods"] {
  background: rgba(240, 165, 0, 0.06);
}
/* etc. — apply via data-kind attribute or row model */
```

---

## 3. More Details Sheet Redesign

The clipping bug is fixed — now the sheet should be genuinely *rich and type-specific*.

### 3.1 Architecture Change

The current sheet uses a single `sections[]` array filtered per mode (health/inspect/edit). This should be replaced with **type-aware section templates** that select and order sections based on `selectedFile.kind`.

### 3.2 Section Templates by Type

#### CAS / CC
**Primary section: `cas-identity`**
- Subtype/category shown as a large hero badge at the top of the sheet lead
- `library-detail-sheet-lead` gets a `has-type-hero` modifier with type-specific gradient
- Embedded names: shown as clickable in-game name tags (like the current `ghost-chip` but styled distinctly as `cas-name-tag`)
- Set relationships: "Part of [SetName]" shown as a linked badge
- Swatch info: if swatch/thumbnail data is available, render a `cas-swatch-strip` — horizontal row of color swatches as small circles

```css
.cas-name-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.18rem 0.5rem;
  border: 1px solid var(--kind-cas);
  background: rgba(224, 123, 211, 0.1);
  color: var(--kind-cas-text);
  font-size: 0.72rem;
  font-weight: 600;
  border-radius: 3px;
}
.cas-swatch-strip {
  display: flex;
  gap: 0.28rem;
  flex-wrap: wrap;
}
.cas-swatch {
  width: 20px;
  height: 20px;
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 50%;
  background: var(--swatch-color);
}
```

**Section order for CAS:**
1. `cas-identity` — hero with subtype, embedded names, set
2. `file-facts` — file size, modified date, path (all views)
3. `cas-swatches` — swatch colors if available (power only)
4. `bundle-warnings` — safety notes, bundle info
5. `version-tracking` — if tracked
6. `path` (power only)

#### Script Mods
**Primary section: `script-identity`**
- Version/build shown as a prominent hero element: `library-script-version-hero`
- Script namespaces listed as a `script-namespace-list` — vertical list with folder icons
- Tracking capability shown prominently: `This mod can be tracked for updates / cannot be tracked`
- Source/tracking details: Provider label + URL

```css
.library-script-version-hero {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.5rem;
  align-items: center;
  padding: 0.6rem 0.7rem;
  border: 1px solid var(--kind-script);
  background: rgba(240, 165, 0, 0.08);
}
.library-script-version-label {
  font-size: 0.62rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--kind-script-text);
}
.library-script-version-value {
  font-size: 1rem;
  font-weight: 800;
  color: var(--text);
  font-family: "Cascadia Code", monospace;
}
.library-script-namespace-list {
  display: grid;
  gap: 0.3rem;
}
.library-script-namespace-item {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.32rem 0.48rem;
  border: 1px solid var(--line);
  background: var(--surface-4);
  font-size: 0.74rem;
  font-family: monospace;
  color: var(--kind-script-text);
}
```

**Section order for ScriptMods:**
1. `script-identity` — version hero + namespaces
2. `script-tracking` — can/cannot track, source provider
3. `file-facts` — size, modified, path
4. `bundle-warnings` — safety + parser warnings
5. `creator-learning` (power only)
6. `path` (power only)

#### Gameplay
**Primary section: `gameplay-identity`**
- Family/domain hints shown as EP-styled chips (`gameplay-ep-chip`)
- Resource summary as a compact list with type icons

```css
.gameplay-ep-chip {
  display: inline-flex;
  align-items: center;
  padding: 0.18rem 0.45rem;
  border: 1px solid var(--kind-gameplay);
  background: rgba(76, 175, 130, 0.1);
  color: var(--kind-gameplay-text);
  font-size: 0.7rem;
  font-weight: 700;
}
```

**Section order for Gameplay:**
1. `gameplay-identity` — EP chips + family hints
2. `resource-summary` — what package resources this contains
3. `creator-info` — creator + related content
4. `file-facts`
5. `bundle-warnings`
6. `path` (power only)

#### Tray / Household / Lot
**Primary section: `tray-identity`**
- Tray type prominently shown: Household / Lot / Room / Item
- Household/Lot name as the title
- Inactive/disabled context: if the tray item is from an inactive household, show a `tray-inactive-banner`
- Package relationships: linked files shown as a `tray-package-list`

```css
.tray-inactive-banner {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.48rem 0.6rem;
  border: 1px solid var(--amber-line);
  background: var(--amber-soft);
  color: var(--amber);
  font-size: 0.74rem;
}
.tray-package-list {
  display: grid;
  gap: 0.25rem;
}
.tray-package-item {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.32rem 0.45rem;
  border: 1px solid var(--line);
  background: var(--surface-4);
  font-size: 0.72rem;
}
```

**Section order for Tray types:**
1. `tray-identity` — type + name + inactive banner if applicable
2. `tray-packages` — related package files
3. `file-facts`
4. `path` (power only)

### 3.3 Sheet Layout Refinements

**Lead section redesign** — `library-detail-sheet-lead`:
- Currently: filename + type pill + creator chip + confidence badge, all stacked
- Redesign: two-column lead — left = filename + type hero, right = quick-meta stack (confidence, watch, source)
- For CAS: left column shows type hero with subtype as large label
- For ScriptMods: left column shows version hero

```css
.library-detail-sheet-lead {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.75rem;
  align-items: start;
  padding: 0.75rem;
  border: 1px solid var(--line);
  background: var(--surface-3);
}
.library-detail-sheet-lead.is-type-hero {
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--kind-{type}) 12%, transparent), var(--surface-3) 70%);
  border-color: color-mix(in srgb, var(--kind-{type}) 30%, var(--line));
}
.library-detail-sheet-lead-left {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
}
.library-detail-sheet-lead-right {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  align-items: flex-end;
}
```

---

## 4. View Differentiation Plan

### 4.1 Casual (beginner)

**Philosophy:** "Just show me what I need to know and let me play." No technical clutter. Trust the defaults.

**Filter rail:**
- Show only: Watch status pills + Type chips (minimal count badges)
- Hide: Confidence, Creator dropdown, Source
- Chip style: soft, rounded, minimal — `library-quick-chip` style
- Subtype chips: shown below type chips but smaller, muted until a type is selected

**Table rows:**
- Columns: [Type bar] [Checkbox] [Name] [Watch Status] [empty]
- Remove the "At a glance" / Clues column entirely
- Type pill shows type label only (no subtype, no icon)
- Confidence badge: hidden
- Issues indicator: show ⚑ only, no tooltip on hover (too technical)
- Hover: soft green tint

**Sidebar (LibraryDetailsPanel):**
- Only two sections visible: Snapshot + More actions
- Snapshot: Creator, Type, Watch status only — no subtype, no confidence, no format
- Care section: plain-language one-liner summary, no tags
- "More details" button: labeled "See more" — styled as a gentle secondary action

**More Details sheet:**
- Sheet title: "About this file"
- Sections shown: type identity (simplified), file facts (size + modified only), care (warnings only)
- Hide: creator learning, category override, path, inspection signals
- All sections collapsed by default

### 4.2 Seasoned (standard/power)

**Philosophy:** "I know what I'm doing. Give me useful filters and good row clues." The workhorse view.

**Filter rail:**
- Full type chip row with counts + color coding
- Subtype chips visible (contextual to selected type)
- Creator dropdown with search
- Watch status pills with improved icons
- Confidence segmented control (inline in rail body)
- Source dropdown

**Table rows:**
- Columns: [Type bar] [Checkbox] [Name + Type pill + badges] [Watch + Health + Dupe pills] [At a glance: 2 facts]
- Type pill: label + small type icon
- Row hover: type-specific color tint
- Confidence badge: visible with ✓/⚠/? symbol
- Issues indicator: ⚑ with short label on hover

**Sidebar:**
- Snapshot: Creator, Type, Subtype, Watch, Confidence (5 lines)
- Care section: shows warning tags inline (seasoned users should see them without opening the sheet)
- Duplicates section: visible
- "More details" + "Open in Updates" buttons

**More Details sheet:**
- Sheet title changes per type (e.g., "Script mod details", "CAS item details")
- Sections per type as defined in Section 3
- Version signals shown with confidence percentages
- Creator learning and category override sections accessible

### 4.3 Creator (power)

**Philosophy:** "Give me everything. I need full metadata, diagnostics, and complete filter control."

**Filter rail:**
- All Seasoned filters plus: Source dropdown inline (not in More Filters)
- Sort options include "Confidence" and "Recently Modified"
- Duplicate filter as a primary chip

**Table rows:**
- Columns: [Type bar] [Checkbox] [Name + Type pill + all badges] [Watch + Health + Dupe pills] [Clues: up to 3 facts]
- Supporting facts column: shows the most diagnostic fact per type (see Section 2.1 table)
- Confidence: always shown with percentage
- Row background: subtle type-color left-border even when not hovered
- Issues: ⚑ + full label

**Sidebar:**
- Full snapshot including file format
- All insight tags shown (creator hints, version hints, embedded names, family hints, resource summary, script namespaces)
- Parser warnings uncapped (no "+N more")
- "Creator Learning" + "Type Override" sections visible inline (not just in sheet)
- "Open folder" button

**More Details sheet:**
- All sections per type (Section 3)
- Version signals with full confidence breakdown
- Full path shown in dedicated `path` section
- Creator learning fully editable inline
- Category override fully editable inline
- Bundle relationships shown

---

## 5. Color System Reference

### 5.1 Complete Color Palette

```css
/* ── Type Identity Colors ────────────────────────────────────── */
:root {
  /* Primary */
  --kind-cas:        #e07bd3;
  --kind-script:     #f0a500;
  --kind-gameplay:   #4caf82;
  --kind-buildbuy:   #5b9bd5;
  --kind-override:   #e05c5c;
  --kind-poses:      #9b7fe8;
  --kind-presets:    #4db6ac;
  --kind-tray:       #78909c;
  --kind-unknown:    #607d8b;

  /* Text variants (on dark bg) */
  --kind-cas-text:        #f0b8e8;
  --kind-script-text:     #ffd04d;
  --kind-gameplay-text:   #7dd4a8;
  --kind-buildbuy-text:   #8bb8e0;
  --kind-override-text:   #f08080;
  --kind-poses-text:      #c4b0f0;
  --kind-presets-text:    #80d4c8;
  --kind-tray-text:       #90a8b3;
  --kind-unknown-text:    #80909a;

  /* Fills (12–15% opacity for backgrounds) */
  --kind-cas-fill:        rgba(224, 123, 211, 0.12);
  --kind-script-fill:     rgba(240, 165, 0, 0.12);
  --kind-gameplay-fill:   rgba(76, 175, 130, 0.12);
  --kind-buildbuy-fill:   rgba(91, 155, 213, 0.12);
  --kind-override-fill:   rgba(224, 92, 92, 0.12);
  --kind-poses-fill:      rgba(155, 127, 232, 0.12);
  --kind-presets-fill:    rgba(77, 182, 172, 0.12);
  --kind-tray-fill:       rgba(120, 144, 156, 0.12);
  --kind-unknown-fill:    rgba(96, 125, 139, 0.12);

  /* ── Health / State Colors ─────────────────────────────── */
  --health-good:     var(--accent);      /* #78f0a1 — green */
  --health-good-soft: rgba(120, 240, 161, 0.1);
  --health-warn:     var(--amber);        /* #f2c47b — amber */
  --health-warn-soft: rgba(242, 196, 123, 0.1);
  --health-danger:   var(--danger);       /* #ff8484 — red */
  --health-danger-soft: rgba(255, 132, 132, 0.1);
  --health-muted:    var(--text-dim);

  /* ── Confidence Colors ─────────────────────────────── */
  --confidence-high:   #78f0a1;
  --confidence-medium: #f2c47b;
  --confidence-low:    #ff8484;
}
```

### 5.2 Usage Rules

1. **Type colors** are for identity only — they tell you what kind of mod it is. Never use a type color to communicate health state.
2. **Health colors** are for status — whether a mod is up to date, needs review, is disabled. Never use health colors to indicate mod type.
3. **Confidence colors** are for classification quality — high/medium/low confidence in the classification itself, not in the mod's quality.
4. **Tray items** always get `--kind-tray` left-border. If inactive, overlay `--health-warn` as a dashed top border.
5. **Script mods with low confidence** get both `--kind-script` type color AND `--confidence-low` badge color simultaneously — these are independent signals.

---

## 6. CSS Class Recommendations

### New classes to add to `globals.css`

**Filter Rail:**
- `.library-kind-chip` — kind filter chip with per-type color via `data-kind`
- `.library-kind-chip-count` — count badge inside kind chip
- `.library-kind-chip.is-active[data-kind="X"]` — per-type active state
- `.library-subtype-chip` — smaller subtype chip, pill shape
- `.library-creator-select` — custom searchable dropdown wrapper
- `.library-creator-search-input` — inline search input inside dropdown
- `.library-dup-chip` — duplicate filter chip (amber colored)
- `.library-filter-section-label` — section header inside rail
- `.watch-filter-chip` (already exists as `.watch-filter-chip`) — reinforce with count badge support

**Table Rows:**
- `.library-table tbody tr[data-kind="X"]` — per-type row hover tint
- `.type-accent` (exists) — increase width from 4px to 6px
- `.type-pill` (exists) — add type icon support
- `.library-row-type-fact` — single diagnostic fact shown in Creator clues column
- `.library-duplicate-badge` (exists) — refine style
- `.library-confidence-badge` (exists) — add `confidence--high/medium/low` modifiers
- `.library-issues-badge` (exists) — improve visibility

**Sidebar:**
- `.library-detail-type-hero` — type-specific hero card in sheet lead
- `.library-detail-type-hero-icon` — icon container in hero
- `.library-detail-type-hero-label` — main label
- `.library-detail-type-hero-sublabel` — secondary label
- `.library-details-panel.is-tray-item` (exists) — add tray-specific styling
- `.cas-name-tag` — in-game name tag for CAS items
- `.cas-swatch-strip` — swatch color row
- `.cas-swatch` — individual swatch circle
- `.gameplay-ep-chip` — EP/tag chip for Gameplay
- `.library-script-version-hero` — version hero for script mods
- `.library-script-version-label` — "VERSION" label
- `.library-script-version-value` — monospace version number
- `.library-script-namespace-list` — namespace list
- `.library-script-namespace-item` — namespace item
- `.tray-inactive-banner` — inactive tray item banner
- `.tray-package-list` — tray related packages
- `.tray-package-item` — individual tray package

**Sheet:**
- `.library-detail-sheet-lead.is-type-hero` — type-hero variant of lead
- `.library-detail-sheet-lead-left` — left column of lead
- `.library-detail-sheet-lead-right` — right column of lead

**Confidence:**
- `.confidence-badge` (exists) — add `.confidence--high`, `.confidence--medium`, `.confidence--low`
- `.confidence-dot` — small dot indicator
- `.confidence-pill` — pill-shaped confidence label

---

## 7. Top 10 Implementation Priorities

### Priority 1: Kind Filter Chips with Color Coding + Counts
**File:** `LibraryFilterRail.tsx` + `globals.css`
**Why first:** This is the single highest-impact visual change. Simmers identify mods by type — making that the most prominent filter, not hidden in a dropdown, transforms how the rail feels.
**Effort:** Medium — new component + CSS, existing facets data structure already has all needed data.
**Deliverable:** Type chips row in rail with per-kind left-border color, count badges, active state fill.

### Priority 2: Row Type Differentiation — Row Hover Tint + Type Pill Icons
**File:** `LibraryCollectionTable.tsx` + `globals.css`
**Why second:** The table is the primary workspace. Making type visually prominent in each row makes scanning 100+ mods feel manageable rather than overwhelming.
**Effort:** Low — CSS changes to existing classes + adding `data-kind` attributes to rows.
**Deliverable:** Row hover with type-color tint, type pills with small icons (SVG inline).

### Priority 3: Subtype Chips — Contextual to Selected Type
**File:** `LibraryFilterRail.tsx`
**Why third:** Type filtering is coarse; subtype filtering is how you find "all the hair CC from this creator." Must come right after kind chips.
**Effort:** Low-Medium — new chip row component, visibility gated by selected kind.
**Deliverable:** Secondary chip row showing only relevant subtypes when a type is active.

### Priority 4: More Details Sheet — Type-Aware Lead Section
**File:** `LibraryDetailSheet.tsx` + `globals.css`
**Why fourth:** The sheet is where deep work happens. Making the lead section type-specific (CAS hero vs. Script version hero) communicates "we know what kind of mod this is" before the user reads a single section.
**Effort:** Medium — new type-hero CSS + conditional rendering in sheet lead.
**Deliverable:** CAS items show subtype hero; Script mods show version hero; Gameplay shows EP chips; Tray shows tray identity.

### Priority 5: Script Mod Sheet — Version + Namespaces Prominently Shown
**File:** `LibraryDetailSheet.tsx` + `LibraryScreen.tsx` + `globals.css`
**Why fifth:** Script mods are the highest-stakes category (can break games). Showing version + namespaces prominently in the sheet is genuinely useful and not available anywhere else.
**Effort:** Medium — new CSS for version hero + namespace list.
**Deliverable:** `library-script-version-hero` at top of sheet, namespace list below it.

### Priority 6: Creator Filter — Searchable Dropdown
**File:** `LibraryFilterRail.tsx` + new component
**Why sixth:** With large libraries (1000+ mods) and many creators, the current flat dropdown is barely usable. Search-within-dropdown is a standard expectation.
**Effort:** Medium — new component with keyboard navigation.
**Deliverable:** Creator dropdown that filters as you type.

### Priority 7: View Differentiation — Casual Sidebar Stripped to 2 Sections
**File:** `LibraryDetailsPanel.tsx` + CSS view mode rules
**Why seventh:** Casual users currently see too many sections in the sidebar for a "just pick something and see it" workflow. Two sections (Snapshot + More) with simplified content is the right mental model.
**Effort:** Low — conditional rendering + CSS hiding.
**Deliverable:** Casual: 2 sidebar sections. Seasoned: full current panel. Creator: adds Creator Learning + Type Override inline.

### Priority 8: Type-Specific Row Facts in Creator View (Clues Column)
**File:** `LibraryCollectionTable.tsx` + `libraryDisplay.ts` + `globals.css`
**Why eighth:** Creator view's "Clues" column currently just shows generic facts. Making it show the most *diagnostic* single fact per type turns the table into a genuine investigation tool.
**Effort:** Medium — update `buildSupportingFacts` + new CSS for `library-row-type-fact`.
**Deliverable:** Clues column shows e.g. "Hair / Long" for CAS, "v2.3.1" for script mods, "EP11 · Family" for gameplay.

### Priority 9: Sheet Section Order — Type-Specific Templates
**File:** `LibraryScreen.tsx` (sheet section building) + `LibraryDetailSheet.tsx`
**Why ninth:** The current sheet shows the same sections in the same order regardless of type. A CAS item and a script mod should feel like different documents.
**Effort:** Medium — refactor section building to a per-kind template.
**Deliverable:** `getSheetSectionsForKind(kind, userView)` function returning ordered sections.

### Priority 10: Confidence Filter Prominence — Segmented Control Visible in Seasoned Rail
**File:** `LibraryFilterRail.tsx` + `globals.css`
**Why tenth:** Confidence is a powerful filter for cleanup workflows ("show me everything I'm unsure about") but is hidden in the More Filters popover. Moving it to the rail body for Seasoned+ makes it a first-class filter.
**Effort:** Low — move the existing `confidence-segmented` component from More Filters popover to rail body.
**Deliverable:** Confidence segmented visible inline in Seasoned + Creator filter rail.

---

## Implementation Notes

1. **Phase order matters**: Priorities 1-3 are all in the filter rail and can share state. Build them as a unified `FilterRailV2` component if possible, then swap it in.
2. **The `data-kind` attribute on `<tr>` elements** is already the right hook for CSS-only type differentiation. Add it if not already present, then write all the hover/fill CSS against it.
3. **Type color CSS variables are already in `globals.css`** — use them as `var(--kind-{type})` rather than hardcoding hex values anywhere.
4. **The sheet sections architecture in `LibraryScreen.tsx`** is already data-driven (`DockSectionDefinition[]`). The refactor to per-kind templates is additive — build `getSheetSectionsForKind()` as a new pure function, keep existing section definitions, wire them via kind switch.
5. **Creator view flag is `userView === "power"`** (not `"creator"`) — verify all new conditional logic uses the correct constant.
6. **Test the Creator view filter rail** — it currently has `showAdvancedFilters: userView === "power"` but confidence segmented is hidden from it too. Priority 10 fixes this.
