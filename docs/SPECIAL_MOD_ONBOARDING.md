# Special Mod Onboarding

This file is the shared playbook for adding a new supported special mod to SimSuite.

Use this process so we do not keep rewriting the same logic for every new mod.

## What Stays True

- Inbox is still the place that answers "is this newer, older, the same, or unclear?"
- Library stays focused on local facts and watch status, not guided install buttons.
- Guided install stays special-mod-only.
- Local installed-vs-downloaded truth comes first.
- Official latest checks stay helper-only.
- The large Sims reference list is frozen and reference-only. It is for research, not runtime truth.

## When A Mod Should Become A Supported Special Mod

Support a mod as a special mod when all of these are true:

- it has a clear official source
- it has a predictable install shape
- it is important enough to justify guided handling
- local version clues can be described clearly
- it does not depend on risky scraping or challenge bypasses
- it does not need a complicated option-picker before install

If those rules are not met, keep it as normal Inbox content for now.

## Support Flow

### 1. Add or update the candidate record

Start in [SPECIAL_MOD_CANDIDATES.json](/C:/Users/likwi/OneDrive/Desktop/PROJS/SimSort/docs/SPECIAL_MOD_CANDIDATES.json).

Each record must say:

- what the mod is
- why it matters
- where the official source is
- what install shape it expects
- what version rule ideas are known already
- whether fixtures are ready
- whether it is only a candidate, still being researched, or fully supported

Do not add it to the runtime catalog yet.

### 2. Research the local install shape

Check the real files, not just download pages.

Write down:

- whether it is one script, one package, or a mixed set
- whether root install is valid
- whether one shallow folder is required
- whether partial packs are common
- whether other support mods are required first

Put those notes in the candidate file first.

### 3. Write the version rule

Add a `versionStrategy` block to the guided profile in `seed/install_profiles.json`.

The current rule parts are:

- `incomingOrder`
- `installedOrder`
- `filenamePatterns`
- `payloadPatterns`
- `ignoredPatterns`
- `rewrites`

Use them like this:

- `incomingOrder`: which local clue types to trust first for the download
- `installedOrder`: which local clue types to trust first for the installed copy
- `filenamePatterns`: clean version patterns from names
- `payloadPatterns`: clean version patterns from readable internal files
- `ignoredPatterns`: noisy values to throw away
- `rewrites`: cleanup rules when the raw value is close but not ready yet

Prefer simple rules over clever rules.

### 4. Keep clue collection separate from decision-making

`file_inspector` collects raw local clues.

The shared compare layer decides:

- whether the download matches installed content strongly enough
- what the version likely is
- whether the compare result is safe to trust

Do not put final compare decisions back into the low-level inspector.

### 5. Add fixtures before trusting the profile

Every supported special mod needs fixture coverage for:

- same version
- incoming newer
- incoming older
- incomplete or partial download
- reinstall when same-version is safe
- blocked flow when the pack is incomplete or missing something important

If the mod needs guided apply, test that too.

### 6. Run real desktop checks

The fixture lane must prove the real Tauri app can:

- open Inbox
- load the mod row
- show the right version evidence
- show the right next step
- stay stable on refresh

Do not rely only on browser preview.

### 7. Only then mark it supported

After the profile, fixtures, backend tests, and native desktop checks are ready:

- change the candidate status to `supported`
- add or keep the runtime profile in `seed/install_profiles.json`
- update the repo memory docs

## Version Rule Tips

### Good rule order

Use this general order unless a mod proves it needs something else:

1. file fingerprint
2. trusted inside-file clue
3. trusted filename clue
4. saved family state as installed-side fallback only

### When to stay cautious

Return `unknown` instead of guessing when:

- the installed match is weak
- strong local clues disagree
- version labels match but fingerprints do not
- the only clues are noisy runtime values

### Lumpinou Toolbox lesson

Toolbox is a good reminder that not all mods expose version data the same way.

For mods like that:

- ignore noisy runtime values
- trust the clean name pattern first
- only use internal payload clues if they are stable

That is why the rule system must stay profile-driven.

## Watch Source Rules

Watch sources can be useful, but they are never the final local compare truth.

Allowed watch source types:

- exact mod page
- creator page

Only use a watch source when it is:

- official
- user-approved
- public
- readable without hacks

Do not add:

- challenge bypasses
- login scraping
- brittle unofficial mirrors as truth sources

## Required Docs To Update

After meaningful onboarding work, update:

- [SESSION_HANDOFF.md](/C:/Users/likwi/OneDrive/Desktop/PROJS/SimSort/SESSION_HANDOFF.md)
- [IMPLEMENTATION_STATUS.md](/C:/Users/likwi/OneDrive/Desktop/PROJS/SimSort/docs/IMPLEMENTATION_STATUS.md)
- [ARCHITECTURE.md](/C:/Users/likwi/OneDrive/Desktop/PROJS/SimSort/docs/ARCHITECTURE.md) when behavior or structure changed

## Quick Checklist

- candidate record exists
- official source is clear
- install shape is understood
- `versionStrategy` is written
- backend tests exist
- fixture files exist
- real desktop checks pass
- docs are updated
- then and only then mark the mod `supported`
