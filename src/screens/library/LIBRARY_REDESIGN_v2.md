# Library UI Redesign v2 — Comprehensive Design
## SimSort Phase 5+ Swatch/Preview Architecture

**Date:** 2026-04-10
**Status:** Design document — not yet implemented
**Scope:** Grid view cards, list row adaptations, inspector redesign, all mod types and edge cases

---

## 1. What This Document Covers

This is a thorough redesign pass for SimSort's Library / My CC screens. It covers:

- How each of the 10 mod types should look in grid view and list view
- How to handle mods that belong in the same pack (grouped files)
- How to handle mods without any preview content
- Tray items (placed, misplaced)
- How variants within a single package should be communicated
- The inspector redesign for when real swatches arrive
- Content strategy: what to show as the "preview" for each type before image extraction exists
- Honest fallback states for every edge case
- Responsive behavior
- View mode interaction with the existing three user views (Casual / Seasoned / Creator)

---

## 2. The Mod Types — Complete Inventory

Every mod type has a different "primary content signal" — the thing a simmer most wants to know about it. This is the foundation of the redesign.

### 2.1 CAS — Create-a-Sim Items

**Files:** `.package` files containing CAS parts (hair, clothing, skintones, makeup, etc.)

**Available signals:**
- `embeddedNames`: Item identifier strings from inside the package — e.g. `"NSW_Skinblend"`, `"TINT_05"`, `"HAIR_MeshName"`. These are the internal names of the actual in-game items. **This is the primary content signal.**
- `resourceSummary`: Type-level summary — e.g. `"4 Skintone resources"`, `"3 CASPart resources"`
- `subtype`: Human category — `"hair"`, `"tops"`, `"bottoms"`, `"fullBody"`, `"skintone"`, `"makeup"`, `"accessories"`, `"shoes"`
- `kind`: Always `"CAS"`
- `creator`: Creator name

**What a simmer wants to know:** "Is this the hair/shirt I want?" — the embeddedNames answer this directly by showing the actual item names. A package called `庁vr_S4PileOfCrap_Hair_v2.package` actually contains items named `NSW_SandyLong` — the name IS the content.

**Swatch status:** DDS format when real extraction exists. Multiple swatches per package possible (different colors/variants of the same item). **Swatch area on card: YES, high priority.**

**Grid card content strategy:**
- Show first 4 `embeddedNames` as chips
- Show `+N more` overflow badge if > 4
- If no embeddedNames: show `resourceSummary` or `"CAS content"` as fallback
- Subtype label below the chips (e.g. "hair", "tops")

**Variants within CAS:** A single package can contain multiple colorways or variants of the same item (e.g. 8 colorways of one hair). The embeddedNames list captures all of them. Showing 4 with overflow is the right balance — it shows variety without overwhelming.

---

### 2.2 ScriptMods — Script Packages

**Files:** `.ts4script` (ZIP archives containing Python scripts) or `.package` files with script bytecode

**Available signals:**
- `scriptNamespaces`: The primary identifier — e.g. `"deaderpool.mccc"`, `"scrumples.core"`. **This is the primary content signal.**
- `versionSignals`: Version extracted from filename or file content — e.g. `"9.1.6"`, `"2024.03"`
- `versionHints`: Weaker version clues
- `embeddedNames`: Almost never present for script mods — this is bytecode, not content
- `resourceSummary`: Occasionally `"S4Script Area"` type entries
- `kind`: Always `"ScriptMods"`
- `creator`: Creator name

**What a simmer wants to know:** "Which script mod is this?" — the namespace answers this. A file named `MCCC_9_1_6_Script.zip` contains namespace `deaderpool.mccc`. Showing the namespace is more useful than the filename.

**Swatch status:** No swatches possible — script files contain no images. **Swatch area on card: placeholder only ("script mod — no preview").**

**Grid card content strategy:**
- Show first 3 `scriptNamespaces` as chips (e.g. `deaderpool.mccc`, `deaderpool.mccc.cycles`)
- If no namespaces: show `versionLabel` as a badge (e.g. `v9.1.6`)
- If neither: show `"Script mod"` + fallback text
- **Never show "No preview available" for ScriptMods — the namespace IS the content**

