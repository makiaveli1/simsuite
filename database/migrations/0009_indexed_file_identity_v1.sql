CREATE INDEX IF NOT EXISTS idx_files_installation_identity
    ON files (installation_profile_id, installation_root_id, profile_relative_path);

CREATE TRIGGER IF NOT EXISTS trg_files_installation_identity_all_or_nothing_insert
BEFORE INSERT ON files
WHEN NOT (
    (NEW.installation_profile_id IS NULL
        AND NEW.installation_root_id IS NULL
        AND NEW.profile_relative_path IS NULL)
    OR
    (NEW.installation_profile_id IS NOT NULL
        AND NEW.installation_root_id IS NOT NULL
        AND NEW.profile_relative_path IS NOT NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'files installation identity must be fully assigned or fully unassigned');
END;

CREATE TRIGGER IF NOT EXISTS trg_files_installation_identity_all_or_nothing_update
BEFORE UPDATE OF installation_profile_id, installation_root_id, profile_relative_path ON files
WHEN NOT (
    (NEW.installation_profile_id IS NULL
        AND NEW.installation_root_id IS NULL
        AND NEW.profile_relative_path IS NULL)
    OR
    (NEW.installation_profile_id IS NOT NULL
        AND NEW.installation_root_id IS NOT NULL
        AND NEW.profile_relative_path IS NOT NULL)
)
BEGIN
    SELECT RAISE(ABORT, 'files installation identity must be fully assigned or fully unassigned');
END;
