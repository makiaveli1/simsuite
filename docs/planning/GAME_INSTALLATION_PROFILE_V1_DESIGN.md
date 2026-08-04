# Game Installation Profile V1 Design

**Date:** 4 August 2026  
**Status:** Implementation contract for Sprint 1  
**Related direction:** `SIMSUITE_APP_DIRECTION_EXECUTION_PLAN_V1.md`

## 1. Purpose

Game Installation Profile V1 replaces the assumption that SimSuite has one global Mods, Tray, and Downloads path.

A profile represents one player-confirmed game environment and its named roots. It gives SimSuite stable authority for path identity, placement rules, scanning, previews, and future transaction records.

V1 is configuration and validation infrastructure only. It does not enable Apply, Restore, deletion, replacement, quarantine, cleanup, or automatic mutation.

## 2. Naming rule

The repository already uses `install_profiles.json` for guided installation recipes for specific mods such as MCCC.

To avoid collision:

- use **Game Installation Profile** for a game environment and its filesystem roots;
- keep **Guided Install Profile** for mod-specific installation recipes;
- use `game_installation_profile` in database, Rust, API, and documentation names.

## 3. V1 product behavior

A player can:

- see the current Sims 4 profile;
- create a profile by choosing folders manually;
- review what SimSuite detected or was told;
- validate that required roots exist and belong together;
- explicitly confirm a profile;
- select which confirmed profile is active.

SimSuite must not:

- silently activate an ambiguous candidate;
- create missing game folders automatically;
- move or reorganize content;
- assume operating-system defaults are correct;
- infer root authority from path text such as a folder named `Tray`;
- remove a profile that already owns indexed or transaction data without a future migration workflow.

## 4. Domain model

### GameInstallationProfile

```text
profile_id
profile_name
game_id
operating_environment
status
detection_method
detection_evidence
confirmation_state
confirmed_at
last_validated_at
created_at
updated_at
```

Recommended V1 values:

```text
game_id:
  sims4

operating_environment:
  native_windows
  native_macos
  native_linux
  wine
  proton
  lutris
  unknown

status:
  draft
  valid
  needs_review
  unavailable

confirmation_state:
  unconfirmed
  confirmed
  confirmation_stale

detection_method:
  manual
  legacy_settings_migration
  platform_candidate
```

### GameInstallationRoot

```text
profile_id
root_id
root_role
configured_path
required
validation_state
filesystem_capabilities
last_validated_at
created_at
updated_at
```

Recommended V1 root IDs and roles:

```text
user_data  -> game_user_data
mods       -> installed_mods
tray       -> installed_tray
downloads  -> intake_downloads
reject     -> intake_reject
```

`root_id` is authority. Folder names and path text are display evidence only.

### GameInstallationCandidate

Candidates are temporary read-only DTOs. They are not saved as confirmed profiles until the player chooses one.

```text
candidate_id
game_id
operating_environment
suggested_name
suggested_roots
detection_evidence
confidence
warnings
```

Confidence is advisory and must never activate a profile automatically.

## 5. Database schema

### `game_installation_profiles`