**Edge case — multiple scripts in one file:** A script package can contain multiple namespaces. Showing the first 3 with overflow handles this gracefully.

---

### 2.3 BuildBuy — Build/Buy Objects

**Files:** `.package` files containing buy/debug objects (furniture, building elements, decor)

**Available signals:**
- `resourceSummary`: Content count — e.g. `"6 build/buy items"`, `"1 object"`. **This is the primary content signal.**
- `embeddedNames`: Object instance names (less commonly present)
- `subtype`: Category — `"lighting"`, `"decor"`, `"seating"`, `"surfaces"`, `"electronics"`, etc.
- `kind`: Always `"BuildBuy"`
- `creator`: Creator name

**What a simmer wants to know:** "What is this object and how many items are in the file?" — the resource count answers this. A single `.package` can contain multiple catalog entries from the same creator.

**Swatch status:** JPEG thumbnails when real extraction exists (~0xF5F2D94D resource type). Usually one preview per object. **Swatch area on card: YES, high priority.**

**Grid card content strategy:**
- Show `resourceSummary` as a prominent label (e.g. "6 build/buy items")
- Below it, show the `subtype` if available (e.g. "lighting", "seating")
- If no resourceSummary: show subtype or `"Build/Buy content"` as fallback
- embeddedNames as secondary chips if present (uncommon for build/buy)

---

### 2.4 OverridesAndDefaults — Default Replacements

**Files:** `.package` files that override EA's default content

**Available signals:**
- `resourceSummary`: Override type — e.g. `"9 default replacement items"`, `"3 override resources"`
- `embeddedNames`: The specific objects being overridden (sometimes available)
- `subtype`: What category is overridden — less commonly present
- `kind`: Always `"OverridesAndDefaults"`
- `creator`: Creator name

**What a simmer wants to know:** "What does this override, and how many things does it replace?" — the resource summary answers this.

**Swatch status:** DDS or JPEG depending on what's being overridden. Can be swatches if overriding CAS items. **Swatch area on card: conditional.**

**Grid card content strategy:**
- Show `resourceSummary` as the primary label (e.g. "9 default replacements")
- Show creator below
- If no resourceSummary: show `"Override package"` as label

---

### 2.5 PosesAndAnimation — Pose Packs and Animation Sets

**Files:** `.package` files containing poses or animation data

**Available signals:**
- `subtype`: Pose type — `"pose_pack"`, `"animation_set"`, `"animation"`, `"pose"`, freeform
- `creator`: Creator name
- `embeddedNames`: Pose names if stored in the package (sometimes)
- `resourceSummary`: Occasionally
- `kind`: Always `"PosesAndAnimation"`

**What a simmer wants to know:** "What type of poses/animation is this?" — the subtype answers this.

**Swatch status:** No standard swatch format. Pose preview images are sometimes stored as custom thumbnails but there is no standard resource type. **Swatch area on card: placeholder ("poses — no standard preview").**

**Grid card content strategy:**
- Show `subtype` as the primary label (e.g. "pose pack", "animation set")
- Show creator below
- If subtype is empty: show `"Pose / Animation content"` as fallback

---

### 2.6 PresetsAndSliders — Sliders and Build/Buy Presets

**Files:** `.package` files containing CAS sliders or build/buy presets

**Available signals:**
- `subtype`: Body area or preset type — `"face"`, `"body"`, `"build"`, `"build_buy_preset"`, freeform
- `creator`: Creator name
- `resourceSummary`: Occasionally
- `kind`: Always `"PresetsAndSliders"`

**What a simmer wants to know:** "What is this preset for?" — the subtype answers this.

**Swatch status:** No standard swatch. **Swatch area on card: placeholder.**

**Grid card content strategy:**
- Show `subtype` as primary label (e.g. "face slider", "body preset")
- Show creator below
- If subtype is empty: show `"Preset / Slider content"` as fallback

---

### 2.7 TrayHousehold — Saved Households

**Files:** `.household` format or `.package` in Tray folder

