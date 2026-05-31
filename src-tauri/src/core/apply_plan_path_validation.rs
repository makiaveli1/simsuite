use std::{collections::HashMap, fs, io::ErrorKind, path::Path};

use crate::models::{
    ApplyPlanConflictStatus, ApplyPlanValidationStatus, LibrarySettings, PersistedApplyPlanItem,
    PersistedApplyPlanItemStatus,
};

#[derive(Debug, Clone)]
pub struct PathValidationOutcome {
    pub validation_status: ApplyPlanValidationStatus,
    pub conflict_status: ApplyPlanConflictStatus,
    pub reason: String,
    pub required_next_step: String,
}

impl PathValidationOutcome {
    fn valid(reason: impl Into<String>, required_next_step: impl Into<String>) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::ValidPreviewOnly,
            conflict_status: ApplyPlanConflictStatus::None,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn unsafe_destination(
        conflict_status: ApplyPlanConflictStatus,
        reason: impl Into<String>,
        required_next_step: impl Into<String>,
    ) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::UnsafeDestination,
            conflict_status,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn unsafe_source(
        conflict_status: ApplyPlanConflictStatus,
        reason: impl Into<String>,
        required_next_step: impl Into<String>,
    ) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::UnsafeSource,
            conflict_status,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn missing_source_root(
        reason: impl Into<String>,
        required_next_step: impl Into<String>,
    ) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::MissingSourceRoot,
            conflict_status: ApplyPlanConflictStatus::Unsupported,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn missing_root(reason: impl Into<String>, required_next_step: impl Into<String>) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::MissingDestinationRoot,
            conflict_status: ApplyPlanConflictStatus::Unsupported,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn cross_root(reason: impl Into<String>, required_next_step: impl Into<String>) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::UnsupportedCrossRoot,
            conflict_status: ApplyPlanConflictStatus::CrossRootBlocked,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn destination_exists(
        reason: impl Into<String>,
        required_next_step: impl Into<String>,
    ) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::DestinationExists,
            conflict_status: ApplyPlanConflictStatus::DestinationExists,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }

    fn error(reason: impl Into<String>, required_next_step: impl Into<String>) -> Self {
        Self {
            validation_status: ApplyPlanValidationStatus::Error,
            conflict_status: ApplyPlanConflictStatus::PermissionUnknown,
            reason: reason.into(),
            required_next_step: required_next_step.into(),
        }
    }
}

pub struct DestinationValidationInput<'a> {
    pub settings: &'a LibrarySettings,
    pub destination_path: &'a str,
    pub destination_root_name: &'a str,
    pub indexed_source_root_name: &'a str,
    pub saved_current_root_name: &'a str,
}

pub fn validate_source_under_root(
    source_path: &str,
    source_root: Option<&str>,
) -> PathValidationOutcome {
    let Some(source_root) = source_root.map(str::trim).filter(|root| !root.is_empty()) else {
        return PathValidationOutcome::missing_source_root(
            "Configured source root is not available for this indexed file.",
            "Configure the source root and regenerate the preview from current Library data.",
        );
    };

    if let Err(reason) = validate_path_shape(source_path) {
        return PathValidationOutcome::unsafe_source(
            ApplyPlanConflictStatus::Unsupported,
            format!("Source path is not eligible for future Apply validation: {reason}"),
            "Regenerate the preview from current Library data.",
        );
    }
    if let Err(reason) = validate_path_shape(source_root) {
        return PathValidationOutcome::unsafe_source(
            ApplyPlanConflictStatus::Unsupported,
            format!("Configured source root is not eligible for future Apply validation: {reason}"),
            "Review Library root settings before future validation.",
        );
    }

    match path_is_symlink(source_root) {
        Ok(true) => {
            return PathValidationOutcome::unsafe_source(
                ApplyPlanConflictStatus::Unsupported,
                "Configured source root is a symlink/reparse-style path, which is unsupported for future Apply validation.",
                "Configure a direct Library root before future validation.",
            );
        }
        Ok(false) => {}
        Err(message) => {
            return PathValidationOutcome::error(
                format!("Configured source root could not be checked: {message}"),
                "Review Library root permissions before future validation.",
            );
        }
    }

    match path_exists(source_path) {
        Ok(true) => {}
        Ok(false) => {
            return PathValidationOutcome {
                validation_status: ApplyPlanValidationStatus::MissingSource,
                conflict_status: ApplyPlanConflictStatus::SourceMissing,
                reason: "The indexed source file was not found on disk.".to_owned(),
                required_next_step: "Rescan or regenerate the preview from current Library data."
                    .to_owned(),
            };
        }
        Err(message) => {
            return PathValidationOutcome::error(
                format!("Source existence could not be checked: {message}"),
                "Review source file permissions before future validation.",
            );
        }
    }

    if path_has_symlink_component(source_path, source_root) {
        return PathValidationOutcome::unsafe_source(
            ApplyPlanConflictStatus::Unsupported,
            "Source path includes a symlink/reparse-style component, which is unsupported for future Apply validation.",
            "Regenerate the preview from a direct file path under a configured root.",
        );
    }

    if !canonical_path_is_under_root(source_path, source_root) {
        return PathValidationOutcome::unsafe_source(
            ApplyPlanConflictStatus::Unsupported,
            "Source path cannot be proven under the configured Library root.",
            "Rescan from the configured Library root before future validation.",
        );
    }

    PathValidationOutcome::valid(
        "Source path is still present under the configured Library root.",
        "Continue destination validation before any future confirmation.",
    )
}

