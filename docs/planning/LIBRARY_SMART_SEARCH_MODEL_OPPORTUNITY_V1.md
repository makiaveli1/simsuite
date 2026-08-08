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

## 11. Measured local text-embedding benchmark

A separate benchmark was run after the deterministic FTS milestone. This was **not** a SimSuite integration. The harness lived outside the repository, used synthetic records only, and added no SimSuite package dependency, model binary, cache, schema, command, UI, network service, or player-file access.

### Candidate and runtime

The first floor candidate was `Xenova/all-MiniLM-L6-v2`, the Transformers.js-compatible ONNX conversion of `sentence-transformers/all-MiniLM-L6-v2`.

For this benchmark:

- licence: Apache-2.0;
- runtime: Transformers.js 3.8.1 in Chrome/WASM;
- inference location: local browser only;
- dtype: q8;
- embedding dimension: `384`;
- measured model/tokenizer artifact set: `23,685,047` bytes, about `22.59 MiB`;
- no cloud inference was used.

Model specifications and licence still need to be reverified immediately before any future distribution decision.

### Corpus and comparison

The benchmark used `10,000` synthetic Library documents:

- `30` meaningful Sims-style records;
- `9,970` generic distractors;
- `43` positive queries across lexical, alias, semantic-paraphrase, typo, and ambiguous classes;
- `6` explicit authority/safety negative controls.

The same metadata classes used by the deterministic prototype were embedded: filename, creator/alias text, kind/subtype, embedded names, family hints, resource-summary labels, and script namespaces. Absolute player paths were not part of the embedding text.

The deterministic comparison used an in-memory SQLite FTS5 index with the same strict normalized `AND` token semantics as the committed prototype. The embedding and a simple FTS+embedding hybrid were scored with recall@5 and MRR.

### Measured performance on the current macOS development machine

Deterministic FTS side:

- FTS construction for `10,000` rows: about `250 ms` in this isolated run;
- all benchmark FTS queries: about `38 ms` total.

Local MiniLM side:

- first model load/download: about `3.66 s`;
- warm cached model load: about `0.92 s` on the latest warm run;
- first `10,000`-document embedding build: about `280.7 s` (`4 min 41 s`);
- warm query embedding: about `9 ms` per query;
- warm incremental embedding of `100` changed items: about `3.41 s`, or `34 ms` per item in this browser/WASM harness.

Vector storage at `10,000 × 384` dimensions:

- float32: about `14.65 MiB`;
- theoretical int8 vector payload: about `3.66 MiB`, before any index/metadata overhead.

Chrome exposed only partial JavaScript-heap measurements. It did **not** expose a trustworthy total WASM/native resident-memory figure, so this benchmark does not establish a production RAM requirement.

### Retrieval result

On this deliberately controlled synthetic fixture:

- FTS overall recall@5: `0.512`;
- FTS semantic recall@5: `0.280`;
- FTS typo recall@5: `0.000`;
- embedding overall recall@5: `1.000`;
- embedding semantic recall@5: `1.000` with semantic MRR `0.980`;
- embedding typo recall@5: `1.000`;
- the simple hybrid matched embedding-only on this fixture and added no measurable relevance benefit.

These values are **capability evidence, not product accuracy**. The meaningful documents and queries were handcrafted in the same benchmark design, which can make semantic relationships cleaner than real libraries. The result proves that a small embedding model can recover semantic wording that exact-token FTS misses; it does not prove that every player query will be understood correctly.

The deterministic spelling comparison was subsequently completed in the same test-only search laboratory; see the next section. That follow-up recovered the controlled typo/partial-name cases without a model, so typo recovery is **not** evidence that embeddings are required. The remaining model-specific opportunity is semantic wording whose useful concepts are not lexically close to the indexed metadata.

### Authority/safety result

This was the most important trust finding.

Raw dense retrieval returned a nearest-neighbour file for **every** authority-seeking negative query, including requests equivalent to:

- safe to delete;
- remove without breaking anything;
- compatible with the current patch;
- malware free;
- missing-mesh proof;
- broken after the latest patch.

That behaviour is normal for nearest-neighbour retrieval: it finds the closest item even when the correct product response is `unknown` or `this requires deterministic evidence`.

A separate deterministic benchmark gate returned no answer for all six controls. This demonstrates a required architecture rule rather than validating that exact gate for production: semantic retrieval must never be allowed to become the authority layer for safety, dependency, update, compatibility, security, or mutation decisions.

### Decision after V1

**Proceed with further R&D, not production integration.**

The model is small enough and the semantic gain is large enough to justify one more benchmark stage, especially for vague Library search. The current evidence is not sufficient to bundle or download a model in SimSuite.

