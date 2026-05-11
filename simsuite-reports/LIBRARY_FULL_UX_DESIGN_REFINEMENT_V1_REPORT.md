# Library Full UX Design Refinement v1

## Audit note before coding

Branch: `codex/library-full-ux-design-refinement-v1`

Latest proof folder reviewed:

- `output/desktop/library-proof/2026-05-10T23-16-40-647Z`

Screenshots reviewed:

- `library-row-sidebar-polish.png`
- `02-library-grid-view.png`
- `03-library-folder-view.png`
- `04-library-detail-sheet.png`
- `05-library-preflight-detail-mccc.png`
- inspector default, resized, narrower, and collapsed captures
- Updates bridge context capture
- Duplicates bridge context capture
- Casual, Seasoned, and Creator mode captures

Skills used as design guidance:

- `anthropic-frontend-design`
- `frontend-design`
- `design-taste-frontend`
- `high-end-visual-design`
- `impeccable-frontend-design`
- `redesign-existing-projects`
- `frontend-testing-debugging`

The design guidance is being applied inside the existing React/Tauri stack. No framework or dependency change is planned.

## View inventory

- List view: working, but still visually mechanical. Rows are readable after the previous polish sprint, but status cues compete with file identity and the status column repeats "No update source" with too much visual weight.
- Grid view: working and thumbnail-first, but cards feel under-labeled at rest. The reveal layer carries too much of the identity work.
- Folder view: working, including Mods/Tray roots and Direct files wording, but the root state feels sparse and needs stronger explanation of what the view is for.
- Left nav rail: working, but dense. Labels are readable after earlier fixes, but the rail still feels utilitarian rather than intentionally composed.
- Toolbar/search/sort/filter controls: working. The repeated `All` filters are ambiguous because one applies to type and the other applies to signals.
- Right inspector: working, resizable, and collapsible. It still repeats row facts more than it explains what the selected file means or what a user should do next.
- More Details/detail sheet: working. It has useful evidence, but the sheet reads as a dense data dump in places.
- Safe Action Preflight: working and cautious. The detail view is still visually dense and should lead with action-oriented context.
- Updates bridge context: working in proof.
- Duplicates bridge context: working in proof.
- Needs Review context: covered by existing proof.
- Empty/loading/error states: present, but some are generic and do not always tell the user what to try next.
- User modes/density modes: Casual, Seasoned, and Creator were captured in proof. The UI should stay useful across all three without hiding the main Library actions.

## Information placement matrix

| Information | List row | Grid card | Folder view | Right inspector | More Details | Preflight | Updates | Duplicates | Needs Review |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| File name | Primary identity | Primary/rest identity | File row/card identity | Header | Header + metadata | Context only | Focus context | Focus context | Focus context |
| Creator | Compact fact when known | Reveal/secondary | Secondary if useful | At a glance | Metadata | Context if relevant | Context if relevant | Context if relevant | Context if relevant |
| Type/category | Small row chip | Small card chip | Summary and row chip | Header/at a glance | Metadata | Context only | Context only | Context only | Context only |
| Subtype | Secondary row line if useful | Reveal only | Secondary row line | At a glance | Metadata | Context if relevant | Not primary | Not primary | Not primary |
| Source root/folder | Secondary context | Reveal only | Primary browsing context | At a glance | Full path/evidence | Context if relevant | Not primary | Not primary | Not primary |
| Thumbnail/preview | Optional icon only | Primary | Optional for file rows | Preview area | Preview/evidence | Not primary | Not primary | Not primary | Not primary |
| Update state | One calm cue | One calm cue | Count/cue if useful | Explanation + route | Evidence | Caution if relevant | Workflow owner | Not primary | Context if relevant |
| Duplicate hint | One calm cue | One calm cue | Count/cue if useful | Explanation + route | Evidence | Caution if relevant | Not primary | Workflow owner | Context if relevant |
| Problem signal | One calm cue | One calm cue | Count/cue if useful | Explanation + route | Evidence | Action-oriented caution | Not primary | Not primary | Workflow owner |
| Related hint | Usually not row unless strongest cue | Reveal only | Summary if useful | Explanation | Evidence | Caution if relevant | Not primary | Not primary | Context if relevant |
| Confidence | Icon/brief only if useful | Reveal only | Not primary | Explanation | Evidence | Evidence if relevant | Source confidence if relevant | Evidence if relevant | Evidence if relevant |
| Full path | No | No | Folder path | Wrapped value | Full evidence | Only if action needs it | No | No | No |
| Package/tray metadata | No | No | Summary only | Key facts | Detailed evidence | Only if caution-relevant | No | No | Evidence if relevant |
| Actions | Row affordance only | Reveal/selection actions | Folder/file actions | Main command area | Context actions | Action routes | Workflow actions | Workflow actions | Workflow actions |
| Caution/preflight notes | Cue only | Cue only | Summary only | Compact explanation | Evidence | Primary owner | Context if update-related | Context if duplicate-related | Primary owner |
| Deeper evidence | No | No | No | Summary only | Primary owner | Only action-relevant | Workflow-specific | Workflow-specific | Workflow-specific |