pub fn validate_destination(input: DestinationValidationInput<'_>) -> PathValidationOutcome {
    let Some(destination_root) = configured_root(input.settings, input.destination_root_name)
    else {
        return PathValidationOutcome::missing_root(
            format!(
                "Destination root `{}` is not configured.",
                input.destination_root_name
            ),
            "Configure the destination root before future validation.",
        );
    };

    if let Err(reason) = validate_path_shape(input.destination_path) {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::Unsupported,
            format!("Destination path is not eligible for future Apply validation: {reason}"),
            "Regenerate the preview with a safe destination path.",
        );
    }
    if let Err(reason) = validate_path_shape(destination_root) {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::Unsupported,
            format!(
                "Configured destination root is not eligible for future Apply validation: {reason}"
            ),
            "Review Library root settings before future validation.",
        );
    }

    match path_is_symlink(destination_root) {
        Ok(true) => {
            return PathValidationOutcome::unsafe_destination(
                ApplyPlanConflictStatus::Unsupported,
                "Configured destination root is a symlink/reparse-style path, which is unsupported for future Apply validation.",
                "Configure a direct Mods/Tray root before future validation.",
            );
        }
        Ok(false) => {}
        Err(message) => {
            return PathValidationOutcome::error(
                format!("Configured destination root could not be checked: {message}"),
                "Review Library root permissions before future validation.",
            );
        }
    }

    if !logical_path_is_under_root(input.destination_path, destination_root) {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::Unsupported,
            "Destination path cannot be proven under the configured Mods/Tray root.",
            "Regenerate the preview with a safe destination path.",
        );
    }

    if !input
        .indexed_source_root_name
        .eq_ignore_ascii_case(input.destination_root_name)
        || (!input
            .saved_current_root_name
            .eq_ignore_ascii_case("unknown")
            && !input
                .saved_current_root_name
                .eq_ignore_ascii_case(input.destination_root_name))
    {
        return PathValidationOutcome::cross_root(
            "Cross-root movement is not supported by the validation preview.",
            "Keep future validation within one configured Library root.",
        );
    }

    match symlink_metadata_optional(input.destination_path) {
        Ok(Some(metadata)) if metadata_is_symlink_or_reparse(&metadata) => {
            return PathValidationOutcome::unsafe_destination(
                ApplyPlanConflictStatus::Unsupported,
                "Destination path already exists as a symlink/reparse-style entry, which is unsupported for future Apply validation.",
                "Resolve the destination conflict before future validation.",
            );
        }
        Ok(Some(_)) => {
            return PathValidationOutcome::destination_exists(
                "A file or folder already exists at the saved destination path.",
                "Resolve the destination conflict before future validation.",
            );
        }
        Ok(None) => {}
        Err(message) => {
            return PathValidationOutcome::error(
                format!("Destination conflict could not be checked: {message}"),
                "Review destination permissions before future validation.",
            );
        }
    }

    let Some(parent) = Path::new(input.destination_path).parent() else {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::FolderMissing,
            "Destination parent folder could not be determined.",
            "Regenerate the preview with a destination inside an existing folder.",
        );
    };
    let parent_string = parent.to_string_lossy().to_string();
    match path_exists(&parent_string) {
        Ok(true) => {}
        Ok(false) => {
            return PathValidationOutcome::unsafe_destination(
                ApplyPlanConflictStatus::FolderMissing,
                "Destination parent folder does not exist; folder creation is not part of validation V1.",
                "Regenerate the preview or create/review the destination folder before future validation.",
            );
        }
        Err(message) => {
            return PathValidationOutcome::error(
                format!("Destination parent folder could not be checked: {message}"),
                "Review destination permissions before future validation.",
            );
        }
    }

    if path_has_symlink_component(&parent_string, destination_root) {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::Unsupported,
            "Destination path includes a symlink/reparse-style component, which is unsupported for future Apply validation.",
            "Regenerate the preview with a direct destination under the configured root.",
        );
    }

    if !canonical_path_is_under_root(&parent_string, destination_root) {
        return PathValidationOutcome::unsafe_destination(
            ApplyPlanConflictStatus::Unsupported,
            "Destination parent folder cannot be proven under the configured Mods/Tray root.",
            "Regenerate the preview with a destination under a configured root.",
        );
    }

    PathValidationOutcome::valid(
        "No current validation blocker was found for this preview item.",
        "Backup/restore and confirmation work must exist before this could go further.",
    )
}