Before any production model decision, the remaining evaluation must:

1. use a larger independently authored query set rather than queries written with knowledge of the fixture contents;
2. compare embeddings only against the now-stronger deterministic FTS + fuzzy/trigram stack on genuinely semantic misses;
3. test native Windows and Linux runtime/packaging paths;
4. test representative low-spec player hardware;
5. obtain a trustworthy total process/WASM memory measurement;
6. design and measure background incremental indexing rather than blocking first-run UI;
7. preserve a deterministic no-model fallback and a hard authority router;
8. reverify model licence, artifact provenance, and distribution terms immediately before any shipping decision.

## 12. Measured deterministic fuzzy/trigram benchmark

A second follow-up stayed inside the existing doubly test-gated Rust search laboratory. It added no production Library table, migration, command, UI, dependency, model, network path, or player-file access.

### Strategy

The prototype keeps strict FTS5 as the first retrieval signal, then adds a bounded spelling-recovery lane:

1. the same privacy-bounded rich metadata is flattened into a test-only FTS5 `trigram` index;
2. query character trigrams retrieve at most `80` candidate rows from SQLite;
3. up to `20` strict FTS candidates are preserved so known-good exact/alias retrieval cannot be displaced;
4. only that bounded candidate set is reranked in Rust using camel-case-aware tokenization plus deterministic normalized Levenshtein/prefix similarity;
5. non-strict candidates below a `0.72` prototype similarity floor are discarded;
6. an explicit benchmark-only authority gate keeps safety/dependency/update/security questions outside fuzzy retrieval.

The candidate design is important for low-spec behavior: it does **not** scan or edit-distance-score all `10,000` records for each query.

### Holdout-style query set

The follow-up added `23` queries after the original seven meaningful fixture documents were already established:

- `7` realistic typo cases;
- `7` partial-name cases;
- `4` reordered/vague-but-lexical cases;
- `3` short creator/mod-name cases;
- `2` ambiguous queries where more than one target is acceptable.

Examples include misspellings such as `pandasam chldbirth`, `twistd mexi exceptions`, `minmal main menue`, and partial forms such as `baysic harr` and `menu replac`.

This is a stronger separation than writing the original metadata and exact queries together, but it is still **not independent player research**: the benchmark author knew the fixture contents while writing the holdout queries. The scores remain capability evidence rather than expected real-player accuracy.

### 10,000-item measured run

On the current macOS development machine, in the existing Library stress lane:

- complete temporary search-lab indexing (`%LIKE%` table + unicode FTS + trigram FTS): about `392 ms`;
- standalone `10,000`-row trigram index build: about `263 ms`;
- current-style `%LIKE%` representative queries: about `37 ms` total;
- strict FTS representative queries: about `1 ms` total;
- strict FTS over all `23` holdout queries: about `2 ms` total;
- bounded fuzzy/trigram over all `23` holdout queries: about `129 ms` total, roughly `5.6 ms/query`;
- fuzzy candidate rows inspected: `16.0` average, `80` maximum.

The standalone trigram timing and complete search-lab timing were separate runs and are not additive production-startup estimates. A production design would also maintain its search representation incrementally rather than rebuild it for every query.

### Retrieval result

On this controlled holdout set:

- strict FTS overall recall@5 / MRR: `0.435 / 0.435`;
- fuzzy/trigram overall recall@5 / MRR: `1.000 / 1.000`;
- typo recall@5: `0.000` strict -> `1.000` fuzzy;
- partial-name recall@5: `0.286` strict -> `1.000` fuzzy;
- vague-but-lexical recall@5: `0.750` strict -> `1.000` fuzzy;
- short-name recall@5: `1.000` for both;
- ambiguous-query recall@5: `1.000` for both.

The original exact/alias representative set remained `1.000` recall@5 and MRR through the fuzzy path, so this prototype did not trade away the stronger strict-search cases to gain typo recovery.

The semantic controls also remained unresolved by the fuzzy layer: it still did not recover the intended targets for `rose kids hairstyle`, `pregnancy delivery gameplay`, or `game error helper`. That is the desired distinction. Trigram/edit similarity helps when the player is *spelling or abbreviating known words differently*; it does not supply semantic knowledge when the wording changes conceptually.

### Decision after deterministic fuzzy V1

**A production search foundation should be model-free first.**

The current evidence supports this order:

1. rich deterministic FTS for exact metadata, aliases, and ordinary keyword search;
2. bounded deterministic fuzzy/trigram recovery for misspellings, partial names, and near lexical matches;
3. optional local embeddings only for the remaining semantic-retrieval problem, if a larger independent evaluation still justifies their footprint and trust cost.

