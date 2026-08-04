CREATE TABLE IF NOT EXISTS game_installation_profiles (
    profile_id TEXT PRIMARY KEY,
    profile_name TEXT NOT NULL,
    game_id TEXT NOT NULL CHECK (game_id IN ('sims4')),
    operating_environment TEXT NOT NULL CHECK (
        operating_environment IN (
            'native_windows',
            'native_macos',
            'native_linux',
            'wine',
            'proton',
            'lutris',
            'unknown'
        )
    ),
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'valid', 'needs_review', 'unavailable')
    ),
    detection_method TEXT NOT NULL CHECK (
        detection_method IN ('manual', 'legacy_settings_migration', 'platform_candidate')
    ),
    detection_evidence_json TEXT NOT NULL DEFAULT '{}',
    confirmation_state TEXT NOT NULL CHECK (
        confirmation_state IN ('unconfirmed', 'confirmed', 'confirmation_stale')
    ),
    confirmed_at TEXT,
    last_validated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS game_installation_roots (
    profile_id TEXT NOT NULL REFERENCES game_installation_profiles(profile_id) ON DELETE CASCADE,
    root_id TEXT NOT NULL,
    root_role TEXT NOT NULL CHECK (
        root_role IN (
            'game_user_data',
            'installed_mods',
            'installed_tray',
            'intake_downloads',
            'intake_reject'
        )
    ),
    configured_path TEXT NOT NULL,
    required INTEGER NOT NULL DEFAULT 0 CHECK (required IN (0, 1)),
    validation_state TEXT NOT NULL CHECK (
        validation_state IN ('unvalidated', 'valid', 'needs_review', 'unavailable')
    ),
    filesystem_capabilities_json TEXT NOT NULL DEFAULT '{}',
    last_validated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (profile_id, root_id)
);

CREATE INDEX IF NOT EXISTS idx_game_installation_profiles_game_id
    ON game_installation_profiles (game_id);
CREATE INDEX IF NOT EXISTS idx_game_installation_profiles_status
    ON game_installation_profiles (status);
CREATE INDEX IF NOT EXISTS idx_game_installation_roots_role
    ON game_installation_roots (root_role);
