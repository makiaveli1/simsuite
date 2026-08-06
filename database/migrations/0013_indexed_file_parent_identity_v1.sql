CREATE INDEX IF NOT EXISTS idx_files_installation_parent_identity
    ON files (
        source_location,
        installation_profile_id,
        installation_root_id,
        profile_parent_relative_path
    )
    WHERE profile_parent_relative_path IS NOT NULL;

CREATE TRIGGER IF NOT EXISTS files_parent_identity_insert_guard
BEFORE INSERT ON files
WHEN
    (NEW.profile_parent_relative_path IS NULL
        AND NEW.profile_parent_relative_path_key IS NOT NULL)
    OR
    (NEW.profile_parent_relative_path IS NOT NULL
        AND (
            TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
            OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
            OR NEW.profile_relative_path IS NULL
        ))
    OR
    (NEW.profile_parent_relative_path_key IS NOT NULL
        AND (
            TRIM(NEW.profile_parent_relative_path_key) = ''
            OR TRIM(COALESCE(NEW.profile_parent_relative_path, '')) = ''
        ))
BEGIN
    SELECT RAISE(ABORT, 'file parent identity must be null or belong to complete indexed file identity');
END;

CREATE TRIGGER IF NOT EXISTS files_parent_identity_update_guard
BEFORE UPDATE OF
    installation_profile_id,
    installation_root_id,
    profile_relative_path,
    profile_parent_relative_path,
    profile_parent_relative_path_key
ON files
WHEN
    (NEW.profile_parent_relative_path IS NULL
        AND NEW.profile_parent_relative_path_key IS NOT NULL)
    OR
    (NEW.profile_parent_relative_path IS NOT NULL
        AND (
            TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
            OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
            OR NEW.profile_relative_path IS NULL
        ))
    OR
    (NEW.profile_parent_relative_path_key IS NOT NULL
        AND (
            TRIM(NEW.profile_parent_relative_path_key) = ''
            OR TRIM(COALESCE(NEW.profile_parent_relative_path, '')) = ''
        ))
BEGIN
    SELECT RAISE(ABORT, 'file parent identity must be null or belong to complete indexed file identity');
END;