**Available signals:**
- `bundleName`: Household name (from file or family hints) — **primary content signal**
- `groupedFileCount`: How many files in this household save
- `familyHints`: Family name clues from inside the file
- `kind`: `"TrayHousehold"`
- `sourceLocation`: `"tray"` (correct) or `"mods"` (misplaced)
- `isMisplaced`: True if in Mods folder

**What a simmer wants to know:** "Whose household is this?" — the household name answers this.

**Swatch status:** JPEG thumbnail of the household in CAS or in the lot view. **Swatch area on card: YES.**

**Tray badge states:**
- Correctly placed: `tray · disabled` in muted style
- Misplaced: `⚠ misplaced in Mods` in attention/warning style — more prominent

**Grid card content strategy:**
- Show `bundleName` (or family hint) as primary label, large
- Show `"Household"` subtype label below
- Show `groupedFileCount` as badge (e.g. "4 files") if > 1
- Misplaced: prominent warning treatment
- Never show "No preview available" — the household name IS the content

---

### 2.8 TrayLot — Saved Lots

**Files:** `.lot` format or `.package` in Tray folder

**Available signals:**
- `bundleName`: Lot name — **primary content signal**
- `groupedFileCount`: How many files in this lot save
- `familyHints`: Lot name clues
- `kind`: `"TrayLot"`
- `sourceLocation`: `"tray"` or `"mods"` (misplaced)
- `isMisplaced`: True if in Mods folder

**What a simmer wants to know:** "Which lot is this?" — the lot name answers this.

**Swatch status:** JPEG thumbnail of the lot from the build/buy catalog view. **Swatch area on card: YES.**

**Grid card content strategy:**
- Show `bundleName` (or family hint) as primary label
- Show `"Lot"` subtype label below
- Show `groupedFileCount` as badge if > 1
- Misplaced: prominent warning treatment

---

### 2.9 TrayRoom — Saved Rooms

**Files:** `.room` format or `.package` in Tray folder

**Available signals:**
- `bundleName`: Room name — **primary content signal**
- `groupedFileCount`: How many files
- `kind`: `"TrayRoom"`
- `sourceLocation`: `"tray"` or `"mods"` (misplaced)

**Swatch status:** JPEG thumbnail of the room. **Swatch area on card: YES.**

**Grid card content strategy:**
- Show `bundleName` as primary label
- Show `"Room"` subtype label
- Show `groupedFileCount` if > 1
- Misplaced: prominent warning treatment

---

### 2.10 TrayItem — Individual Tray Objects

**Files:** `.package` in Tray folder containing a single placed object

**Available signals:**
- `bundleName`: Item name
- `familyHints`: Item clues
- `kind`: `"TrayItem"`
- `sourceLocation`: `"tray"` or `"mods"` (misplaced)
- `isMisplaced`: True if in Mods folder

**What a simmer wants to know:** "What is this tray item?"

**Swatch status:** JPEG thumbnail of the object. **Swatch area on card: YES.**

**Grid card content strategy:**
- Show `bundleName` or `"Tray item"` as primary label
- Misplaced treatment prominent

---

### 2.11 Unknown — Unclassified Files

**Files:** `.package` files that don't match any known pattern

**Available signals:**
- `resourceSummary`: What the parser found
- `creator`: From path or inference
- `kind`: `"Unknown"`

**What a simmer wants to know:** "What is this and should I be concerned?"

**Grid card content strategy:**
- Show `resourceSummary` if available (e.g. "1 CASPart resource")
- Show `"Unknown type"` as fallback
- Confidence bar shows how uncertain the classification is
- ⚑ warning badge if parserWarnings exist

---

## 3. Grouped Files (Packs) — How to Handle

When multiple files belong to the same pack, they share a `bundleName` and have a `groupedFileCount`.

### Current state (from source):
- `bundleName: string | null` — the pack name
- `groupedFileCount?: number | null` — how many files in the group
- `hasDuplicate: boolean` — flagged if appears in duplicate pairs

### The grouping problem in grid view

