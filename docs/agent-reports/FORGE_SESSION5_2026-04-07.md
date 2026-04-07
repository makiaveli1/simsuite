# Forge report — 2026-04-07

## 1. Best implementation slices for this session
- Best backend slice: extend `src-tauri/src/core/file_inspector/mod.rs` and let `src-tauri/src/core/scanner/mod.rs` keep merging those signals through `apply_inspection_hints`, because that improves creator/kind/subtype/version depth without reopening the whole scan pipeline.
- Best UI slice: treat Library as the vertical slice, specifically `src/screens/LibraryScreen.tsx`, `src/screens/library/LibraryDetailSheet.tsx`, `src/screens/library/LibraryDetailsPanel.tsx`, `src/screens/library/LibraryCollectionTable.tsx`, and `src/screens/library/libraryDisplay.ts`, because the current inspector, row, and sheet already contain the right seams.

## 2. Where to modernize the desktop audit path first
- First fix `scripts/desktop/desktop-smoke.mjs` selectors and readiness checks. It still depends on brittle body text and stale structure assumptions like `tr[role='button']` plus `.file-title`, while the current Library rows are motion divs with `.library-list-row` and `.library-row-title`.
- Next split the harness around stable contracts, not copy. Add explicit test hooks for app shell ready, nav buttons, list rows, inspector open state, and detail-sheet mode, then stop asserting long prose strings except where copy itself is the feature.

## 3. Which code paths to modify for high-value metadata gains
- Highest-value extraction work is in `file_inspector`: the ts4script manifest/payload path already reads names, authors, namespaces, and version clues, and the package path already derives resource summaries, embedded names, creator hints, family hints, and kind hints. That is the cheapest place to add richer structured signals.
- The follow-through path is `scanner::insert_parsed_file` plus `scanner::apply_inspection_hints`, then `src-tauri/src/models.rs` / `src/lib/types.ts` for any new `FileInsights` fields. That is where better inspection starts affecting saved classification and visible detail data.

## 4. Which UI components need changes for More Details to become truly deeper
- `LibraryDetailsPanel` is too shallow and its CTA is misrouted. `onOpenMoreDetails` always opens `health`, so the inspector never branches into the deeper inspect path even when the real value is embedded names, version evidence, or resource contents.
- `LibraryScreen` section construction plus `LibraryDetailSheet` need the real depth pass: separate health vs inspect entry points, stronger evidence grouping inside the sheet, and row-level cues from `libraryDisplay.ts` / `LibraryCollectionTable.tsx` so users can tell which files actually have deeper metadata worth opening.

## 5. Top regression risks while implementing
- Biggest backend risk: noisier inspection signals can over-promote creator, subtype, or kind in `apply_inspection_hints`, which would quietly worsen classification quality if new metadata is not ranked harder than filename/path signals.
- Biggest frontend/test risk: the desktop smoke harness is already behind the current DOM contract, so More Details or row changes can break audit coverage immediately unless selectors are stabilized first or in the same slice.