## Redundancy audit

- `No update source`: useful in rows as a quick cue, but should be visually quiet. The inspector should explain that SimSuite does not know where to check, not merely repeat the label.
- `Possible duplicate`: useful in rows and grid cards as a cue. The inspector and Preflight should explain review/comparison without implying safe deletion.
- Type/category/subtype: useful in rows, grid cards, and inspector, but the row should keep it compact and More Details should hold the deeper metadata.
- Creator: useful as a row fact when available. It should not become a filler value when unknown.
- "All" filters: unnecessary ambiguity. The type filter and signal filter need different visible labels.
- Preflight cautions: useful reinforcement, but the detail view should avoid repeating the entire inspector.
- Route context cards: useful if they confirm the focused file. They should not pretend to be a full workflow by themselves.

## Visual hierarchy audit

- Weak hierarchy: rows have better clipping now, but file identity, status, and facts still compete.
- Noisy badges/chips: status badges are truthful but still too blocky. `No update source` should become a quiet cue, while duplicate/review cues can carry slightly more emphasis.
- Unclear grouping: toolbar filter rows need group labels or clearer first-chip text.
- Cramped spacing: preflight/detail sheet content is dense and the sheet background allows too much visual competition from the Library behind it.
- Excessive empty space: folder root view has a large empty body without enough helpful orientation.
- Poor selected state: acceptable after previous sprint, but row content can still feel detached from inspector context.
- Accessibility concern: filter controls are buttons, but repeated visible labels reduce clarity.
- Performance risk: no new heavy rendering should be introduced. Keep virtualization, avoid list animations, and keep inspector explanations derived from selected file data only.

## User workflow audit

- Scanning list for attention: works, but status cues should be quieter and ordered by usefulness.
- Searching by name/creator: works. Search needs to remain visually prominent without consuming the whole toolbar.
- Filtering by type/signal: works, but labels should distinguish type filters from signal filters.
- Switching list/grid/folder: proof-covered. The visual language should feel consistent across views.
- Selecting a file: works. Inspector should tell the user what the selected cues mean and what to do next.
- Opening More Details: proof-covered. The sheet should feel structured, not raw.
- Opening Safe Action Preflight: proof-covered. The panel should lead with calm next-step language.
- Following Updates bridge: proof-covered. The Library should only use this as a bridge, not as the update workflow owner.
- Following Duplicates bridge: proof-covered. Wording must remain "Possible duplicate" and review-focused.
- Reviewing a Tray item: works. Tray storage context should stay honest.
- Browsing folders/direct files: works. Folder view should make Direct files feel normal, not suspicious.
- Resizing/collapsing inspector: proof-covered. The center stage should stay readable at narrower widths.
- Smaller laptop screen: needs proof after this pass, ideally at 1366 x 768 and/or 1440 x 900.

## Initial classification

- Row hierarchy issue: confirmed.
- Status badge issue: confirmed.
- Metadata chip issue: mostly acceptable, but could be calmer.
- Right inspector issue: confirmed, mostly repetition and weak explanation.
- Left nav issue: minor; dense but currently acceptable for v1.
- Folder sidebar issue: acceptable, but folder root content needs better orientation.
- Detail sheet issue: confirmed, primarily density and background readability.
- Preflight panel issue: confirmed, primarily dense action/evidence presentation.
- Responsive issue: needs proof after changes.
- Accessibility issue: repeated `All` filter labels reduce clarity.
- Performance risk: avoid adding work inside virtualized rows.
- Future work, not v1: full design-system migration, full nav redesign, and provider/workflow expansion.

## Pre-existing worktree changes

At sprint start, the worktree had unrelated Home/status changes:

- `src/screens/HomeScreen.tsx`: Home module card heading spacing tweak.
- `src/styles/globals.css`: Home hero metric color/line-height tweak.
- `SESSION_HANDOFF.md`: pre-existing Home/UI polish notes.
- `docs/IMPLEMENTATION_STATUS.md`: pre-existing Home/UI polish notes.

These are not part of this Library UX sprint. If `src/styles/globals.css` is edited for Library CSS, only Library-related hunks should be staged for this sprint.

## Implementation summary

This sprint kept the existing React/Tauri stack and refined the Library UI in place. No backend schema, provider, update persistence, scraping, download, replacement, or cleanup behavior was added.

### What changed

