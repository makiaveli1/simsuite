CREATE INDEX IF NOT EXISTS idx_files_installation_comparison
    ON files (
        installation_profile_id,
        installation_root_id,
        profile_relative_path_key
    );

CREATE TRIGGER IF NOT EXISTS trg_files_installation_comparison_key_insert
BEFORE INSERT ON files
WHEN NEW.profile_relative_path_key IS NOT NULL
    AND (
        TRIM(NEW.profile_relative_path_key) = ''
        OR TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
        OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
        OR TRIM(COALESCE(NEW.profile_relative_path, '')) = ''
    )
BEGIN
    SELECT RAISE(ABORT, 'files comparison key requires complete installation identity');
END;

CREATE TRIGGER IF NOT EXISTS trg_files_installation_comparison_key_update
BEFORE UPDATE OF installation_profile_id, installation_root_id, profile_relative_path, profile_relative_path_key ON files
WHEN NEW.profile_relative_path_key IS NOT NULL
    AND (
        TRIM(NEW.profile_relative_path_key) = ''
        OR TRIM(COALESCE(NEW.installation_profile_id, '')) = ''
        OR TRIM(COALESCE(NEW.installation_root_id, '')) = ''
        OR TRIM(COALESCE(NEW.profile_relative_path, '')) = ''
    )
BEGIN
    SELECT RAISE(ABORT, 'files comparison key requires complete installation identity');
END;
