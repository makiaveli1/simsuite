# Session 12 — Tray Relationship Graph and Binary-Intelligence Feasibility Spike

Date: 2026-04-09
Canonical root: `/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/`

## Goal
Answer how much stronger tray intelligence SimSort can honestly gain right now, especially around:
- relationship grouping
- stronger related-file modeling
- deeper tray binary understanding
- practical low-risk wins vs work that should be deferred

## Agent participation
Session 12 again hit a real agent-launch reliability problem.

### What failed
- normal `sessions_spawn` remained unreliable on the gateway WS handshake path
- generic RPC still worked
- the failure path was launch-specific, not total gateway loss

### Recovery path used
Required agent participation was recovered using isolated agent turns instead of normal spawn.

Observed outcomes:
- Ariadne: usable product guidance recovered
- Sentinel: participation path recovered, but transcript body was not useful enough to quote directly
- Scout: participation path recovered, but output was incomplete / non-usable as a final report
- Forge: participation path recovered, but no fresh useful report body was available for direct quoting

No fabricated quotes or fake conclusions were used.

## Source-backed relationship audit

### What currently links tray files together
Tray grouping today is primarily driven by the bundle detector, not a true relationship graph.

In source:
- `bundle_detector/mod.rs` groups files by a bundle key built from:
  - parent directory
  - filename stem
- bundle records persist:
  - `bundle_name`
  - `bundle_type`
  - `file_count`

This means current grouping is fundamentally:
- shared naming/path structure
- shared bundle membership

It is **not** a file-to-file semantic relationship graph.

### What `bundleName` and `bundleType` really mean
They are bundle-detector outputs, not deep binary truth.

`bundleName`
- a grouping label derived from bundle grouping logic
- useful as a hint
- not proof of canonical household/lot identity

`bundleType`
- a bundle classifier label
- useful for tray grouping semantics
- not proof of a richer underlying graph

### What would count as a true relationship vs just a grouping hint
**Grouping hint**
- same bundle stem
- same bundle id
- same folder/stem-derived cluster
- same family hint label

**True relationship graph** would need stronger evidence such as:
- explicit cross-file references
- stable parsed identifiers shared across tray companions
- binary-level linkages
- directional evidence (e.g. exported-from / depends-on / same household package set)

Current source audit found no such real graph in place.

## Binary-intelligence feasibility audit

### What is currently being read
The current tray path mostly stops at extension/classification level.

Findings:
- `filename_parser/mod.rs` classifies tray-like extensions and assigns tray-oriented kinds/subtypes
- `scanner/mod.rs` accepts tray extensions into the library and stores generic file facts
- `bundle_detector/mod.rs` groups files into bundles and stores `file_count`
- `file_inspector/mod.rs` has rich package/script inspection paths, but non-supported formats fall through to `InspectionOutcome::default()`

### What is not being deeply parsed today
No dedicated deep parsing was found for:
- `.trayitem`
- `.householdbinary`
- `.blueprint`
- `.room`
- `.bpi`

No source-backed evidence was found for:
- household member extraction
- lot title extraction
- room title extraction
- stable semantic tray-to-tray relationship extraction

### Feasibility now vs later
**Low-hanging now**
- expose bundle file count as an honest grouped-files signal
- improve grouping-vs-relationship language
- keep derived grouping clearly separated from true related-file claims

**Would require a dedicated parsing project**
- deep household / lot / room identity extraction
- stable binary-derived relationship graph
- edge-level related-file semantics
- binary-backed stronger naming

## Real-sample findings

### Live dataset reality
The live SimSort database at:
`/home/likwid/.local/share/com.likwi.simsuite/simsuite.sqlite3`
currently contained **zero indexed tray-format rows** during this session.

That means direct real-dataset tray binary inspection was **not possible** from the current indexed library state.

### Representative samples used
Because the live DB had no tray rows, representative tray examples were taken from the current app/dev sample layer:
- `LooseBlueprint.blueprint`
- `OakHousehold_0x00ABCDEF.trayitem`

These were used only for UI verification of the prototype surface, not as proof of deep binary extraction.

## Feasibility verdicts

### A. Stronger related-file graph
**Partially feasible now**

Why:
- we can surface stronger grouped-file counts from real bundle membership
- we cannot honestly claim true semantic relationships yet
- current system supports a better grouping surface, not a real graph

### B. Household / lot / room identity enrichment
**Partially feasible now**

Why:
- classification-level identity is already available
- grouping/state language can improve
- deep binary identity is not implemented
- no source-backed evidence of deeper extractable identity in the current pipeline

### C. More trustworthy tray naming
**Feasible now**

Why:
- tray UI can use classification + grouping + grouped-file count carefully
- this improves naming without inventing deep identity

### D. Better tray More Details sections
**Feasible now**

Why:
- More Details can surface grouped-file count and tray summary honestly
- section language can clearly distinguish grouping from relationship certainty

## Prototype implemented
A low-risk prototype was justified and implemented.

### Prototype
Surface `bundles.file_count` as an honest grouped-files signal.

### What changed
Backend / model plumbing:
- added `grouped_file_count` to tray-facing row/detail models
- wired `b.file_count` through library index list/detail queries

Frontend:
- added grouped-file count to TS row/detail types
- mock tray examples updated with representative grouped counts
- tray rows now support grouped-file count as a derived signal
- tray inspector shows `Tray set` with grouped-file count
- tray More Details tray section shows `Tray set` count
- tray summary copy now references grouped-file count when available

### What it is not
This is **not** a true relationship graph.
It is an honest grouped-files surface derived from bundle membership.

## Product surfacing implications

### Rows
Good now:
- grouping label
- grouped-file count
- storage state

Avoid:
- “related files” phrasing in rows unless real graph evidence exists

### Inspector
Good now:
- tray type
- stored
- grouped as
- tray set count
- tray summary sentence

### More Details
Good now:
- tray set count in tray-specific section
- grouping hint language
- derived evidence labeling

### Casual / Seasoned / Creator
- Casual: storage + simple tray identity only
- Seasoned: grouping + tray set count when real
- Creator: grouping + tray set count + deeper diagnostics, but still no fake graph edges

## Visual verification reviewed
Fresh live screenshots reviewed for:
- `LooseBlueprint.blueprint` row + inspector + More Details
- `OakHousehold_0x00ABCDEF.trayitem` row + inspector

Verified live:
- grouped-file count appears as a tray-set signal
- no raw path leak returned
- relationship wording did not overstep into fake certainty
- footer/scroll behavior stayed stable in the verified surfaces

## Smoke checks
- targeted tray tests passed
- build completed cleanly

## Session 12 conclusion
Current SimSort can honestly move beyond vague tray grouping by surfacing real bundle membership counts.

It cannot yet claim a true tray relationship graph or deep binary-derived identity.

That means the right current product stance is:
- stronger grouped-file surfacing now
- clear grouping-vs-relationship language
- defer deep binary/graph work until a dedicated parsing project exists
