# Sims Mod Suite — PRD Part 16: Test Plan and QA Strategy

## Status
Draft v1

## Purpose
This document defines how SimSuite should be tested before and during development.

The goal is to make the product trustworthy.

Because SimSuite touches a player’s real files, testing must focus heavily on:
- safety
- correctness
- undo reliability
- classification accuracy
- performance on large libraries
- behavior on weak computers
- clear failure handling

This document is written for Codex and for any future QA work.

---

# Current QA note (March 11, 2026)

Recent work added real coverage for:

- guided special-mod updates
- local-first version comparison
- internal file-inspection version clues
- same-version and older-version special-mod cases
- Inbox startup state and locked-read retry behavior

But there is still a major gap between code checks and real desktop
behavior:

- Inbox can still feel slow or hang in live use even when tests pass

Because of that, the next QA focus should be:

1. real desktop Inbox flow checks
2. first-open Inbox behavior without pressing Refresh
3. item-selection responsiveness
4. special-mod selection and update responsiveness
5. lock and refresh races during watcher activity

Do not treat browser-preview-only checks as enough for Inbox work.

---

# 1. Testing Principles

## 1.1 Safety over speed
A slow safe system is better than a fast risky one.

## 1.2 Realistic libraries matter
Testing should use realistic Sims content sets, not only tiny fake examples.

## 1.3 Deterministic behavior first
Rules and validators must be tested thoroughly before AI behavior is trusted.

## 1.4 AI must be treated as uncertain
AI output is always validated and never trusted blindly.

## 1.5 Undo must be proven
Every move-related feature must be tested with rollback scenarios.

---

# 2. Main Test Categories

The project should use these test categories:

- unit tests
- integration tests
- filesystem simulation tests
- UI workflow tests
- performance tests
- regression tests
- AI schema validation tests
- migration and rollback tests

---

# 3. Unit Tests

Unit tests should cover small isolated logic.

Priority areas:

## 3.1 Filename parser
Test:
- creator extraction
- subtype extraction
- version extraction
- noise token handling
- conflicting token detection

Examples:
- simstrouble_breezy_hair_v2.package
- peacemaker_modern_sofa.package
- default_eyes.package
- ab12_final_new.package

## 3.2 Rule engine
Test:
- rule token substitution
- rule priority order
- user rule override
- mirror mode path calculation
- fallback behavior

## 3.3 Safety validator
Test:
- script depth protection
- tray file placement rules
- package depth limits
- invalid path correction
- path collision handling

## 3.4 Creator normalizer
Test:
- alias matching
- underscore / dash normalization
- case-insensitive matching

Examples:
- peacemaker_ic
- Peacemaker-IC
- PEACEMAKER IC

## 3.5 Taxonomy mapping
Test:
- allowed category mapping
- unknown fallback
- subtype mapping behavior

---

# 4. Integration Tests

Integration tests should verify that modules work together correctly.

Priority flows:

## 4.1 Scan flow
scanner → bundle detector → database

## 4.2 Suggestion flow
scanner → parser → rule engine → validator → preview data

## 4.3 Move flow
preview → approval → move engine → snapshot manager → database update

## 4.4 AI flow
heuristics → AI input builder → AI validator → final classification result

## 4.5 Duplicate flow
scanner → hash compare → duplicate grouping → duplicate actions

---

# 5. Filesystem Simulation Tests

These tests should use temporary directories and sample files.

Important scenarios:

## 5.1 Existing organized library
Simulate:
- CAS/Hair/Simstrouble
- BuildBuy/Peacemaker
- Gameplay/Lumpinou

Verify:
- structure is detected correctly
- Mirror Mode keeps using this layout

## 5.2 Chaotic library
Simulate:
- random folders
- duplicate files
- mixed tray files inside Mods
- deep script paths

Verify:
- issues are detected
- unsafe actions are blocked
- review queue is populated

