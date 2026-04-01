# Library Redesign — Design Spec

## Layout Decision: Top-bar-first, railless

**Chosen over alternatives because:**
- The filter rail consumed 248–360px of horizontal space that belongs to the mod list
- Filters (type, creator, source) are inherently flat/1D — they don't need a persistent vertical panel
- A top toolbar makes filter state always visible without sacrificing list real estate
- The rail can collapse to 0 but that leaves a gap; better to eliminate it entirely for the default view
- Quick chips for watch filters are already top-positioned and work well — they stay

**Structure:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Toolbar: [Search ────────────] [Type▾] [Creator▾] [Sort▾] [⚙ Adv] │
│ Chips:   All | Has Updates | Needs Review | Not Tracked | Duplicates│
│ Summary: 1,247 files · 892 tracked · 23 updates · 8 need review    │
├─────────────────────────────────────────────────────────────────────┤
│ ⬤ Filename.package          ● Current  ✓     The Pinnacle · CAS   │
│   ScriptMod · ts4script     ⚠ Update   ⚑      Maxis · High conf   │
│ ⬤ AnotherFile.package      ● Current         AnotherCreator      │
│ ...                                                               │
├─────────────────────────────────────────────────────────────────────┤
│ [Inspector panel — right side, collapsible]                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Toolbar Spec

**Row 1 — primary toolbar:**
- `Search` — text input, grows to fill available space, placeholder: "Search by name or creator..."
- `Type` — dropdown select, shows "All Types" or selected type label, options from facets.kinds
- `Creator` — dropdown select, shows "All Creators" or selected creator, options from facets.creators
- `Sort` — dropdown, options: Name (A–Z), Name (Z–A), Creator, Recent, Type
- `Advanced` — icon button (⚙ or sliders icon), opens a popover with:
  - Source (Mods / Tray / All)
  - Min confidence slider (Low / Medium / High)
  - Subtype filter
  - Active filter count badge

**Row 2 — quick chips (watch filter):**
- All / Has Updates / Needs Review / Not Tracked / Duplicates
- Compact pill style, active state is filled
- No text labels beyond the chip text

**Row 3 — summary strip:**
- One-line text: "X files · Y tracked · Z updates · N need review · M duplicates"
- Smaller text, muted color
- Only shown when filters are active or when data is loaded

## Table Row Spec

### Visual structure per row
```
┌──────────────────────────────────────────────────────────────────────┐
│ [●] Filename.package                           [StatusPill] [Dup] │
│     [TypeLabel] · [CreatorName] · [At-a-glance fact]    [ConfBar] │
└──────────────────────────────────────────────────────────────────────┘
```

**Left type indicator:** 4px colored left-border or colored dot — color by `kind`
**Column 1 (filename):** filename + extension + source badge (tray = 🔖 or "tray" tag)
**Status pills:** watch status pill (colored by tone) + duplicate icon if `has_duplicate`
**Row 2 line:** TypeLabel · Creator · Supporting fact (type-specific) · Confidence bar
**Hover:** subtle background highlight, row cursor pointer

### Type color system (CSS variables)
```css
--type-cas: #a855f7;        /* purple — CAS items */
--type-script: #f59e0b;     /* amber — script mods */
--type-gameplay: #22c55e;   /* green — gameplay */
--type-buildbuy: #f97316;   /* orange — build/buy */
--type-override: #64748b;  /* slate — overrides/defaults */
--type-poses: #ec4899;     /* pink — poses/animation */
--type-presets: #06b6d4;   /* cyan — presets/sliders */
--type-tray: #94a3b8;      /* muted blue-gray — tray items */
--type-unknown: #6b7280;   /* gray — unknown */
```

### Watch status pill colors
```css
--status-current: #22c55e (green)
--status-update: #f59e0b (amber)
--status-review: #ef4444 (red)
--status-not-tracked: #6b7280 (gray)
```

### Supporting facts by type (row 2)
| Kind | Fact to show | Example |
|---|---|---|
| CAS | Subtype | "hair · Maxis" |
| ScriptMods | Confidence level | "High confidence · Maxis" |
| Gameplay | Creator | "Maxis · Gameplay" |
| BuildBuy | Source location | "tray · Build/Buy" |
| OverridesAndDefaults | Confidence | "Low confidence · Overrides" |
| PosesAndAnimation | Creator + pack | "Maxis · Pose Pack" |
| PresetsAndSliders | Body area (from subtype) | "Face · Presets" |
| TrayHousehold/Lot/Room | Tray badge + creator | "🔖 tray · Maxis" |
| Unknown | Source location | "mods · Unknown" |

### Confidence bar
- Thin horizontal bar (3px) at right of row 2
- Green = high (≥0.8), amber = medium (≥0.55), red = low (<0.55)
- Or: colored left border matching confidence

### Duplicate indicator
- Small ⚑ icon in status pill area when `has_duplicate === true`
- Color: muted amber/orange

## Advanced filters popover
- Opens below the Advanced button
- Contains: source toggle (Mods / Tray / All), min confidence slider, subtype free-text
- Shows active filter count as badge on the Advanced button
- Framer Motion animated open/close

## Summary strip
- Position: below quick chips, above table
- Text only, no icons, muted color
- Format: "X files · Y tracked · Z updates · N need review · M duplicates · K disabled"
- `librarySummary` from `getLibrarySummary` — already computed by backend

## Inspector (right panel)
- Remains largely as-is
- Add type-specific quick-action suggestion at top when relevant
- Tray items: prominent "Enable" action suggestion
- Script mods: show update status prominently

## Empty / loading states
- **Empty (no rows):** Show centered empty state with icon and message: "No mods match your filters" with a Reset filters button
- **Loading:** Skeleton rows (3-5) with animated shimmer — not a spinner
- **Error:** Red banner above table with error message and retry button

## Components to change
1. `LibraryScreen.tsx` — restructure state: remove `libraryFiltersCollapsed`/`libraryRailWidth`, add `advancedFiltersOpen`
2. `LibraryTopStrip.tsx` — redesign as compact toolbar + chips + summary (3 logical rows)
3. `LibraryFilterRail.tsx` — **replace entirely** with a new `LibraryToolbarFilters.tsx` popover component
4. `libraryDisplay.ts` — update `buildLibraryRowModel` to return richer supporting facts + typeColor + sourceIcon
5. `LibraryCollectionTable.tsx` — update row rendering to use new richer row model
6. `globals.css` — add `.library-toolbar*`, `.library-chip*`, `.library-summary*`, `.type-*` color classes, enhanced row CSS
7. Remove filter rail from WorkbenchRail in LibraryScreen (or keep it 0-width with `width={0}`)

## Implementation order
1. Update `buildLibraryRowModel` + CSS type colors — low risk, immediate visual improvement
2. Redesign `LibraryTopStrip` + new `LibraryToolbarFilters` popover
3. Update `LibraryCollectionTable` rows
4. Update `LibraryScreen` to remove left rail
5. Test in running app
6. Commit
