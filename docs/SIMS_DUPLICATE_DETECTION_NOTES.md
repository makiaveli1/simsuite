# Sims Duplicate Detection Notes

Date: 2026-05-12

These notes are research guidance for SimSuite's duplicate truth engine. They are not copied code and do not add any external service dependency.

## Practical Findings

- Sims duplicate tools and help pages often separate exact duplicate files from broader conflicts or lookalikes. For example, Sims Mod Assistant describes exact duplicates as copy-paste files with different names, while conflict detection is treated as a separate resource-overlap workflow.
- TSR's duplicate help tells users to verify that items really are duplicates because similar thumbnails can mislead, and to check whether the same creation exists in multiple locations. That supports SimSuite using cautious comparison wording when evidence is not exact.
- DBPF package identity is resource-based. DBPF archives contain indexed entries identified by resource type, group, and instance identifiers. Matching resource keys alone is conflict/review evidence, not same-content proof, because payloads can differ.
- `.ts4script` files are ZIP-like script archives. A deterministic script fingerprint can compare normalized archive entry paths and payload hashes, but SimSuite should only use that if it stores or computes those payload hashes safely during scan.

## SimSuite Rule

SimSuite should only show user-facing `Duplicate` wording when it has deterministic same-content evidence:

- same non-empty full file hash;
- future: same normalized package resource fingerprint with payload hashes;
- future: same normalized script archive fingerprint with entry payload hashes.

Everything else is comparison or review language:

- same filename with different content: `Name match`;
- different version tokens: `Version review`;
- same folder or same pack: `Related hint`;
- same creator/family clues: `Same mod family`, only when bounded and tested.

## Sources Reviewed

- [Sims Mod Assistant on Mod The Sims](https://modthesims.info/d/647653/sims-mod-assistant.html?old=1)
- [DBPF format overview](https://simstek.fandom.com/wiki/DBPF)
- [TSR duplicate files help](https://thesimsresource.zendesk.com/hc/en-us/articles/31443197550995-Duplicate-Files-Show-in-My-Game)
- [The Sims 4 Modders Reference file types](https://thesims4moddersreference.org/reference/file-types/)