This materially narrows the model opportunity. Embeddings no longer need to solve normal typos or partial creator/mod names; their case rests on genuinely semantic retrieval and later visual similarity.

## 13. Production-shape deterministic index feasibility

A third follow-up tested how the model-free search foundation could be maintained against a production-shaped SQLite lifecycle without changing the real SimSuite schema or Library query path. The proof remains inside the doubly test-gated Rust laboratory and uses temporary synthetic databases only.

### Why explicit synchronization is the leading design

The current Library lifecycle has a small number of meaningful metadata-change boundaries rather than one simple append-only table:

- scanner refreshes replace installed Mods/Tray `files` rows inside one transaction;
- creator learning can change a creator association and a creator alias applies to every installed file owned by that creator;
- a user-learned alias can be reassigned from one creator to another, so both the previous and new creator scopes must be refreshed;
- category overrides can change `kind`/`subtype` for selected file rows;
- deferred detail inspection can later enrich searchable `insights`;
- the real creator vocabulary includes both seeded aliases and user-learned aliases, so a production search document must combine both sources and rebuild when seed-owned alias meaning changes;
- `downloads` rows are not installed Library search content.

The feasibility proof therefore avoids broad SQLite triggers. The smallest understandable future synchronization surface is:

1. rebuild the installed search index inside the scanner's existing replacement transaction;
2. refresh a bounded list of file IDs when searchable file metadata changes;
3. refresh all installed file IDs for every affected creator when learned alias ownership changes, including both the old and new creator when an alias is reassigned.

This keeps synchronization explicit at the existing transaction boundaries and gives SimSuite one obvious full-repair operation if the search index ever becomes stale.

### Privacy and scope proof

The production-shaped search document deliberately never reads `files.path` into searchable text. Tests use a path containing a synthetic private username and prove that username is absent from both Unicode and trigram search. A separate downloads-only fixture with unique searchable words is also excluded.

The indexed text remains limited to the same player-useful local metadata classes already proven in the earlier search work:

- filename;
- canonical creator and learned creator aliases;
- kind/subtype;
- embedded names;
- family hints;
- resource-summary labels;
- script namespaces.

### Lifecycle and repair proof

The temporary production-shaped database proves:

- initial backfill from installed Mods/Tray source rows;
- source-row insert plus search-row insert in one transaction;
- selected-file category and `insights` refresh;
- creator-wide alias refresh across every installed file for that creator;
- source deletion and out-of-installed-scope movement remove the search row;
- scan-style delete/reinsert plus full search rebuild can commit atomically;
- injected stale search rows are removed by a full rebuild;
- full rebuild is idempotent;
- a deliberately interrupted rebuild rolls back and leaves the previous complete index queryable rather than exposing a partial new index.

These proofs passed for both ordinary FTS tables and the leaner contentless-delete strategy described below.

### Ordinary FTS storage baseline

A `10,000`-row on-disk synthetic production-shape comparison measured:

- source tables only: `4,722,688` bytes, about `4.50 MiB`;
- source + Unicode FTS: `7,135,232` bytes;
- source + trigram FTS: `10,153,984` bytes;
- source + both ordinary FTS indexes: `12,566,528` bytes, about `11.98 MiB`.

Relative to the synthetic source database, the two ordinary FTS indexes added `7,843,840` bytes, about `7.48 MiB`. Ordinary FTS is simple and fast, but it stores another readable copy of indexed text internally.

Measured ordinary dual-index operations on the current macOS development machine:

- initial `10,000`-row dual-index backfill: about `897 ms`;
- `100` source+search refreshes in one transaction: about `26 ms` total;
- `100` source deletes plus search cleanup: about `6 ms` total;
- full repair/rebuild after `100` removals (`9,900` indexed rows): about `1,156 ms`;
- warm strict retrieval: about `203 µs/query` averaged over `100` queries;
- warm bounded fuzzy retrieval: about `25.2 ms/query` averaged over `100` queries.

### Contentless-delete comparison

The bundled SQLite runtime also successfully creates and mutates FTS5 `contentless_delete=1` Unicode and trigram tables. In this shape the FTS tables retain the search index but do not expose a second readable copy of the source text. Search returns row IDs/rank; bounded fuzzy reranking reconstructs the candidate document from the authoritative source tables.

The correctness proof verifies normal delete/reinsert synchronization, stale-row repair, and rollback of a deliberately interrupted full rebuild. It also confirms that selecting an indexed user column from the contentless table returns no stored source text.

On the same `10,000`-row synthetic source database:

- source + both contentless indexes: `9,424,896` bytes, about `8.99 MiB`;
- contentless search overhead above source-only: `4,702,208` bytes, about `4.48 MiB`.