- Toolbar filters now distinguish `All types` from `All signals`, with quiet group labels for type and signal filters.
- List rows keep file identity first, show a small thumbnail/fallback without source-text overlays, and limit row cues to the strongest trust-safe signals.
- `No update source` is styled as a quieter cue. `Possible duplicate` and review cues stay visible without claiming confirmed duplicate or safe deletion.
- Grid cards now show a resting name/identity label instead of relying only on hover/reveal, and fallback thumbnail swatches no longer cover the title zone.
- Folder view root and folder states now explain Mods/Tray roots and `Direct files` in plain language.
- The right inspector now adds a `What this means` section for selected cues instead of only repeating row labels.
- Detail sheet and Safe Action Preflight copy were tightened to say what to check before changing files, without dependency or safe-delete claims.
- Detail/preflight sheet surfaces were made more opaque so background Library text does not compete with the active panel.
- Desktop proof now captures `library-full-ux-refinement.png` and verifies 1366 x 768 and 1440 x 900 Library geometry.

### Thumbnail decisions

- List rows: small real thumbnails render when available; otherwise, type fallback placeholders render without pretending to be a preview.
- Grid cards: thumbnail/fallback remains the main card surface, with resting identity labels for scanning.
- Folder view: Direct files use the same file thumbnail/fallback logic as Library rows; folder icons remain for folders only.
- Inspector/detail: missing previews are normal and still shown as `No preview available`, not as an error.

### Wording decisions

Kept or added careful wording:

- `No update source`
- `Possible duplicate`
- `Review suggested`
- `Stored in Tray`
- `Direct files`
- `SimSuite has limited information here`

Avoided:

- `confirmed duplicate`
- `safe to delete`
- `safe to replace`
- `missing mesh`
- `missing dependency`
- `definitely outdated`
- `official source found`
- broad dependency/requires language

### Dependencies

None. The sprint used existing React, TypeScript, CSS, proof scripts, and test tooling.

### Files changed

- `src/screens/library/LibraryTopStrip.tsx`: clarified filter labels and groups.
- `src/screens/library/LibraryCollectionTable.tsx`: cleaned row thumbnail handling and list empty state copy.
- `src/screens/library/LibraryThumbnailGrid.tsx`: added resting grid identity labels.
- `src/screens/library/FolderContentPane.tsx`: added folder guidance and better Direct files empty copy.
- `src/screens/library/LibraryDetailsPanel.tsx`: added selected-cue explanations in the inspector.
- `src/screens/library/LibraryDetailSheet.tsx`: softened detail sheet guidance around changing files.
- `src/screens/library/actionPreflight.tsx`: removed stronger dependency/safety phrasing and made guidance action-oriented.
- `src/styles/globals.css`: Library-only visual hierarchy, thumbnail, chip, inspector, folder, detail sheet, and preflight styling.
- `scripts/desktop/desktop-library-proof.mjs`: added named UX screenshot capture and practical desktop viewport geometry checks.
- Library tests: added/updated coverage for filter labels, thumbnail/fallback behavior, inspector explanations, Direct files wording, safe copy, and grid identity labels.

## Verification

Commands run from the project root in native Windows PowerShell:

- `npm run build` passed with the existing Vite chunk-size warning.
- `npx tsc --noEmit` passed.
- `npm run test:unit` passed: 21 files, 75 tests.
- `npm run desktop:proof:fixtures` passed.
- `npm run desktop:smoke:fixtures` passed.

The desktop proof rebuilt the Tauri release app and captured the final Library screenshots.

Latest proof folder:

- `output/desktop/library-proof/2026-05-11T00-50-35-715Z`

Named screenshots captured include:

- `library-full-ux-refinement.png`
- `02-library-grid-view.png`
- `03-library-folder-view.png`
- `04-library-detail-sheet.png`
- `05-library-preflight-detail-mccc.png`
- `library-responsive-1366x768.png`
- `library-responsive-1440x900.png`

Desktop proof geometry checks passed for:

- Casual list layout
- Seasoned list layout
- Creator list layout
- selected list layout
- inspector collapsed layout
- inspector expanded layout
- 1366 x 768 Library layout
- 1440 x 900 Library layout

Runtime console/error capture reported no severe runtime errors in the proof summary.

## What could not be verified

- WSL runtime was not run; this sprint verified native Windows PowerShell only.
- A true huge-library performance run was not performed. The changes keep virtualization and avoid new row-time expensive work.
- Missing preview extraction behavior was not rebuilt. This sprint only made preview/fallback presentation clearer and more honest.
- A full design-system migration was not attempted and remains future work if the app needs broader visual standardization.

## Remaining Library readiness work

- Continue tuning the global navigation labels and density in a dedicated app-shell pass.
- Add a large-library performance fixture if realistic high-volume fixture data becomes available.
- Consider a more formal Library component/token split so future polish does not depend on a large global stylesheet.
- Continue screenshot review with real user libraries that contain extracted CC thumbnails, Tray images, and mixed Mods/Tray folders.

## Commit tracking

- Implementation commit: `df9635c`.
- Docs hash commit: pending.