## 5.3 Downloads intake
Simulate:
- ZIP archive with CAS set
- ZIP archive with mixed Mods + Tray content
- loose ts4script file
- unknown extension file

Verify:
- extraction works safely
- mixed content is flagged
- previews are correct

---

# 6. UI Workflow Tests

These tests check whether the app behaves correctly from the user’s point of view.

Important workflows:

## 6.1 First launch
- detect Sims folder
- scan Mods and Tray
- show structure summary
- offer Observe / Mirror / Migrate / Fresh options

## 6.2 Review queue handling
- open review item
- accept suggestion
- edit destination
- create “Always do this” rule

## 6.3 Duplicate cleanup
- view duplicate group
- keep newest
- archive older
- undo the action

## 6.4 Patch recovery
- create snapshot
- compare snapshots
- restore snapshot

---

# 7. AI Validation Tests

The AI layer needs special tests.

## 7.1 Schema validation
Test:
- valid JSON
- invalid JSON
- missing fields
- wrong enum-like values
- confidence out of range

## 7.2 Retry behavior
If model returns invalid JSON:
- one repair attempt allowed
- second failure routes safely to fallback

## 7.3 Benchmark cases
Maintain a benchmark set using real-style filenames.

Examples:
- simstrouble_breezy_hair.package
- peacemaker_modern_sofa.package
- lumpinou_relationship_overhaul.package
- mc_cmd_center.ts4script
- default_eyes.package
- tray household bundle sample

Track:
- heuristic result
- AI result
- final result
- human expected result

---

# 8. Performance Tests

The app should be tested with larger libraries.

Suggested targets:
- 1,000 files
- 5,000 files
- 10,000 files
- 20,000 files if possible

Measure:
- initial scan time
- incremental scan time
- duplicate detection time
- review list rendering time
- search/filter responsiveness

Also test on:
- stronger machine
- weaker laptop
- no dedicated GPU scenario

---

# 9. Migration and Undo Tests

These are critical.

## 9.1 Assisted migration
Verify:
- snapshot created first
- preview matches actual move set
- failed move does not corrupt state

## 9.2 Undo system
Verify:
- single move undo
- batch move undo
- rollback after partial failure

## 9.3 Snapshot restore
Verify:
- original paths restored
- hashes still match expected files
- UI reflects restored state correctly

---

# 10. Regression Test Priorities

Every major release should rerun regression checks for:

- script depth safety
- tray bundle handling
- duplicate grouping
- creator alias matching
- rule precedence
- undo/rollback
- AI schema validation
- downloads watcher behavior

---

# 11. Failure Case Tests

Important failure scenarios:

- archive cannot be opened
- file is locked by another process
- permission denied
- duplicate hash collision edge case
- missing tray bundle partner file
- AI model unavailable
- SQLite file temporarily locked

The app must fail gracefully and show plain-language messages.

---

# 12. Acceptance Criteria for v1

Before release, SimSuite should pass these checks:

- read-only scanning works on Mods and Tray
- Mirror Mode respects existing structures
- script mods are never moved into unsafe depth
- tray content is detected and grouped correctly
- duplicate detection works for exact duplicates
- downloads watcher produces correct previews
- undo works for approved move batches
- invalid AI output never causes unsafe actions

---

# 13. Recommended Test Assets

Codex should prepare a local sample library containing:

- CAS files
- Build/Buy files
- gameplay files
- ts4script files
- default replacements
- pose packs
- tray household bundle
- tray lot bundle
- tray room bundle
- mixed archives
- duplicate files
- fake creator aliases

This asset pack should be reusable in automated tests.

---

# 14. Handoff Notes for Codex

Codex should build testing into the project from the start.

Minimum requirements:
- unit tests for parser, rules, validator
- integration tests for scan and move pipelines
- filesystem simulation tests
- AI schema validation tests
- rollback tests

A file management app without strong testing is not trustworthy enough to ship.