Compared with the ordinary dual-index database, contentless-delete reduced total database size by exactly `25%` in this fixture and reduced the search-index overhead by about `40%`.

Measured contentless operations:

- initial `10,000`-row backfill: about `815 ms`;
- `100` source+search refreshes in one transaction: about `80 ms` total, still below `1 ms` per changed item in this local run;
- warm strict retrieval: about `206 µs/query`, effectively the same order as ordinary FTS;
- warm bounded fuzzy retrieval: about `11.8 ms/query` in this run.

The faster contentless fuzzy timing should **not** be interpreted as an inherent contentless advantage from one local run. It reconstructs metadata through bounded source lookups and needs broader profiling before any production latency claim. Likewise, the roughly `3.1×` slower `100`-item incremental refresh than ordinary FTS is real in this run even though the absolute time remained small.

### Leading future schema recommendation

**Contentless-delete Unicode FTS + contentless-delete trigram FTS with explicit synchronization is the leading migration candidate, not an implemented migration.**

Why it currently leads:

- source Library tables stay the only readable metadata authority;
- absolute paths never need to enter the search representation;
- installed-only scope is explicit;
- full repair is straightforward and idempotent;
- source replacement and index rebuild can share one transaction;
- per-file and creator-wide metadata refreshes are explicit;
- measured storage overhead is materially lower than ordinary FTS;
- strict and fuzzy retrieval remain comfortably interactive in the synthetic macOS proof.

Before turning this into a real migration, SimSuite still needs:

1. native Windows and Linux proof of the bundled FTS5/contentless-delete behavior;
2. measurements against a database whose row sizes and metadata distribution resemble real SimSuite data more closely;
3. a production implementation review of transaction ownership and error propagation at scanner/creator/category/insight update boundaries, including alias reassignment refreshing both old and new creator scopes;
4. a compact searchable-insight projection that reads only embedded names, family hints, resource-summary labels, and script namespaces instead of deserializing/copying thumbnail or other media payloads during search-index maintenance;
5. a batched source-document reconstruction design for fuzzy candidate IDs, rather than relying on one source lookup per candidate indefinitely;
6. explicit combination of seeded creator aliases plus user-learned aliases, with seed-version changes forcing a repair/rebuild when needed;
7. independent player-query evaluation and realistic concurrency/locked-database testing;
8. schema upgrade/rollback and self-repair tests before any production migration is accepted.

No production migration, command, UI, or search replacement is enabled by this feasibility milestone.

## 14. LLM position

A general-purpose LLM is not currently justified as a core SimSuite dependency.

Most safety explanations can already be generated from deterministic proof objects. A chatbot would add hallucination and product-complexity risk without solving the hardest underlying evidence problems.

A small optional local LLM could be revisited later for narrowly bounded tasks such as rewriting proven evidence for a casual player or translating an already-determined explanation. It should receive structured evidence rather than raw unrestricted file context and should never be allowed to emit an executable file action directly.

## 15. Recommended next sequence

1. Keep production Library search unchanged while the contentless deterministic design remains a proven candidate rather than a migration.
2. Expand retrieval evaluation with a larger query set authored independently from indexed fixture descriptions, ideally using voluntarily supplied/public player phrasing rather than private Library content.
3. Do a migration-design-only review for explicit contentless search synchronization at scanner, creator-learning, category-override, and searchable-insight update boundaries, including lock/concurrency and schema-repair behavior.
4. Re-run local embeddings only against genuinely semantic cases that the deterministic FTS + fuzzy stack still misses.
5. If semantic embeddings still justify themselves, keep them optional/local with explicit model download, deterministic fallback, native Windows/Linux proof, low-spec testing, and a hard authority router.
6. Separately benchmark local image embeddings for screenshot-to-CC similarity if thumbnail coverage is good enough.
7. Keep LLM work behind both retrieval tracks because it currently has less direct product value and a larger trust surface.

## 16. What this work does not do

It does not change SimSuite production behavior. In particular, it does not:

- change production Library search;
- create a production FTS/vector table or migration;
- bundle, silently download, or ship a model with SimSuite;
- add a SimSuite inference runtime or package dependency;
- add production embeddings;
- add an LLM;
- add an AI setting or UI;
- send player data to a model or network service;
- read real player files for the embedding benchmark;
- mutate any player file;
- enable Apply or Restore;
- change the standing SimSuite trust boundary.

The isolated developer benchmark did temporarily download the q8 MiniLM artifacts into the browser cache outside the SimSuite repository. Those artifacts are not part of SimSuite and must not be committed or treated as a product dependency.
