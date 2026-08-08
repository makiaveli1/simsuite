# Library Smart Search and Model Opportunity V1

Date: 2026-08-08
Status: measured design/prototype note; no production model or search integration

## 1. Decision summary

SimSuite should not add an embedding model or LLM simply because model-assisted features are technically possible.

The first search proof shows that a large part of the current Library search gap can be closed with deterministic local retrieval using metadata SimSuite already extracts. The bundled SQLite build supports FTS5 on the current macOS development runtime, so a strong model-free search foundation is feasible without adding a new runtime, model download, network dependency, or AI-facing product claim.

Models still have plausible future value, but only where they clearly outperform that stronger deterministic baseline. The highest-value candidate is semantic matching for vague human descriptions. A later visual-similarity experiment may also be valuable because SimSuite already extracts first-party package/cache thumbnails. Neither is implemented by this milestone.

The standing trust boundary remains unchanged: model output may suggest, rank, group, match, or explain, but may not become authority for safe delete, dependency truth, patch compatibility, exact duplicate proof, path safety, malware safety, Apply/Restore eligibility, or any file-changing action.

## 2. Current production search limitation

Current `library_index::build_filters` uses a leading/trailing `%LIKE%` search across only:

- filename;
- absolute path;
- canonical creator name;
- subtype.

It does not currently search several useful local metadata fields that already exist in `FileInsights`, including:

- embedded/in-game names;
- family hints;
- resource-summary labels;
- script namespaces;
- creator hints.

The database also has user-learned creator aliases, but current Library search does not use those aliases.

This means a model would be competing against an unnecessarily weak baseline if SimSuite compared embeddings directly with the current search.

## 3. Hidden deterministic search prototype

`src-tauri/src/core/library_smart_search_prototype.rs` is test-only at both levels:

- `core/mod.rs` registers it only under `#[cfg(test)]`;
- the file itself starts with `#![cfg(test)]`.

It creates temporary in-memory search tables only during Rust tests. It does not alter the product database, migrations, Tauri commands, frontend, Library query path, scanner, or player files.

The richer deterministic search document contains:

- filename;
- canonical creator;
- representative creator-alias input;
- kind/subtype;
- embedded names;
- family hints;
- resource-summary labels;
- script namespaces.

Absolute local paths are deliberately excluded from the richer document. A regression test proves that a username present only inside an absolute path is retrievable through the current-style baseline but is not indexed by the richer search document.

The FTS query builder accepts only normalized alphanumeric tokens and quotes them before passing them to FTS5. Empty or punctuation-only input therefore does not broaden into all rows or inject FTS query syntax.

The prototype uses conservative `AND` keyword semantics. The BM25 field weights are prototype values only; they are not production-tuned.

## 4. FTS5 availability proof

A Rust test creates a real FTS5 virtual table through the same bundled `rusqlite` dependency used by SimSuite and executes a query successfully.

Result on the current macOS development runtime:

- bundled SQLite FTS5: available;
- local FTS insert/query: passed.

This is current-runtime evidence, not native Windows/Linux runtime proof. Those environments still need their own validation before a production search migration is treated as cross-platform complete.

## 5. Representative relevance proof

The synthetic fixture set intentionally covers common Sims-style identity patterns:

- toddler hair plus a colour/swatches clue;
- a HARRIE kitchen/build-buy set;
- MCCC modules;
- PandaSama childbirth gameplay;
- TwistedMexi / TMex / Better Exceptions aliases;
- a main-menu replacement;
- a Thai translation/string package.

Representative keyword queries include:

- `pink toddler hair`;
- `harrie kitchen clutter`;
- `mccc modules`;
- `childbirth mod`;
- `tmex better exceptions`;
- `main menu replacement`;
- `thai translation strings`;
- `deaderpool`;
- `BetterExceptions`.

These are synthetic capability fixtures, not real-player telemetry and not a claim of production search accuracy.

On these deliberately difficult cases:

- current-style SQLite `%LIKE%` recall@5: `0.222`;
- current-style MRR: `0.222`;
- richer deterministic FTS5 recall@5: `1.000`;
- richer deterministic FTS5 MRR: `1.000`.

The important conclusion is not the literal 100% number. The fixture was designed so the richer metadata should contain the target clues. The useful conclusion is that SimSuite can already recover important metadata/alias searches that current search misses without adding a model.

## 6. 10,000-item synthetic timing proof

The final benchmark compares both retrieval strategies inside SQLite so the timing comparison is structurally fairer than an earlier discarded Rust-loop baseline.

Final measured run:

- rows: `10,000`;
- representative queries: `9`;
- temporary fixture/index construction: `3,096 ms`;
- current-style SQLite `%LIKE%` total query time: `256 ms`;
- richer FTS5 total query time: `108 ms`;
- current recall@5: `0.222`;
- current MRR: `0.222`;
- FTS recall@5: `1.000`;
- FTS MRR: `1.000`.

The construction timing creates both benchmark tables from scratch and is not a production startup estimate. A production design should maintain its search representation incrementally as scan/index metadata changes. The benchmark deliberately has no hard latency threshold because local timing is noisy and the existing large Library stress lane shares the process with other expensive synthetic tests.

## 7. Explicit no-answer safety controls

The richer retrieval test includes queries such as:

- `safe to delete`;
- `compatible with current patch`;
- `missing mesh`;
- `malware free`.

The synthetic search index returns no positive result for these controls.

This does not create a production natural-language safety filter. It proves the desired design principle: a future search or model layer must not convert a vague authority-seeking question into an authoritative safety answer merely because some file is semantically similar.

Safety/dependency/update/mutation questions must remain routed to deterministic evidence systems that can prove or explicitly decline the requested claim.

