CREATE INDEX IF NOT EXISTS idx_library_folders_source_location
    ON library_folders (source_location);

CREATE INDEX IF NOT EXISTS idx_library_folders_source_path
    ON library_folders (source_location, normalized_relative_path);

CREATE INDEX IF NOT EXISTS idx_library_folders_source_parent
    ON library_folders (source_location, parent_normalized_relative_path);

CREATE INDEX IF NOT EXISTS idx_library_folders_source_depth
    ON library_folders (source_location, depth);

CREATE UNIQUE INDEX IF NOT EXISTS uq_library_folders_legacy_source_path
    ON library_folders (source_location, normalized_relative_path)
    WHERE installation_profile_id IS NULL
      AND installation_root_id IS NULL
      AND profile_relative_path IS NULL
      AND profile_relative_path_key IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_library_folders_unkeyed_installation_path
    ON library_folders (
        source_location,
        installation_profile_id,
        installation_root_id,
        normalized_relative_path
    )
    WHERE installation_profile_id IS NOT NULL
      AND installation_root_id IS NOT NULL
      AND profile_relative_path IS NOT NULL
      AND profile_relative_path_key IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_library_folders_keyed_installation_path
    ON library_folders (
        source_location,
        installation_profile_id,
        installation_root_id,
        profile_relative_path_key
    )
    WHERE installation_profile_id IS NOT NULL
      AND installation_root_id IS NOT NULL
      AND profile_relative_path IS NOT NULL
      AND profile_relative_path_key IS NOT NULL;
