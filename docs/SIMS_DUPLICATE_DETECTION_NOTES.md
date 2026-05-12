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
- same normalized package resource fingerprint with payload hashes;
- same normalized script archive fingerprint with entry payload hashes.

Everything else is comparison or review language:

- same filename with different content: `Name match`;
- different version tokens: `Version review`;
- same folder or same pack: `Related hint`;
- same creator/family clues: `Same mod family`, only when bounded and tested.

## v2.1 Guardrail

Same-content duplicate proof must also be a real distinct-file pair:

- both joined file rows exist;
- both file IDs are positive and distinct;
- both hashes are non-empty after trimming;
- normalized hashes match;
- both paths are non-empty;
- normalized paths are not the same Windows path.

Malformed exact rows, self-pairs, same-path rows, and stale rows with missing or mismatched hashes are not user-facing duplicates.

## v3 Package / Script Fingerprints

SimSuite now stores scan-time content fingerprints for packages and scripts when it can compute them safely.

- Package fingerprints use DBPF resource type, group, instance IDs, and resource payload hashes in stable sorted order.
- Script fingerprints use normalized archive entry paths and entry payload hashes in stable sorted order.
- ZIP order, ZIP timestamps, and package container ordering are not duplicate evidence by themselves.
- Resource keys alone are not duplicate proof.
- Corrupt, partial, oversized, or unreadable files produce no duplicate proof.
- Fingerprints are scan-time metadata. Library browsing and the Duplicates route do not parse package/script payloads during render.

These fingerprints only expand exact duplicate proof. They still do not prove safe deletion, missing dependencies, missing meshes, broken content, or update state.

## Sources Reviewed

- [Sims Mod Assistant on Mod The Sims](https://modthesims.info/d/647653/sims-mod-assistant.html?old=1)
- [DBPF format overview](https://simstek.fandom.com/wiki/DBPF)
- [TSR duplicate files help](https://thesimsresource.zendesk.com/hc/en-us/articles/31443197550995-Duplicate-Files-Show-in-My-Game)
- [The Sims 4 Modders Reference file types](https://thesims4moddersreference.org/reference/file-types/)