## 8. Residual semantic gap

FTS5 is retrieval, not semantic understanding. The prototype explicitly records cases where the correct conceptual target is not recovered because the query uses different wording from the indexed metadata, for example:

- `rose kids hairstyle` versus pink/toddler/hair metadata;
- `pregnancy delivery gameplay` versus childbirth metadata;
- `game error helper` versus Better Exceptions / exception-diagnostics metadata.

These failures are useful. They define the exact class of problem that a future embedding model must improve.

A future embedding benchmark should therefore compare at least:

1. current production-style `%LIKE%`;
2. richer deterministic FTS5;
3. embedding-only retrieval;
4. preferably hybrid FTS5 + embedding reranking/union.

The embedding layer should not be integrated unless it adds material relevance on a larger, independently reviewed query fixture set while keeping acceptable memory, package-size, indexing, privacy, and latency costs.

## 9. Model opportunities by trust level

### Green: strong future candidates

#### Semantic Library search

Use embeddings only to retrieve likely files for vague human wording. The result is a ranked match, not a factual claim about the file.

Examples:

- `that pink toddler hairstyle`;
- `the mod that helps with game errors`;
- `kitchen decorations from Harrie`.

The deterministic FTS result should remain available and may be combined with semantic ranking.

#### Creator/mod-family matching suggestions

Embeddings may rank likely alias/family matches where names are messy. The user must be able to accept or reject any learned association, and deterministic creator evidence must remain distinguishable from a model suggestion.

#### Visual similarity / screenshot-to-CC

A later local image-embedding experiment could compare a screenshot/crop against thumbnails SimSuite already extracts. This could help answer “which installed hair/shirt/chair looks like this?”

Results must be labelled as possible visual matches. Visual similarity must never become delete, dependency, duplicate, or broken-CC proof.

### Amber: optional later assistance

- rank candidate update/source pages after deterministic candidate generation;
- suggest tags or collection groups;
- rank already-proven problems by likely user relevance;
- rewrite deterministic evidence into simpler prose;
- parse a natural-language search request into ordinary filters.

These are secondary because SimSuite already has deterministic proof-level-aware explanations and structured filters. An LLM must prove that it adds enough value to justify runtime and trust cost.

### Red: model must not be authority

A model must not independently decide:

- safe to delete;
- dependency truth;
- missing-mesh proof;
- exact duplicate proof;
- patch compatibility or “safe on current patch”;
- malware/safety verdicts;
- path containment or destination safety;
- whether a move/delete/restore may occur;
- Apply/Restore confirmation or eligibility;
- automatic organization destination;
- authoritative update-source ownership without deterministic verification.

## 10. Product language and player trust

SimSuite should avoid turning model use into the product identity.

Player-facing features can use ordinary capability language such as:

- `Smart search`;
- `Similar items`;
- `Suggested match`;
- `Possible creator match`.

However, neutral wording must not become concealment. If a feature uses a model, Settings/About/details should state clearly:

- that a model is being used;
- whether it runs locally or remotely;
- whether any data leaves the device;
- what inputs it receives;
- that suggestions can be wrong;
- that deterministic safety rules remain separate.

Recommended default posture:

- deterministic search/core behavior works without a model;
- local model functionality is optional unless later evidence justifies bundling a very small model;
- no model should silently download on first launch;
- any optional model download should disclose approximate size before download;
- local embeddings should never include absolute local paths when those paths are not necessary for retrieval;
- cloud-assisted features, if ever offered, require explicit opt-in and a precise data-boundary explanation.

## 11. Candidate text-embedding benchmark ladder

No model is selected by this document. Model specifications and licenses must be reverified immediately before any implementation decision.

Useful future benchmark classes include:

- a very small English sentence-transformer baseline such as MiniLM-class models;
- a compact retrieval-focused BGE-class model;
- a multilingual small model if player-language coverage proves important;
- a larger device-oriented model only as a quality ceiling, not as the default assumption.

Selection criteria should include:

- relevance gain over FTS5 and hybrid FTS5;
- model/package download size;
- resident memory;
- CPU latency on low-spec machines;
- macOS/Windows/Linux runtime packaging complexity;
- licence/distribution constraints;
- offline operation;
- deterministic fallback when the model is unavailable;
- vector storage size and incremental re-index cost.

## 12. LLM position

A general-purpose LLM is not currently justified as a core SimSuite dependency.

Most safety explanations can already be generated from deterministic proof objects. A chatbot would add hallucination and product-complexity risk without solving the hardest underlying evidence problems.

A small optional local LLM could be revisited later for narrowly bounded tasks such as rewriting proven evidence for a casual player or translating an already-determined explanation. It should receive structured evidence rather than raw unrestricted file context and should never be allowed to emit an executable file action directly.

## 13. Recommended next sequence

1. Keep this milestone test-only and commit it only after the normal project verification gate passes.
2. Design the production feasibility of an incrementally maintained FTS search representation; do not silently replace Library search yet.
3. Build a separate **embedding benchmark**, not an integration, against a larger query set containing both lexical and genuinely semantic cases.
4. Proceed with embeddings only if the measured relevance improvement is meaningful relative to footprint and runtime cost.
5. Later, separately benchmark local image embeddings for screenshot-to-CC similarity if thumbnail coverage is good enough.
6. Keep LLM work behind both of those because it currently has less direct product value.

## 14. What this milestone does not do

It does not:

- change production Library search;
- create a production FTS table or migration;
- download or bundle a model;
- add an inference runtime;
- add embeddings;
- add an LLM;
- add an AI setting or UI;
- send any player data to a network service;
- mutate any player file;
- enable Apply or Restore;
- change the standing SimSuite trust boundary.
