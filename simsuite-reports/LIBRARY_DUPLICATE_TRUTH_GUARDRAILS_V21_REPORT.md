# Library Duplicate Truth Guardrails v2.1 Report

Date: 2026-05-12

Branch: `codex/library-duplicate-truth-guardrails-v21`

## Worktree State

- Started from `codex/library-duplicate-truth-engine-v2`.
- Pre-existing unrelated dirty files were present before this sprint:
  - `.cocoindex_code/cocoindex.db/mdb/data.mdb`
  - `.cocoindex_code/target_sqlite.db`
  - `SESSION_HANDOFF.md`
  - `docs/IMPLEMENTATION_STATUS.md`
  - `src/screens/HomeScreen.tsx`
  - `src/styles/globals.css`
- These unrelated Home/status/global CSS/generated changes will be left unstaged.

## 1. Current Duplicate Truth Model

Duplicate Truth Engine v2 currently decides:

- exact duplicate: duplicate table row with `duplicate_type = exact` and matching non-empty hashes in the joined file rows;
- name match: same filename or `duplicate_type = filename`, not duplicate proof;
- version review: version-token comparison or `duplicate_type = version`, not duplicate proof;
- unknown/manual review: anything else.

User-facing `Duplicate` means deterministic same-content proof. The only implemented proof today is matching non-empty full file hash.

## 2. Guardrail Risk Audit

| Risk | Current status before v2.1 | Decision |
| --- | --- | --- |
| Empty hash | `hashes_match` rejects empty strings, but SQL counts only filtered by `duplicate_type = exact`. | needs small guardrail |
| Null hash | Pair classification rejects null hashes, but exact counts did not rejoin and revalidate. | needs small guardrail |
| Whitespace hash | Not consistently trimmed in SQL or Rust comparison. | needs small guardrail |
| Placeholder hash | No known placeholder hash value exists; scanner hashes file bytes. | not applicable |
| Same file ID on both sides | Exact insert uses `a.id < b.id`, but stale table rows could still exist. | needs small guardrail |
| Same canonical path on both sides | Exact insert did not exclude duplicate rows pointing at the same canonical path. | needs small guardrail |
| Duplicate row points to missing/deleted file | `JOIN files` skips it in pair listing; count paths need validation joins. | needs small guardrail |
| Exact row but hashes differ | Classification already avoids `Duplicate`, but count/filter paths still needed revalidation. | needs small guardrail |
| Filename row but hashes match | Deterministic hash proof should win, not the stale row type. | needs small guardrail |
| Version row but hashes match | Deterministic hash proof should win, not the stale row type. | needs small guardrail |
| Unknown duplicate type | Schema check usually blocks it; legacy rows should surface as manual review if reachable. | already safe in listing, future schema cleanup if needed |
| Source/path mismatch | Path context is display evidence only. | already safe |
| Very large name/version candidate groups | Current generation is still group-pair based and not fully stress-proven. | future larger work |
| Duplicate overview counts | Exact count did not yet validate joined file hashes/path/self-pair. | needs small guardrail |
| Library detail duplicate counts | Exact count did not yet validate joined file hashes/path/self-pair. | needs small guardrail |
| Library duplicate filter | Exact filter did not yet validate joined file hashes/path/self-pair. | needs small guardrail |
| Duplicates focused-file context | Proof now targets exact-content fixture; keep it. | already safe |

## 3. User-Facing Wording Audit

Search scope: `src/lib`, `src/screens`, and `src-tauri/src`.

- `Possible duplicate`: no active user-facing source occurrences found in touched runtime surfaces. Remaining references are negative tests or historical docs outside runtime.
- `Duplicate candidate`: no active user-facing source occurrences found in touched runtime surfaces.
- `safe to delete`: remaining runtime/test occurrences are negative assertions or unrelated download/staging copy outside this sprint.
- `confirmed duplicate`: remaining occurrences are negative tests.

## Implementation Notes

- Added a shared exact-file proof predicate in the duplicate detector:
  - joined file rows must exist;
  - file IDs must be positive and distinct;
  - both hashes must be non-empty after trimming;
  - normalized hashes must match;
  - both paths must be non-empty;
  - normalized paths must not point at the same canonical Windows path.
- Rebuilt exact duplicate generation so it trims/lowercases hashes and skips same canonical path pairs.
- Reworked duplicate listing/filtering so exact proof wins over stale row type:
  - exact filter returns only validated same-content pairs;
  - filename/version filters exclude rows that now have exact proof;
  - malformed exact rows surface as `unknown` / manual review if shown, never as `Duplicate`.
- Reworked overview, Library summary, Library filter, and file detail duplicate counts to count validated exact-content proof instead of trusting `duplicate_type = exact`.
- Hardened filename duplicate generation so equal normalized hashes are not also inserted as weak filename rows.
- Updated Duplicates helper copy to avoid weak or future-action wording.
- Added no-forbidden-wording coverage for touched duplicate surfaces.