If a hair pack has 8 files (one per colorway), showing 8 nearly identical cards is:
1. Visually overwhelming
2. Confusing (are these duplicates? Are they variants?)
3. Wasteful of grid space

### Recommended approach: Pack Indicator Pattern

**Option A — Collapsed pack (recommended for large groups ≥ 4 files):**
- Show only the first card normally
- Add a `library-card--is-pack-head` modifier
- Show a `+N files` badge on the card (e.g. `+7 files`)
- Other pack members shown in a collapsed state below or accessible via expand
- Clicking the pack badge opens an inline expansion showing all members

**Option B — Expanded pack (default for small groups < 4 files):**
- Show all cards with a shared `library-card--in-pack` modifier
- First card gets a "Pack: [bundleName]" label
- Subsequent cards get a "Part of [bundleName]" label
- Visual connector (subtle left border or pack color stripe)

**Decision criteria:**
- Groups of 1–3 files: expanded (Option B) — not confusing, shows variety
- Groups of 4+ files: collapsed with pack badge (Option A) — avoids overwhelm
- User can toggle to see full pack on demand

### Pack indicator in list view

The list already has `⚠ Duplicate` badge. For grouped files that aren't duplicates:
- Add a `📦 Part of "[bundleName]"` fact in the supporting facts area
- Show `groupedFileCount` (e.g. "8 in pack") in the facts column
- Grouped files sort together (already the case if sorted by bundle)

---

## 4. The "No Preview Available" Problem

This is the most important honest edge case. We need a per-type fallback, not a generic message.

### Per-type fallback hierarchy:

| Type | Fallback 1 | Fallback 2 | Fallback 3 |
|---|---|---|---|
| CAS | `resourceSummary` | `"CAS content"` | Never show generic empty |
| ScriptMods | `versionLabel` | `"Script mod"` | Never empty |
| BuildBuy | `subtype` | `"Build/Buy content"` | Never empty |
| OverridesAndDefaults | `resourceSummary` | `"Override package"` | Never empty |
| PosesAndAnimation | `subtype` | `"Pose / Animation"` | Never empty |
| PresetsAndSliders | `subtype` | `"Preset / Slider"` | Never empty |
| TrayHousehold | `bundleName` | `"Household"` | Never empty |
| TrayLot | `bundleName` | `"Lot"` | Never empty |
| TrayRoom | `bundleName` | `"Room"` | Never empty |
| TrayItem | `bundleName` | `"Tray item"` | Never empty |
| Unknown | `resourceSummary` | `"Unknown type"` | ⚑ warning state |

**Key principle: Every card always has something to show. "No preview available" should never appear as the primary content.**

The CSS class `.library-card-empty-preview` should only be used as a last-resort visual state, and only for types where we genuinely have zero content signals.

---

## 5. Revised LibraryCardModel

The current model needs to be extended to handle the per-type content strategy:

```typescript
interface LibraryCardModel {
  // Identity (always shown)
  id: number;
  title: string;           // filename
  kind: string;
  typeLabel: string;
  typeColor: TypeColor;

  // Status (always shown in header)
  creatorLabel: string;
  isTray: boolean;
  isMisplaced: boolean;
  watchStatusLabel: string;
  watchStatusTone: "calm" | "attention" | "muted";
  healthLabel: string | null;
  healthTone: "attention" | "muted" | null;
  hasDuplicate: boolean;
  hasIssues: boolean;
  confidenceLevel: "high" | "medium" | "low";

  // ── Grouping ───────────────────────────────────────────────
  isGrouped: boolean;         // true if bundleName is set
  groupedCount: number;        // total files in pack
  bundleName: string | null;   // pack name

  // ── Content signals — ONE of these is always set ───────────
  // CAS: item names as chips
  casNames: string[];              // embeddedNames, max 4
  casNamesOverflow: number;        // total - 4
  // ScriptMods: namespaces as chips
  scriptNamespaces: string[];      // scriptNamespaces, max 3
  scriptNamespaceOverflow: number;
  scriptVersionLabel: string | null;
  // BuildBuy / Overrides: resource summary
  contentSummary: string | null;   // resourceSummary[0]
  // Generic: subtype
  subtype: string | null;
  // Tray: bundle/household name
  trayIdentityLabel: string | null;
  // Version signal (for ScriptMods, also shown elsewhere)
  versionLabel: string | null;

  // ── The raw row ────────────────────────────────────────────
  row: LibraryFileRow;
}
```

