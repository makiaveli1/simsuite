# Unified Version and Update-Watch Plan

## Summary

Expand the rework from “special-mod version fixes” to a broader foundation that can handle **all mods and CC**, while still keeping **guided install only for special mods**.

The core idea is:
- build **one shared version and identity system** for everything in the library
- let Inbox use that system to compare a new download against what is already installed
- let Library use that system to show installed version facts and update-watch status
- keep the special-mod workflows on top of that shared foundation for the harder install cases

This means the rework is no longer just version parsing. It also needs a **general matching layer** so SimSuite can tell which installed content a download belongs to.

## Key Changes

### 1. Build one shared version-and-match foundation for all content
- Add a universal version signal model for all files, not just special mods.
- `file_inspector` should collect structured signals from:
  - filenames
  - archive member paths
  - embedded names
  - manifest text
  - readable script/package payload text
  - resource summaries
- Add a shared content identity layer that builds a **version subject** for installed content and downloaded content.
- A version subject should represent one mod or CC item, even if it has several files.
- Matching rules for non-special content should score:
  - creator hints
  - family hints
  - embedded names
  - script namespaces
  - normalized filename tokens
- If the match is weak, SimSuite must say so instead of pretending it knows.

### 2. Keep special mods as the strict supported layer
- Special mods should keep:
  - guided install
  - repair logic
  - dependency checks
  - blocked-flow rules
  - rollback safety
- Move special mods onto the same new shared version-and-match engine, but let them keep their stricter profile rules.
- Add `versionStrategy` to special-mod profiles so each supported mod can say:
  - which local clue types to trust first
  - which noisy values to ignore
  - how to clean raw version text
  - where useful internal payload clues usually live
- Lumpinou Toolbox should be the first proof case for this new rule system.

### 3. Expand local compare to all mods and CC
- Inbox should compare any download to installed content when SimSuite can make a believable match.
- For all content, the compare result should be:
  - `not_installed`
  - `incoming_newer`
  - `same_version`
  - `incoming_older`
  - `unknown`
- Add a separate confidence field:
  - `exact`
  - `strong`
  - `medium`
  - `weak`
  - `unknown`
- Signature matches remain the strongest proof.
- If installed and incoming versions look equal but signatures disagree, return `unknown`.
- If the subject match is weak, return `unknown` instead of making a firm update claim.
- For normal mods and CC, SimSuite should stop at compare guidance.
- Only special mods should offer guided install or guided reinstall actions.

### 4. Give Library a clean role in the broader system
- Inbox stays the place for:
  - “is this download newer, older, same, or unclear?”
  - “is this safe to install?”
- Library should gain installed-version awareness for all content, but not install workflow actions.
- In Library, the selected item or selected content group should show:
  - installed version summary
  - local evidence summary
  - watch status
- Library should not show guided apply buttons.
- Keep Library list rows simple at first:
  - do not add lots of extra columns in phase 1
  - show update/watch state in the detail panel first
- Add one Library filter later for:
  - `Update available`
  - `Possible update`
  - `Unknown`

### 5. Add a watch system for broader update alerts
- Add a separate watch-source system for installed content.
- Watch sources should support:
  - exact mod pages
  - creator pages
- Creator page watches are allowed, but only when they are:
  - official
  - user-approved
  - publicly readable without hacks
- Watch results should have two levels:
  - exact update match
  - creator page changed / possible update
- Do not treat “creator page changed” as the same thing as “this exact mod has a confirmed update.”
- Keep official latest helper-only:
  - it should help Library and Home show alerts
  - it must not override local Inbox compare truth
- No challenge bypasses, login scraping, or risky workarounds.

### 6. Add a SimSuite-owned candidate queue for growth
- Keep the large Sims mod list frozen and reference-only.
- Do not build runtime support directly from it.
- Add a separate SimSuite-owned candidate file for future supported mods.
- That file should track:
  - candidate identity
  - official source
  - install-shape notes
  - version-rule notes
  - watch-source notes
  - fixture readiness
  - support status
- Only reviewed `supported` entries should move into the runtime special-mod catalog.

## Phase Order

### Phase 1. Universal version signals
- Add structured `versionSignals` to `FileInsights`.
- Keep `versionHints` as a short derived list for compatibility.
- Refactor file inspection so it collects signals instead of deciding final versions.

### Phase 2. Universal content matching and compare
- Add internal `VersionSubject` building for installed and downloaded content.
- Add shared compare resolver with:
  - subject match score
  - version result
  - confidence
  - evidence
- Use it for generic Inbox compare on non-special content.

### Phase 3. Migrate current special mods
- Move the current six supported special mods onto the new resolver:
  - MCCC
  - XML Injector
  - Lot 51 Core Library
  - Sims 4 Community Library
  - Lumpinou Toolbox
  - Smart Core Script
- Keep their existing guided install behavior.

### Phase 4. Library watch status and broader alerts
- Add watch-source data and parser strategies.
- Show installed version + watch status in Library detail.
- Add Home-level update counts after Library watch status is stable.
- If background mode is enabled later, only exact high-confidence updates may raise a desktop notification.

### Phase 5. Small curated first expansion wave
- Add a small reviewed batch of new supported special mods using the new onboarding flow.
- Do not start with large gameplay mods or option-heavy installers.

## Public Interfaces and Types

- Add `versionStrategy` to guided special-mod profile data.
- Add `versionSignals` to `FileInsights`.
- Add additive compare-confidence fields to version summaries.
- Keep current Tauri command names stable.
- Keep current special-mod decision types stable where possible; extend them instead of replacing them.
- Add internal shared models:
  - `VersionSubject`
  - `VersionResolution`
  - `WatchSource`
  - `WatchResult`

## Test Plan

### Backend
- Signal collection tests for:
  - filename
  - payload
  - ignore rules
  - rewrite rules
  - derived hints
- Subject matching tests for:
  - strong match
  - weak match
  - wrong match rejected
- Compare tests for:
  - same version by signature
  - same version by trusted clue
  - newer
  - older
  - equal version but mismatched signatures
  - weak match becomes `unknown`
- Special-mod regression tests for all built-ins.

### Real Desktop
- Inbox checks for:
  - regular mod same-version duplicate
  - regular mod newer download
  - regular mod older download
  - CC file with weak identity stays cautious
  - supported special-mod same-version reinstall
  - supported special-mod blocked flow
  - supported special-mod safe apply
- Library checks for:
  - installed version summary
  - local evidence display
  - watch status display
  - exact update vs possible creator-page update wording

### Performance
- Queue stays light.
- Full detail stays on selected item only.
- No network on the local compare hot path.
- Existing perf tracing must confirm the rework does not bring Inbox slowness back.

## Assumptions and Defaults

- Guided install remains special-mod-only.
- Broad version compare should cover all mods and CC where the local match is strong enough.
- Weak matches should stay cautious and use `unknown`.
- Creator page watches are allowed, but only for official or user-approved public pages.
- The large reference mod index remains frozen and non-runtime.
- The first supported expansion wave after the foundation lands should stay small and reviewed.
