ALTER TABLE apply_plans ADD COLUMN folder_config_json TEXT;
ALTER TABLE apply_plans ADD COLUMN context_trail_json TEXT NOT NULL DEFAULT '[]';