**Design principle:** Exactly ONE of `{casNames, scriptNamespaces, contentSummary, trayIdentityLabel}` is the primary content signal, chosen by kind. The card renderer knows which to show.

---

## 6. Grid Card — Revised Layout

### Card anatomy (top to bottom):

```
┌──────────────────────────────────────────┐
│ [TYPE PILL]        [WATCH STATUS] [BADGE] │  ← Header (always)
│                                          │
│  ─── CONTENT AREA (type-specific) ───   │  ← Primary signal
│  [chip] [chip] [chip] [+N]              │    (see Section 7)
│  or resource summary or tray identity     │
│                                          │
├──────────────────────────────────────────┤
│ Filename.package                         │  ← Title (always)
│ Creator name          v1.2.3 (if set)   │  ← Footer
│ [TRAY BADGE if applicable]               │
└──────────────────────────────────────────┘
│ [CONFIDENCE BAR]                         │  ← Bottom edge
└──────────────────────────────────────────┘
```

### Content area height strategy:
- Minimum height: 56px (enough for 2 lines of chips or 1 summary line)
- Maximum height: 96px (enough for 4 chips + overflow line)
- If content is empty (never happens with the fallback hierarchy): show a slim placeholder with type icon

### Card header badges — expanded set:
- `library-type-pill` — always shown, type color
- `library-health-pill` — watch status, color by tone
- `⚑` warning badge — if `hasIssues`
- `Duplicate` badge — if `hasDuplicate`
- `⚠ misplaced` badge — if `isMisplaced` (more prominent than warning badge)
- `📦 +N` pack badge — if `isGrouped && groupedCount > 1`

### Misplaced tray badge (special treatment):
```css
.library-card-misplaced-badge {
  display: inline-block;
  padding: 0.1rem 0.4rem;
  background: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: #ef4444;
  font-size: 0.62rem;
  font-weight: 600;
  border-radius: 4px;
}
```
This is more prominent than the standard tray badge because misplaced items need attention.

---

## 7. Per-Type Card Content Rendering

### 7.1 CAS — Show chips

```
│ [NSW_Skinblend] [TINT_05]                │
│ [SandyLong] [+3]                         │
```
- Up to 4 `embeddedNames` chips
- If overflow: `+N more` chip in muted style
- Below chips: `subtype` label if available (e.g. "hair")

### 7.2 ScriptMods — Show namespace chips or version

```
│ [deaderpool.mccc] [mccc.tray]            │
│ v9.1.6                                   │
```
- Up to 3 `scriptNamespaces` chips
- If no namespaces: show `versionLabel` as a badge (e.g. `v9.1.6`)
- If neither: show `"Script mod"` + `"No namespace detected"`

### 7.3 BuildBuy — Show resource summary + subtype

```
│ 6 build/buy items                        │
│ lighting                                 │
```
- `resourceSummary` as prominent line (not a chip — too long for chip)
- `subtype` as secondary label below

### 7.4 OverridesAndDefaults — Show resource summary

```
│ 9 default replacements                   │
│ [CreatorName]                           │
```

### 7.5 PosesAndAnimation — Show subtype

```
│ pose pack                                │
│ [CreatorName]                           │
```

### 7.6 PresetsAndSliders — Show subtype

```
│ face slider                              │
│ [CreatorName]                           │
```

### 7.7 Tray items — Show bundle/identity

```
│ Household: The Smith Family              │
│ 4 files                                 │
│ ⚠ misplaced in Mods                     │  ← if misplaced
│ tray · disabled                         │  ← if correctly placed
```

- `bundleName` or family hint as primary label (large, prominent)
- `groupedFileCount` as secondary if > 1
- Misplaced: show `⚠ misplaced in Mods` badge (prominent red)
- Correctly placed: show `tray · disabled` (muted)

### 7.8 Unknown — Show what was detected

