# SimSuite Icon & Asset Plan

_Created: Phase 5aj (2026-04-21)_
_Status: Ready for visual identity work_

---

## How to Use This Plan

- **A — Must design now**: Core identity, high-frequency, currently generic or weak
- **B — Should design soon**: Useful polish, recurring UI language
- **C — Can defer**: Low value, currently acceptable

Generated visuals should:
- Avoid text inside images
- Be adaptable for later vectorization (SVG preferred)
- Keep style cohesive with SimSuite's dark/sophisticated aesthetic
- NOT use generated images as fake extracted game previews
- Generated visuals are for app identity / UI assets / fallback illustrations only

---

## A — Must Design Now

| Asset | Type | Where Used | Notes |
|---|---|---|---|
| **App logo / favicon** | SVG + 16/32/192px PNG | App window, taskbar, installer | Core brand identity. Design first — everything else follows. |
| **Grid view icon** | 16×16 / 20×20 SVG | Toolbar view toggle (Grid mode) | Replaces `lucide-react Grid3X3`. High-frequency, core product gesture. |
| **List view icon** | 16×16 / 20×20 SVG | Toolbar view toggle (List mode) | Replaces `lucide-react LayoutList`. High-frequency. |
| **Folder view icon** | 16×16 / 20×20 SVG | Toolbar view toggle (Folder mode) | Replaces `lucide-react Folder`. High-frequency. |
| **Density control icon** | 16×16 / 20×20 SVG | New Phase 5aj density rail button | Replaces the old slider. Could be a dot-grid or small chart icon. |
| **File type: CAS** | 20×20 SVG | Fallback icon, type pills | Replaces current inline SVG. Professional pack/icon aesthetic. |
| **File type: BuildBuy** | 20×20 SVG | Fallback icon, type pills | Replaces current inline SVG. Home/building motif. |
| **File type: ScriptMods** | 20×20 SVG | Fallback icon, type pills | Replaces current inline SVG. Code/terminal aesthetic. |
| **File type: Tray** | 20×20 SVG | Fallback icon, type pills | Replaces current inline SVG. Box/tray motif. |
| **Section: Library** | 20×20 SVG | Sidebar nav | Identifies the Library section in nav rail |
| **Section: Organize** | 20×20 SVG | Sidebar nav | Identifies Organize section |
| **Section: Downloads** | 20×20 SVG | Sidebar nav | Identifies Downloads/tray section |
| **Folder tree: open/closed** | 16×16 SVG pair | Folder tree rows | Chevron + folder. Needs open and closed states. |

---

## B — Should Design Soon

| Asset | Type | Where Used | Notes |
|---|---|---|---|
| **Advanced filter icon** | 16×16 SVG | Filter toggle button | Replaces `lucide-react SlidersHorizontal`. More thematic. |
| **Sort icon** | 16×16 SVG | Sort dropdown | Replaces `lucide-react ArrowUpDown`. Cleaner, consistent style. |
| **Watch status: up to date** | 16×16 SVG | Status pills | ✓ checkmark or green dot variant |
| **Watch status: update available** | 16×16 SVG | Status pills | ↻ refresh/arrow icon |
| **Watch status: not tracked** | 16×16 SVG | Status pills | ○ circle — neutral |
| **Watch status: warning** | 16×16 SVG | Status pills | ⚠ triangle or ! |
| **Confidence: high** | 16×16 SVG | Sidebar confidence badge | Strong signal icon |
| **Confidence: medium** | 16×16 SVG | Sidebar confidence badge | Moderate signal icon |
| **Empty state: no files** | 48×48 SVG illustration | Library empty state | Friendly, on-brand illustration when library is empty |
| **Empty state: no search results** | 48×48 SVG illustration | Search empty state | Illustration for zero-result searches |
| **Type badge leading icons** | 12×12 SVG | Type pills in sidebar/detail | Small leading icon per type kind |

---

## C — Can Defer

| Asset | Status | Reason |
|---|---|---|
| Generic `X` (close/dismiss) | OK — `lucide-react X` is clean | Standard, well-executed |
| Generic `Search` | OK — `lucide-react Search` is standard | Standard |
| Generic `Eye` (view action) | OK — functional | Standard |
| Generic `ExternalLink` | OK | Standard |
| Generic `PanelLeftClose` | OK | Functional |
| Version badge icons | Acceptable — ghost chips work | Low frequency |
| Duplicate warning icons | Acceptable — text tags work | Low frequency |

---

## Icon Style Guide

When designing for SimSuite:

- **Dark theme first**: icons should read clearly on `#0d0d1a` to `#1a1a2e` backgrounds
- **Subtle glow on active states**: use `rgba(249, 115, 22, 0.6)` orange glow for active/selected states (matches SimSuite's orange accent `#f97316`)
- **Line style preferred**: 1.5–2px stroke weight, rounded caps
- **Neutral fills for inactive**: `rgba(255,255,255,0.5–0.7)` for inactive state
- **White/bright for active**: `#ffffff` or `#f97316` for active state
- **Avoid thick blocky fills**: prefer line/stroke icons
- **Category-specific motifs**: CAS → person/hanger/clothing; BuildBuy → house; Scripts → terminal/code bracket; Tray → box

---

## Suggested Design Order

1. **App logo** — blocks all branding work; must come first
2. **View mode icons** (Grid + List + Folder) — daily-driver toolbar icons, highest frequency
3. **File type icons** (CAS, BuildBuy, Script, Tray) — sidebar, recurring
4. **Density control icon** — Phase 5aj new element, needs to match toolbar style
5. **Section nav icons** (Library, Organize, Downloads) — sidebar
6. **Folder tree open/closed** — folder tree is a core navigation surface
7. **Watch status icons** — sidebar + detail sheet, frequent power-user surface
8. **Empty state illustrations** — last, lower urgency

---

## Design Tool Recommendations

- **Vector first**: Figma or Affinity Designer for SVG output
- **Export**: SVG + PNG (16, 32, 192px for app icon; 16/20px for UI icons)
- **Consistency check**: generate all icons at once to ensure unified style
- **SimSuite aesthetic**: dark, clean, minimal — avoid gradients, text, or photorealism