```sql
CREATE TABLE game_installation_profiles (
    profile_id TEXT PRIMARY KEY,
    profile_name TEXT NOT NULL,
    game_id TEXT NOT NULL,
    operating_environment TEXT NOT NULL,
    status TEXT NOT NULL,
    detection_method TEXT NOT NULL,
    detection_evidence_json TEXT NOT NULL DEFAULT '{}',
    confirmation_state TEXT NOT NULL,
    confirmed_at TEXT,
    last_validated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Required checks should constrain known enum values while allowing future migrations to add values deliberately.

### `game_installation_roots`

```sql
CREATE TABLE game_installation_roots (
    profile_id TEXT NOT NULL REFERENCES game_installation_profiles(profile_id) ON DELETE CASCADE,
    root_id TEXT NOT NULL,
    root_role TEXT NOT NULL,
    configured_path TEXT NOT NULL,
    required INTEGER NOT NULL DEFAULT 0,
    validation_state TEXT NOT NULL,
    filesystem_capabilities_json TEXT NOT NULL DEFAULT '{}',
    last_validated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (profile_id, root_id)
);
```

Indexes:

```text
game_installation_profiles(game_id)
game_installation_profiles(status)
game_installation_roots(root_role)
```

### Active profile

V1 stores the selected profile ID in existing `app_settings`:

```text
active_game_installation_profile_id
```

This avoids a fragile `is_active` flag across multiple rows and keeps one explicit selection per current app installation.

## 6. Legacy settings migration

Current settings remain:

```text
mods_path
tray_path
downloads_path
```

Migration V8 should:

1. create the two profile tables;
2. check whether a profile already exists;
3. read non-empty legacy paths from `app_settings`;
4. if at least one legacy path exists, create one profile with:
   - name: `Current Sims 4 setup`;
   - game: `sims4`;
   - environment: current native platform where known, otherwise `unknown`;
   - detection method: `legacy_settings_migration`;
   - confirmation state: `confirmed`, because the player previously saved these paths;
5. add only roots that have non-empty legacy values;
6. store the new profile ID in `active_game_installation_profile_id`;
7. leave the original legacy settings intact during the compatibility period;
8. remain idempotent across repeated startup.

The migration must not probe, create, move, rename, or delete user folders.

A missing path may produce an unavailable or needs-review validation state after runtime validation, but must not erase the saved configuration.

## 7. Compatibility layer

The first profile implementation should not force every subsystem to change at once.

### Read compatibility

`get_library_settings()` should eventually derive a `LibrarySettings` view from the active profile:

```text
mods_path      <- root_id `mods`
tray_path      <- root_id `tray`
downloads_path <- root_id `downloads`
reject_folder  <- root_id `reject`
```

During transition:

1. environment-variable test overrides remain highest priority;
2. an active profile is the preferred stored source;
3. legacy `app_settings` are the fallback when no profile exists.

### Write compatibility

Until the profile UI replaces the old folder picker:

- `save_library_paths` remains available;
- saving paths updates the legacy settings;
- if the active profile was created by legacy migration, it updates matching profile roots in the same database transaction;
- manually created or discovered profiles must not be silently overwritten by the old command;
- the response remains the current `LibrarySettings` shape so existing frontend code does not break.

This dual-write bridge is temporary and must be removed after the profile UI becomes the only settings owner.

## 8. Runtime validation

Profile validation is read-only.

For each configured root, it should record:

- absolute-path requirement;
- exists or missing;
- file versus directory;
- readable metadata;
- canonical root identity;
- root-local case sensitivity;
- symlink or alias observations where supported;
- relationship to the user-data root where the Sims4Adapter requires it;
- whether required Mods and Tray roots form one coherent Sims 4 profile.

Validation outcomes:

```text
valid
needs_review
unavailable
```

Unknown filesystem capability or unreadable metadata must not be guessed. It produces a blocker or review state.

## 9. Sims4Adapter boundary

The profile service owns generic profile persistence and root validation.

The Sims4Adapter owns game-specific rules:

- expected user-data root layout;
- Mods and Tray root roles;
- whether roots are coherent for one Sims 4 setup;
- script-depth and placement rules;
- supported file types;
- default candidate layout descriptions;
- game-specific readiness evidence.

The platform layer owns:

- Documents and Downloads candidate discovery;
- OneDrive or redirected-folder evidence;
- Wine, Proton, and Lutris environment evidence;
- filesystem capabilities;
- canonical containment;
- Finder, Explorer, or Linux file-manager behavior.

## 10. API sequence

V1 backend API should be introduced in this order:

1. `list_game_installation_profiles`
2. `get_active_game_installation_profile`
3. `validate_game_installation_profile`
4. `create_manual_game_installation_profile`
5. `set_active_game_installation_profile`
6. `detect_game_installation_candidates`

Candidate detection should arrive after manual creation and validation are proven.

Profile deletion, automatic folder creation, profile merging, and relocation automation are not V1 APIs.

## 11. Frontend sequence

### First UI slice

Add a calm profile section in Settings:

- active profile name;
- game and environment;
- Mods, Tray, and Downloads roots;
- validation status;
- `Review profile` action;
- `Choose folders manually` action.

### Later setup journey

Home can guide an unconfigured user into profile setup, but it should not duplicate the complete Settings editor.

Casual, Seasoned, and Creator views show the same validation result. They differ only in wording and evidence depth.

## 12. Required tests

### Migration

- fresh database creates profile tables;
- existing legacy paths backfill one profile;
- empty legacy settings do not create a fake profile;
- repeated initialization is idempotent;
- active profile setting points to the backfilled profile;
- original legacy settings remain intact.

### Persistence

- manual profile round-trip;
- multiple roots per profile;
- one active profile setting;
- unknown enum values fail safely or map to explicit unknown states;
- malformed evidence or capabilities JSON fails closed.

### Validation

- valid native macOS profile;
- missing Mods root;
- custom Tray root;
- relative configured path;
- unreadable metadata;
- root-local case sensitivity unknown;
- external symlink escape;
- roots from two different user-data trees;
- custom manual profile accepted when coherent;
- ambiguous candidate never activates automatically.

### Compatibility

- active profile derives the existing `LibrarySettings` response;
- no-profile state falls back to legacy settings;
- environment overrides still win in tests;
- old `save_library_paths` updates only a legacy-migrated active profile;
- manually created profiles are not silently rewritten.

## 13. First implementation batch

The smallest safe batch contains:

1. migration `0008_game_installation_profiles_v1.sql`;
2. Rust profile/root models and enum parsing;
3. database persistence helpers;
4. idempotent legacy-path backfill;
5. active-profile lookup;
6. derived `LibrarySettings` compatibility read;
7. migration, persistence, and compatibility tests;
8. documentation/status update.

It does not contain:

- candidate discovery;
- profile UI;
- scanner migration;
- Downloads watcher migration;
- profile deletion;
- folder creation;
- transaction execution;
- file mutation.

## 14. Exit criteria

Sprint 1 foundation is complete when:

- existing users keep the same effective paths after migration;
- fresh users without settings do not receive a fabricated profile;
- profiles have stable IDs and explicit root IDs;
- `LibrarySettings` remains compatible for existing frontend and backend callers;
- migration and full project verification pass;
- real Apply, Restore, delete, replace, quarantine, cleanup, and automatic mutation remain unavailable.