```
│ 1 CASPart resource                      │
│ Unknown type                           │
│ [CreatorName]              ⚑           │
```

---

## 8. List Row — Grouping Adaptations

The current row model already handles most of this. The additions needed:

### Grouped files in list:

Current facts column shows supporting facts. For grouped files:
- Add `"📦 N in pack"` to `supportingFacts` when `groupedFileCount > 1`
- This appears naturally in the facts column without changing column structure
- For `isMisplaced` tray items: `"⚠ misplaced"` replaces `"tray"` in facts

### ScriptMods in list:

Current row already shows namespace via `summarizeScriptScopeForUi`. Already handled.

### Tray items in list:

Current row shows `trayIdentity` info. Already handled by `buildSupportingFacts`.

---

## 9. Inspector Redesign

### Current structure:
```
┌─────────────────────────────────────────────────┐
│ [Eyebrow: Inspect]                             │
│ [Title: File facts and deeper clues]           │
│ [Subtitle]                                     │
├────────────────────────┬────────────────────────┤
│ Selected              │ Creator: Maxis         │
│ Hairstyle_v2.package  │ Confidence: 95%       │
│ CAS / Hair            │                        │
│ [Version badge]       │                        │
│ [Content badge]       │                        │
├────────────────────────┴────────────────────────┤
│ [DockSectionStack with tabs: Inspect|Health|Edit] │
└─────────────────────────────────────────────────┘
```

### Proposed: Inspector with preview area

**When real swatches exist (Phase 2+):**

```
┌─────────────────────────────────────────────────────────────┐
│ [Eyebrow] [Title]                              [X Close]   │
├─────────────────────────────────┬───────────────────────────┤
│ Selected                        │ ┌─────────────────────┐   │
│ Filename.package                │ │                     │   │
│ CAS / Hair                      │ │   [SWATCH TILE]     │   │ ← Preview
│ v2.1                            │ │   256×256           │   │   area
│ [Content badge]                 │ │   DDS extracted     │   │
│                                 │ └─────────────────────┘   │
│ Creator: Maxis (from file)      │                           │
│ Confidence: 95%                 │ [8 colorways]            │ ← Swatch
│                                 │ [chip][chip][chip]...     │   strip
├─────────────────────────────────┴───────────────────────────┤
│ [Inspect] [Health] [Edit]                                   │
│ ...DockSectionStack content...                              │
└─────────────────────────────────────────────────────────────┘
```

**Swatch tile specifications:**
- Fixed size: 200×200px in the lead area (right column)
- Below the tile: swatch strip if multiple swatches (horizontal scroll)
- Swatch strip: 48×48px thumbnail chips, scroll horizontally
- Label on swatch tile: `"Preview extracted"` (evidence badge: Extracted)
- If placeholder: `"Preview not available"` (muted)

**Without real swatches (Phase 1):**

The inspector lead area stays as-is. The content strip below in the Inspect tab (`buildSheetContentsSection`) already shows embeddedNames and scriptNamespaces intelligently. This is fine — it's already good.

What we ADD in Phase 1:
- A condensed content preview strip in the lead area, below the filename and badges, showing the same type-specific content as the grid card
- E.g. for CAS: `[NSW_Skinblend] [TINT_05] [+3]` as a horizontal chip strip
- E.g. for ScriptMods: `[deaderpool.mccc]` namespace chip + `v9.1.6` version badge
- This bridges the gap between the card preview and the detailed inspector content

### Inspector lead area — Phase 1 addition:

```
┌─────────────────────────────────────────────────────────────┐
│ Selected                        │ Content preview             │
│ Hairstyle_v2.package            │ [NSW_Skinblend][TINT_05] │
│ CAS / Hair                      │ [+3 more]                  │
│ [Version badge] [Content badge] │                            │
│                                 │ "Preview: not yet avail." │
│ Creator: Maxis (from file)      │                            │
│ Confidence: 95%                 │                            │
└─────────────────────────────────┴─────────────────────────────┘
```

The "Preview: not yet available" message uses the muted empty preview style and is labeled honestly — not a fake swatch placeholder.

