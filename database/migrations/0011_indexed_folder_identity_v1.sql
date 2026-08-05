CREATE INDEX IF NOT EXISTS idx_library_folders_installation_identity
    ON library_folders (
        installation_profile_id,
        installation_root_id,
        profile_relative_path
    );

CREATE INDEX IF NOT EXISTS idx_library_folders_installation_comparison
    ON library_folders (
        installation_profile_id,
        installation_root_id,
        profile_relative_path_key
    );

CREATE TRIGGER IF NOT EXISTS trg_library_folders_installation_identity_all_or_nothing_insert
BEFORE INSERT ON library_folders
WHEN NOT (
    (NEW.installation_profile_id IS NULL
        AND NEW.installation_root_id IS NULL
        AND NEW.profile_relative_path IS NULL)
    OR
    (NEW.installation_profile_id IS NOT NULL
        AND TRIM(NEW.installation_profile_id) != ''
        AND NEW.installation_root_id IS NOT NULL
        AND TRIM(NEW.installation_root_id) != ''
        AND NEW.profile_relative_path IS NOT NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'library folder installation identity must be fully assigned or fully unassigned');
END;

CREATE TRIGGER IF NOT EXISTS trg_library_folders_installation_identity_all_or_nothing_update
BEFORE UPDATE OF installation_profile_id, installation_root_id, profile_relative_path ON library_folders
WHEN NOT (
    (NEW.installation_profile_id IS NULL
        AND NEW.installation_root_id IS NULL
        AND NEW.profile_relative_path IS NULL)
    OR
    (NEW.installation_profile_id IS NOT NULL
        AND TRIM(NEW.installation_profile_id) != ''
        AND NEW.installation_root_id IS NOT NULL
        AND TRIM(NEW.installation_root_id) != ''
        AND NEW.profile_relative_path IS NOT NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'library folder installation identity must be fully assigned or fully unassigned');
END;

CREATE TRIGGER IF NOT EXISTS trg_library_folders_installation_comparison_key_insert
BEFORE INSERT ON library_folders
WHEN NEW.profile_relative_path_key IS NOT NULL
    AND (
        TRIM(NEW.profile_relative_path_key) = ''
        OR TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
        OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
        OR TRIM(COALESCE(NEW.profile_relative_path, '')) = ''
    )
BEGIN
    SELECT RAISE(ABORT, 'library folder comparison key requires non-root installation identity');
END;

CREATE TRIGGER IF NOT EXISTS trg_library_folders_installation_comparison_key_update
BEFORE UPDATE OF installation_profile_id, installation_root_id, profile_relative_path, profile_relative_path_key ON library_folders
WHEN NEW.profile_relative_path_key IS NOT NULL
    AND (
        TRIM(NEW.profile_relative_path_key) = ''
        OR TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
        OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
        OR TRIM(COALESCE(NEW.profile_relative_path, '')) = ''
    )
BEGIN
    SELECT RAISE(ABORT, 'library folder comparison key requires non-root installation identity');
END;