pub(crate) fn is_future_move_candidate(item: &PersistedApplyPlanItem) -> bool {
    item.file_id.is_some()
        && !item.blocked
        && !item.review_only
        && item.item_status != PersistedApplyPlanItemStatus::Blocked
        && item.item_status != PersistedApplyPlanItemStatus::ReviewOnly
        && item.action_kind.eq_ignore_ascii_case("suggest_move")
        && !item.evidence_level.eq_ignore_ascii_case("heuristic")
        && !has_duplicate_review_signal_or_blocker(item)
        && !item
            .confidence_label
            .as_deref()
            .is_some_and(|label| label.eq_ignore_ascii_case("heuristic"))
}

fn has_duplicate_review_signal_or_blocker(item: &PersistedApplyPlanItem) -> bool {
    item.blockers.iter().any(|blocker| {
        contains_duplicate_text(&blocker.reason_code)
            || contains_duplicate_text(&blocker.message)
            || contains_duplicate_text(&blocker.blocker_kind)
    }) || item.signals.iter().any(|signal| {
        contains_duplicate_text(&signal.signal_kind)
            || contains_duplicate_text(&signal.signal_label)
            || signal
                .signal_value
                .as_deref()
                .is_some_and(contains_duplicate_text)
    })
}

fn contains_duplicate_text(value: &str) -> bool {
    value.to_ascii_lowercase().contains("duplicate")
}

pub fn detect_destination_conflicts(
    items: &[PersistedApplyPlanItem],
) -> HashMap<i64, ApplyPlanConflictStatus> {
    let mut by_canonical: HashMap<String, Vec<(i64, String)>> = HashMap::new();
    for item in items {
        if !is_future_move_candidate(item) {
            continue;
        }
        let Some(destination_path) = item
            .destination_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let canonical = normalized_path_key(destination_path);
        by_canonical
            .entry(canonical)
            .or_default()
            .push((item.id, normalize_separators_for_display(destination_path)));
    }

    let mut conflicts = HashMap::new();
    for destinations in by_canonical.values() {
        if destinations.len() < 2 {
            continue;
        }
        let first_path = &destinations[0].1;
        let conflict_status = if destinations.iter().any(|(_, path)| path != first_path) {
            ApplyPlanConflictStatus::CaseConflict
        } else {
            ApplyPlanConflictStatus::SameNameConflict
        };
        for (item_id, _) in destinations {
            conflicts.insert(*item_id, conflict_status.clone());
        }
    }
    conflicts
}

pub(crate) fn configured_root<'a>(
    settings: &'a LibrarySettings,
    root_name: &str,
) -> Option<&'a str> {
    match root_name {
        value if value.eq_ignore_ascii_case("mods") => settings.mods_path.as_deref(),
        value if value.eq_ignore_ascii_case("tray") => settings.tray_path.as_deref(),
        _ => None,
    }
}

pub(crate) fn normalized_path_key(path: &str) -> String {
    normalize_separators_for_display(path).to_ascii_lowercase()
}

fn normalize_separators_for_display(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}

fn path_exists(path: &str) -> Result<bool, String> {
    Path::new(path)
        .try_exists()
        .map_err(|error| error.to_string())
}

fn path_is_symlink(path: &str) -> Result<bool, String> {
    symlink_metadata_optional(path).map(|metadata| {
        metadata
            .map(|metadata| metadata_is_symlink_or_reparse(&metadata))
            .unwrap_or(false)
    })
}

fn metadata_is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || metadata_has_windows_reparse_attribute(metadata)
}