---

## 10. Trust and Evidence Labeling

### Evidence badge system (already exists):
- `Extracted` — came directly from parsing the file
- `Derived` — inferred from file content (e.g. family hints)
- `Inferred` — guessed from path or context (e.g. folder-based creator)

### Swatch-specific trust:

| Swatch state | Label | When to use |
|---|---|---|
| Real DDS/JPEG swatch extracted | `Preview: extracted` | When we decode and display real image data |
| Generated placeholder | `Preview: not yet available` | When no swatch data exists yet |
| Content preview (text) | `Content preview` (no evidence badge) | When showing embeddedNames as text |

**Critical rule:** Never show a placeholder swatch tile that looks like a real image. The placeholder must be obviously a placeholder — muted background, no fake image frame, clear label.

---

## 11. View Modes and Progressive Disclosure

### Three user views and their swatch/content behavior:

| View | Card content | Inspector content | Badge depth |
|---|---|---|---|
| **Casual (beginner)** | Type pill + title + creator + primary content signal only | Filename + type + creator (no evidence badges) | Minimal |
| **Seasoned (standard)** | + version label + watch status + health indicator | + version evidence + content details | Standard |
| **Creator (power)** | + all namespace/name chips + full overflow + grouped file count | + full evidence chains + diagnostics section | Full |

### What Casual view hides:
- Namespace chips (ScriptMods): show only `v{version}` badge, no chips
- embeddedNames overflow: `+N more` is hidden — show only first 2
- `⚑` warning badge: always shown (safety-relevant)
- Misplaced badge: always shown (safety-relevant)
- Creator confidence suffix: hidden

---

## 12. Responsive Behavior

### 13" laptop (1280×800 viewport):
- Grid: `repeat(auto-fill, minmax(200px, 1fr))` — 4-5 cards per row
- Card title truncates at 1 line
- Chips truncate at 2 lines with `+N` overflow
- Comfortable density

### 15" laptop (1920×1080):
- Grid: `repeat(auto-fill, minmax(220px, 1fr))` — 6-7 cards per row
- More breathing room between cards

### 4K (3840×2160):
- Grid: `repeat(auto-fill, minmax(240px, 1fr))` — up to 14 cards per row
- Consider `max(240px, 20vw)` to cap card width at ~270px
- Otherwise cards stretch uncomfortably wide

### List view:
- No changes needed — already responsive
- Column widths already set with min/max constraints

### Very narrow (mobile/tablet):
- Grid collapses to 1 column (single card per row)
- List view stays functional but with horizontal scroll on some columns

---

## 13. Empty States

### Grid view — empty results:
```
┌──────────────────────────────────────────────────────┐
│                                                      │
│      🔍 Nothing matches these filters right now.      │
│                                                      │
│              [ Reset filters ]                       │
│                                                      │
└──────────────────────────────────────────────────────┘
```
- Centered, full-width within the grid container
- Same copy as list view for consistency
- `Reset filters` button calls `onResetFilters()`

### Card with genuinely no content (last resort):
```
│ [TYPE PILL]           [WATCH STATUS]                 │
│                                                      │
│  ⚠ No content detected                              │
│  This file may be empty or use an unrecognized      │
│  format.                                            │
│                                                      │
│ Filename.package                                     │
│ Creator · Unknown type                               │
└──────────────────────────────────────────────────────┘
```
This state should be extremely rare — only for genuinely empty Unknown-type files.

---

## 14. The Transition: From Content Preview to Real Swatches

When real DDS/JPEG extraction is added in Phase 2+:

1. **Card content area** — swap text chips for the swatch image tile:
   ```
   BEFORE (Phase 1):          AFTER (Phase 2+):
   [chip][chip][chip]         ┌────────────┐
   [+3]                       │  [SWATCH]  │
                              └────────────┘
                              [chip][chip]...
   ```
   The text preview remains below the swatch as secondary context.

2. **Inspector lead area** — swatch tile replaces content preview strip:
   ```
   BEFORE: Content preview    AFTER: Swatch tile
   [NSW_Skinblend]           ┌──────────────┐
   [+3 more]                  │              │
                              │  [DDS/JPEG]  │
                              │              │
                              └──────────────┘
                              8 colorways ↓
   ```