## Validation

- `npm run build`: passed. Vite still reports the existing large chunk warning.
- `npx tsc --noEmit`: passed.
- `npm run test:unit`: passed (`22` files, `87` tests).
- `npm run test:unit -- --run src/screens/DuplicatesScreen.test.tsx src/screens/library/LibraryCollectionTable.test.tsx src/screens/library/LibraryDetailsPanel.test.tsx src/screens/library/actionPreflight.test.ts src/screens/library/actionPreflight.render.test.tsx`: passed (`5` files, `22` tests).
- `cargo fmt`: passed.
- `cargo check`: passed with the repo's existing warning set.
- `cargo test duplicate`: passed (`16` duplicate-focused tests).
- `cargo test`: passed (`231` tests).
- `cargo build --release`: passed with the repo's existing warning set.
- `npm run test:rust`: passed (`231` tests).
- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`; summary at `output/desktop/library-proof/latest-summary.json`.
- `npm run desktop:smoke:fixtures`: passed.

## Guardrail Risks Found

- Exact duplicate counts and filters were still too trusting of stored duplicate row type.
- Stale `exact` rows with empty, whitespace, missing, mismatched, self-pair, or same-path evidence needed stronger validation.
- Stale filename/version rows can become exact proof if joined hashes now match; the deterministic content proof should win over the stored weak row type.
- Duplicates helper copy still contained future-action language that could sound stronger than the current no-cleanup product surface.

## Duplicate Rule After This Pass

User-facing `Duplicate` now requires validated exact-file proof:

- both joined file rows exist;
- file IDs are positive and distinct;
- both full file hashes are non-empty after trimming;
- normalized hashes match;
- both paths are non-empty;
- normalized canonical paths are distinct.

The only implemented deterministic proof remains matching non-empty full file hash. Package/script fingerprints remain future scan-time work.

## What No Longer Counts As Duplicate

- Name match.
- Version review.
- Same mod family.
- Same folder.
- Same pack.
- Related hint.
- Missing, empty, whitespace, or mismatched hash rows.
- Self-pairs.
- Same canonical path pairs.
- Stale or malformed exact rows.

These may still be review/comparison evidence, but they do not count as Library duplicates.

## Backend Changes

- `src-tauri/src/core/duplicate_detector/mod.rs`
  - added exact-proof helpers for hashes, IDs, and canonical paths;
  - hardened exact pair rebuild SQL;
  - hardened pair classification for stale/malformed rows;
  - made exact filtering/listing validate joined file proof;
  - updated overview counts to count showable/validated rows;
  - added duplicate guardrail tests.
- `src-tauri/src/core/library_index/mod.rs`
  - added exact-proof SQL helpers;
  - updated summary counts, duplicate filters, row duplicate flags, and detail counts to use validated exact-content proof;
  - updated Library duplicate filter/detail tests for malformed exact and weak rows.

## Frontend/API Changes

- `src/lib/uiLanguage.ts`
  - removed weak/future duplicate-action phrasing from Duplicates helper copy.
- `src/screens/DuplicatesScreen.test.tsx`
  - added a focused forbidden-wording assertion for touched duplicate runtime labels.

No public Tauri command contract or TypeScript duplicate shape was changed in this light pass.

## Database / Migration Notes

No migration was added. The existing duplicate table and files table already contain the fields needed for this guardrail pass. The fix revalidates joined rows at query/count time and tightens rebuild insertion logic.

## Desktop / Runtime Proof

- `npm run desktop:proof:fixtures`: passed with `DESKTOP_LIBRARY_PROOF_OK`.
  - Latest proof folder: `output/desktop/library-proof/2026-05-12T14-24-58-071Z`.
  - `runtimeErrors`: none in `latest-summary.json`.
  - Browser log inspection still shows the existing Tauri callback-id warnings during reload/proof transitions.
- `npm run desktop:smoke:fixtures`: passed.
- Runtime proof used disposable fixture paths under `%TEMP%` and did not touch real Mods/Tray data.

## Performance Notes

- No fuzzy matching, package parsing, script parsing, thumbnail loading, or all-library all-pairs comparison was added.
- Exact proof is evaluated against existing duplicate rows and joined file records, not by recomputing duplicate candidates during ordinary Library render.
- Large same-name/version group stress behavior remains intentionally deferred to the next backend sprint.

## What Could Not Be Verified

- Package content fingerprints.
- Script archive fingerprints.
- Same-mod-family classification beyond current filename/version behavior.
- Real-user large-library duplicate behavior.
- Large duplicate-group stress performance.

## Next Backend Sprint

Next: Large-library backend stress and SQL-direct folder query optimization.

## Commit

- Implementation commit: `2c492803237e6ec70a70cc31c515053648b2f62b` - `Harden Library duplicate truth guardrails`.
- Hash-record commit: this follow-up docs commit records the implementation hash; final object ID is listed in the task closeout.