#[cfg(windows)]
fn metadata_has_windows_reparse_attribute(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_has_windows_reparse_attribute(_metadata: &fs::Metadata) -> bool {
    false
}

fn symlink_metadata_optional(path: &str) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(Path::new(path)) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn logical_path_is_under_root(path: &str, root: &str) -> bool {
    if validate_path_shape(path).is_err() || validate_path_shape(root).is_err() {
        return false;
    }
    let path = normalized_path_key(path);
    let root = normalized_path_key(root);
    path == root || path.starts_with(&format!("{root}/"))
}

fn canonical_path_is_under_root(path: &str, root: &str) -> bool {
    if !logical_path_is_under_root(path, root) {
        return false;
    }

    let path_ref = Path::new(path);
    let root_ref = Path::new(root);
    if path_ref.is_absolute() && root_ref.is_absolute() {
        match (fs::canonicalize(path_ref), fs::canonicalize(root_ref)) {
            (Ok(path_canonical), Ok(root_canonical)) => path_canonical.starts_with(root_canonical),
            _ => false,
        }
    } else {
        true
    }
}

fn path_has_symlink_component(path: &str, root: &str) -> bool {
    let path_ref = Path::new(path);
    let root_ref = Path::new(root);
    if !path_ref.is_absolute() || !root_ref.is_absolute() {
        return false;
    }
    if !logical_path_is_under_root(path, root) {
        return false;
    }

    let Ok(relative_path) = path_ref.strip_prefix(root_ref) else {
        return false;
    };

    let mut current = root_ref.to_path_buf();
    for component in relative_path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata_is_symlink_or_reparse(&metadata) {
                return true;
            }
        }
    }
    false
}

fn validate_path_shape(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is empty".to_owned());
    }
    if trimmed.contains('\0') {
        return Err("path contains a null byte".to_owned());
    }

    let normalized = trimmed.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("//?/") || lower.starts_with("//./") || lower.starts_with("//") {
        return Err("UNC or device namespace paths are unsupported".to_owned());
    }

    let bytes = normalized.as_bytes();
    let has_windows_drive =
        bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic();
    let has_drive_relative_prefix = bytes.len() >= 2
        && bytes[1] == b':'
        && !has_windows_drive
        && bytes[0].is_ascii_alphabetic();
    if has_drive_relative_prefix {
        return Err("drive-relative paths are unsupported".to_owned());
    }
    if !normalized.starts_with('/') && !has_windows_drive {
        return Err("relative paths are unsupported".to_owned());
    }

    let path_without_drive = if has_windows_drive {
        &normalized[3..]
    } else {
        normalized.trim_start_matches('/')
    };

    for component in path_without_drive.split('/') {
        if component.is_empty() {
            return Err("empty path components are unsupported".to_owned());
        }
        if component == "." || component == ".." {
            return Err("dot or parent-directory path components are unsupported".to_owned());
        }
        if component.ends_with(' ') || component.ends_with('.') {
            return Err(
                "Windows trailing-space or trailing-dot path components are unsupported".to_owned(),
            );
        }
        if component.contains(':') {
            return Err("alternate data stream style path components are unsupported".to_owned());
        }
        if is_reserved_windows_name(component) {
            return Err("reserved Windows device names are unsupported".to_owned());
        }
    }

    Ok(())
}

fn is_reserved_windows_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;

    #[cfg(unix)]
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn normalized_path_keys_are_separator_and_case_insensitive() {
        assert_eq!(
            normalized_path_key("C:\\Sims\\Mods\\CAS\\Hair.package"),
            "c:/sims/mods/cas/hair.package"
        );
    }

    #[test]
    fn logical_root_check_rejects_prefix_spoofing() {
        assert!(!logical_path_is_under_root(
            "C:/Sims/Mods2/a.package",
            "C:/Sims/Mods"
        ));
        assert!(logical_path_is_under_root(
            "C:/Sims/Mods/a.package",
            "C:/Sims/Mods"
        ));
    }

    #[test]
    fn path_shape_rejects_windows_parent_and_device_paths() {
        assert!(validate_path_shape("C:\\Sims\\Mods\\..\\Elsewhere\\x.package").is_err());
        assert!(validate_path_shape("\\\\?\\C:\\Sims\\Mods\\x.package").is_err());
        assert!(validate_path_shape("C:relative\\x.package").is_err());
        assert!(validate_path_shape("C:\\Sims\\Mods\\x.package:stream").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn destination_validation_rejects_configured_destination_root_symlink() {
        let temp = tempdir().expect("tempdir");
        let real_mods_root = temp.path().join("RealMods");
        let linked_mods_root = temp.path().join("ModsLink");
        let destination_path = linked_mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(real_mods_root.join("CAS")).expect("real destination parent");
        std::os::unix::fs::symlink(&real_mods_root, &linked_mods_root)
            .expect("configured destination root symlink");

        let settings = LibrarySettings {
            mods_path: Some(linked_mods_root.to_string_lossy().to_string()),
            ..Default::default()
        };

        let outcome = validate_destination(DestinationValidationInput {
            settings: &settings,
            destination_path: &destination_path.to_string_lossy(),
            destination_root_name: "mods",
            indexed_source_root_name: "mods",
            saved_current_root_name: "mods",
        });

        assert_eq!(
            outcome.validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
        assert!(outcome.reason.contains("root") && outcome.reason.contains("symlink"));
    }
}