3. **Type-specific swatch strip** — shown below the main swatch tile:
   - CAS: horizontal strip of colorway swatches (scrollable)
   - Build/Buy: single thumbnail
   - Tray items: single thumbnail
   - ScriptMods: no swatch strip (script files have no images)

4. **Graceful degradation** — if swatch extraction fails for a given file, fall back to the Phase 1 text preview. Never show a broken image icon.

---

## 15. Implementation Priority

### Phase 1 (this session):
1. ✅ `LibraryThumbnailGrid.tsx` — built, needs revision for per-type content
2. ✅ `LibraryCardModel` in `libraryDisplay.tsx` — needs extension with grouping fields
3. ✅ View toggle in `LibraryTopStrip.tsx` — built
4. ✅ CSS for grid and cards — built, needs additions for grouping + misplaced + per-type chips
5. **Revised:** Per-type content strategy in card rendering
6. **Revised:** Grouped file badges on cards
7. **Revised:** Misplaced tray badge (prominent)
8. **New:** Inspector content preview strip (Phase 1 addition to lead area)

### Phase 2 (future):
1. Real JPEG thumbnail extraction for Build/Buy items
2. Swatch tile in inspector lead area
3. Swatch strip for CAS colorways
4. Collapsed pack expansion UI

### Phase 3 (deferred):
1. DDS decoding for CAS swatches (HIGH complexity — DXT codec needed)
2. Full inspector redesign with swatch preview area

---

## 16. Open Questions

1. **Should ScriptMods without namespaces show a version badge prominently?** — Yes, `versionLabel` should be shown in the content area when no namespaces exist. It's the most useful signal available.

2. **Should the grid view show `relativeDepth`?** — Not in the card itself. Depth is a diagnostic signal — belongs in the inspector diagnostics section, not on the browsing card.

3. **Should grouped items be visually connected with a shared border or background?** — Yes, `library-card--in-pack` with a shared top border color. The pack head gets `library-card--pack-head`. This is a Phase 1 polish addition.

4. **Should the card title ever show the `bundleName` instead of the filename?** — Only for tray items where `bundleName` is the actual household/lot name. For other types, filename is the primary identifier and should always be shown as title.

5. **Should variants be collapsed into a single card showing the variant count?** — Not automatically. Let the simmer decide. A group of 8 colorways should show with the pack indicator pattern (Option A), but individual variant cards are still clickable.

---

## 17. Summary of Changes Needed

### LibraryCardModel additions:
- `isGrouped: boolean`
- `groupedCount: number`
- `bundleName: string | null`
- `casNames: string[]`
- `casNamesOverflow: number`
- `scriptNamespaces: string[]`
- `scriptNamespaceOverflow: number`
- `scriptVersionLabel: string | null`
- `contentSummary: string | null`
- `trayIdentityLabel: string | null`
- Remove/generalize: `embeddedNames`, `resourceSummary`, `totalEmbeddedNames`

### LibraryThumbnailGrid rendering changes:
- Use kind to select which content block to render
- CAS: chip list + overflow
- ScriptMods: namespace chips + version
- BuildBuy: resource summary line
- Overrides: resource summary line
- Poses/Presets: subtype label
- Tray items: bundle name + group badge + misplaced treatment
- Unknown: resource summary + ⚑ warning

### CSS additions needed:
- `.library-card--is-pack-head` + `.library-card--in-pack` modifiers
- `.library-card-misplaced-badge` (red, prominent)
- `.library-card-pack-badge` ("📦 +N files")
- `.library-card-subtype-label` (secondary text below content)
- `.library-card-namespace-chips` (smaller, tighter than name chips)
- `.library-card-content-summary` (prominent line for build/buy)
- Swatch tile placeholder styles for inspector

### Inspector changes:
- Lead area: add content preview strip (Phase 1)
- When real swatches exist: swatch tile right column + swatch strip below

---

_This document is a living design spec. Update as implementation reveals new constraints._
