use std::{
    ffi::OsString,
    fmt,
    fs,
    io,
    path::{Component, Path, PathBuf},
};

use super::PlatformId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseSensitivity {
    Sensitive,
    Insensitive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeNormalization {
    Preserved,
    Normalized,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemCapabilities {
    pub case_sensitivity: CaseSensitivity,
    pub unicode_normalization: UnicodeNormalization,
    pub symlink_support: CapabilitySupport,
    pub atomic_rename_support: CapabilitySupport,
    pub trash_availability: CapabilitySupport,
    pub readable: bool,
    pub writable: bool,
    pub available_space: Option<u64>,
    pub cloud_or_network_hint: bool,
    pub removable_hint: bool,
    pub path_length_limit: Option<usize>,
}

impl Default for FilesystemCapabilities {
    fn default() -> Self {
        Self {
            case_sensitivity: CaseSensitivity::Unknown,
            unicode_normalization: UnicodeNormalization::Unknown,
            symlink_support: CapabilitySupport::Unknown,
            atomic_rename_support: CapabilitySupport::Unknown,
            trash_availability: CapabilitySupport::Unknown,
            readable: false,
            writable: false,
            available_space: None,
            cloud_or_network_hint: false,
            removable_hint: false,
            path_length_limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootIdentity {
    pub root_id: String,
    pub platform: PlatformId,
    pub canonical_root: PathBuf,
    pub capabilities: FilesystemCapabilities,
}

impl RootIdentity {
    pub fn from_existing_root(
        root_id: impl Into<String>,
        platform: PlatformId,
        root: &Path,
        capabilities: FilesystemCapabilities,
    ) -> Result<Self, PathSemanticsError> {
        let root_id = root_id.into();
        if root_id.trim().is_empty() {
            return Err(PathSemanticsError::EmptyRootId);
        }

        let canonical_root = fs::canonicalize(root).map_err(|source| {
            PathSemanticsError::RootUnavailable {
                path: root.to_path_buf(),
                source,
            }
        })?;
        let metadata = fs::metadata(&canonical_root).map_err(|source| {
            PathSemanticsError::MetadataUnreadable {
                path: canonical_root.clone(),
                source,
            }
        })?;
        if !metadata.is_dir() {
            return Err(PathSemanticsError::RootIsNotDirectory(canonical_root));
        }

        Ok(Self {
            root_id,
            platform,
            canonical_root,
            capabilities,
        })
    }

    pub fn comparison_key(
        &self,
        relative_path: &Path,
    ) -> Result<PathComparisonKey, PathSemanticsError> {
        comparison_key(relative_path, self.capabilities.case_sensitivity)
    }

    pub fn prove_containment(
        &self,
        candidate: &Path,
    ) -> Result<CanonicalContainment, PathSemanticsError> {
        prove_canonical_containment(
            &self.canonical_root,
            candidate,
            self.capabilities.case_sensitivity,
        )
    }

    pub fn identify_path(
        &self,
        profile_id: impl Into<String>,
        candidate: &Path,
    ) -> Result<PortablePathIdentity, PathSemanticsError> {
        let profile_id = profile_id.into();
        if profile_id.trim().is_empty() {
            return Err(PathSemanticsError::EmptyProfileId);
        }

        let containment = self.prove_containment(candidate)?;
        let relative_path = relative_path_after_root(
            &containment.resolved_candidate,
            &containment.canonical_root,
            self.capabilities.case_sensitivity,
        )?;
        let comparison_key = self.comparison_key(&relative_path)?;

        Ok(PortablePathIdentity {
            profile_id,
            root_id: self.root_id.clone(),
            relative_path,
            comparison_key,
            display_path: candidate.to_path_buf(),
            native_identity: None,
            metadata_state: inspect_path_metadata(candidate),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComparisonComponent {
    Native(OsString),
    Folded(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathComparisonKey {
    components: Vec<ComparisonComponent>,
}

impl PathComparisonKey {
    pub fn components(&self) -> &[ComparisonComponent] {
        &self.components
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFileIdentity {
    pub namespace: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMetadataState {
    Missing,
    File,
    Directory,
    Symlink,
    Other,
    Unreadable(io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortablePathIdentity {
    pub profile_id: String,
    pub root_id: String,
    pub relative_path: PathBuf,
    pub comparison_key: PathComparisonKey,
    pub display_path: PathBuf,
    pub native_identity: Option<NativeFileIdentity>,
    pub metadata_state: PathMetadataState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalContainment {
    pub canonical_root: PathBuf,
    pub canonical_existing_ancestor: PathBuf,
    pub resolved_candidate: PathBuf,
    pub unresolved_suffix: PathBuf,
}

#[derive(Debug)]
pub enum PathSemanticsError {
    EmptyProfileId,
    EmptyRootId,
    RootUnavailable { path: PathBuf, source: io::Error },
    RootIsNotDirectory(PathBuf),
    CandidateMustBeAbsolute(PathBuf),
    RelativePathRequired(PathBuf),
    ParentTraversal(PathBuf),
    UnsupportedPathComponent(PathBuf),
    UnknownCaseSensitivity,
    NonUnicodeCaseFold(PathBuf),
    MetadataUnreadable { path: PathBuf, source: io::Error },
    ExistingAncestorNotDirectory(PathBuf),
    OutsideRoot { root: PathBuf, candidate: PathBuf },
}

impl fmt::Display for PathSemanticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProfileId => write!(formatter, "profile identity cannot be empty"),
            Self::EmptyRootId => write!(formatter, "root identity cannot be empty"),
            Self::RootUnavailable { path, source } => write!(
                formatter,
                "root '{}' could not be canonicalized: {}",
                path.display(),
                source
            ),
            Self::RootIsNotDirectory(path) => {
                write!(formatter, "root '{}' is not a directory", path.display())
            }
            Self::CandidateMustBeAbsolute(path) => write!(
                formatter,
                "candidate '{}' must be absolute before containment can be proved",
                path.display()
            ),
            Self::RelativePathRequired(path) => write!(
                formatter,
                "comparison path '{}' must be relative to a confirmed root",
                path.display()
            ),
            Self::ParentTraversal(path) => write!(
                formatter,
                "path '{}' contains a parent-directory traversal component",
                path.display()
            ),
            Self::UnsupportedPathComponent(path) => write!(
                formatter,
                "path '{}' contains an unsupported component",
                path.display()
            ),
            Self::UnknownCaseSensitivity => write!(
                formatter,
                "filesystem case sensitivity is unknown; comparison must fail closed"
            ),
            Self::NonUnicodeCaseFold(path) => write!(
                formatter,
                "path '{}' cannot be case-folded without losing native identity",
                path.display()
            ),
            Self::MetadataUnreadable { path, source } => write!(
                formatter,
                "metadata for '{}' could not be read safely: {}",
                path.display(),
                source
            ),
            Self::ExistingAncestorNotDirectory(path) => write!(
                formatter,
                "existing ancestor '{}' is not a directory",
                path.display()
            ),
            Self::OutsideRoot { root, candidate } => write!(
                formatter,
                "candidate '{}' resolves outside root '{}'",
                candidate.display(),
                root.display()
            ),
        }
    }
}

impl std::error::Error for PathSemanticsError {}

pub fn comparison_key(
    relative_path: &Path,
    case_sensitivity: CaseSensitivity,
) -> Result<PathComparisonKey, PathSemanticsError> {
    if relative_path.is_absolute() {
        return Err(PathSemanticsError::RelativePathRequired(
            relative_path.to_path_buf(),
        ));
    }

    let mut components = Vec::new();
    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(PathSemanticsError::ParentTraversal(
                    relative_path.to_path_buf(),
                ));
            }
            Component::Normal(value) => {
                let key = match case_sensitivity {
                    CaseSensitivity::Sensitive => {
                        ComparisonComponent::Native(value.to_os_string())
                    }
                    CaseSensitivity::Insensitive => {
                        let value = value.to_str().ok_or_else(|| {
                            PathSemanticsError::NonUnicodeCaseFold(
                                relative_path.to_path_buf(),
                            )
                        })?;
                        ComparisonComponent::Folded(value.to_lowercase())
                    }
                    CaseSensitivity::Unknown => {
                        return Err(PathSemanticsError::UnknownCaseSensitivity);
                    }
                };
                components.push(key);
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(PathSemanticsError::UnsupportedPathComponent(
                    relative_path.to_path_buf(),
                ));
            }
        }
    }

    Ok(PathComparisonKey { components })
}

pub fn prove_canonical_containment(
    canonical_root: &Path,
    candidate: &Path,
    case_sensitivity: CaseSensitivity,
) -> Result<CanonicalContainment, PathSemanticsError> {
    if !candidate.is_absolute() {
        return Err(PathSemanticsError::CandidateMustBeAbsolute(
            candidate.to_path_buf(),
        ));
    }
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(PathSemanticsError::ParentTraversal(
            candidate.to_path_buf(),
        ));
    }

    let canonical_root = fs::canonicalize(canonical_root).map_err(|source| {
        PathSemanticsError::RootUnavailable {
            path: canonical_root.to_path_buf(),
            source,
        }
    })?;

    let mut existing_ancestor = candidate.to_path_buf();
    let mut missing_components = Vec::<OsString>::new();

    loop {
        match fs::symlink_metadata(&existing_ancestor) {
            Ok(_) => break,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                let file_name = existing_ancestor.file_name().ok_or_else(|| {
                    PathSemanticsError::MetadataUnreadable {
                        path: existing_ancestor.clone(),
                        source,
                    }
                })?;
                missing_components.push(file_name.to_os_string());
                if !existing_ancestor.pop() {
                    return Err(PathSemanticsError::MetadataUnreadable {
                        path: candidate.to_path_buf(),
                        source: io::Error::new(
                            io::ErrorKind::NotFound,
                            "no existing ancestor was found",
                        ),
                    });
                }
            }
            Err(source) => {
                return Err(PathSemanticsError::MetadataUnreadable {
                    path: existing_ancestor,
                    source,
                });
            }
        }
    }

    let canonical_existing_ancestor =
        fs::canonicalize(&existing_ancestor).map_err(|source| {
            PathSemanticsError::MetadataUnreadable {
                path: existing_ancestor.clone(),
                source,
            }
        })?;

    if !path_starts_with_policy(
        &canonical_existing_ancestor,
        &canonical_root,
        case_sensitivity,
    )? {
        return Err(PathSemanticsError::OutsideRoot {
            root: canonical_root,
            candidate: canonical_existing_ancestor,
        });
    }

    if !missing_components.is_empty()
        && !fs::metadata(&canonical_existing_ancestor)
            .map_err(|source| PathSemanticsError::MetadataUnreadable {
                path: canonical_existing_ancestor.clone(),
                source,
            })?
            .is_dir()
    {
        return Err(PathSemanticsError::ExistingAncestorNotDirectory(
            canonical_existing_ancestor,
        ));
    }

    missing_components.reverse();
    let unresolved_suffix = missing_components.iter().collect::<PathBuf>();
    let resolved_candidate = canonical_existing_ancestor.join(&unresolved_suffix);

    Ok(CanonicalContainment {
        canonical_root,
        canonical_existing_ancestor,
        resolved_candidate,
        unresolved_suffix,
    })
}

fn path_starts_with_policy(
    candidate: &Path,
    root: &Path,
    case_sensitivity: CaseSensitivity,
) -> Result<bool, PathSemanticsError> {
    if case_sensitivity == CaseSensitivity::Unknown {
        return Err(PathSemanticsError::UnknownCaseSensitivity);
    }

    let mut candidate_components = candidate.components();
    for root_component in root.components() {
        let Some(candidate_component) = candidate_components.next() else {
            return Ok(false);
        };
        if !components_equal(
            candidate_component,
            root_component,
            case_sensitivity,
            candidate,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn components_equal(
    left: Component<'_>,
    right: Component<'_>,
    case_sensitivity: CaseSensitivity,
    context: &Path,
) -> Result<bool, PathSemanticsError> {
    if case_sensitivity == CaseSensitivity::Sensitive {
        return Ok(left == right);
    }

    match (left, right) {
        (Component::RootDir, Component::RootDir)
        | (Component::CurDir, Component::CurDir)
        | (Component::ParentDir, Component::ParentDir) => Ok(true),
        (Component::Prefix(left), Component::Prefix(right)) => {
            let left = case_fold_component(left.as_os_str(), context)?;
            let right = case_fold_component(right.as_os_str(), context)?;
            Ok(left == right)
        }
        (Component::Normal(left), Component::Normal(right)) => {
            let left = case_fold_component(left, context)?;
            let right = case_fold_component(right, context)?;
            Ok(left == right)
        }
        _ => Ok(false),
    }
}

fn case_fold_component(
    value: &std::ffi::OsStr,
    context: &Path,
) -> Result<String, PathSemanticsError> {
    value
        .to_str()
        .map(str::to_lowercase)
        .ok_or_else(|| PathSemanticsError::NonUnicodeCaseFold(context.to_path_buf()))
}

fn relative_path_after_root(
    candidate: &Path,
    root: &Path,
    case_sensitivity: CaseSensitivity,
) -> Result<PathBuf, PathSemanticsError> {
    let mut candidate_components = candidate.components();
    for root_component in root.components() {
        let Some(candidate_component) = candidate_components.next() else {
            return Err(PathSemanticsError::OutsideRoot {
                root: root.to_path_buf(),
                candidate: candidate.to_path_buf(),
            });
        };
        if !components_equal(
            candidate_component,
            root_component,
            case_sensitivity,
            candidate,
        )? {
            return Err(PathSemanticsError::OutsideRoot {
                root: root.to_path_buf(),
                candidate: candidate.to_path_buf(),
            });
        }
    }

    Ok(candidate_components.collect())
}

pub fn inspect_path_metadata(path: &Path) -> PathMetadataState {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                PathMetadataState::Symlink
            } else if file_type.is_file() {
                PathMetadataState::File
            } else if file_type.is_dir() {
                PathMetadataState::Directory
            } else {
                PathMetadataState::Other
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => PathMetadataState::Missing,
        Err(error) => PathMetadataState::Unreadable(error.kind()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn capabilities(case_sensitivity: CaseSensitivity) -> FilesystemCapabilities {
        FilesystemCapabilities {
            case_sensitivity,
            readable: true,
            writable: true,
            ..FilesystemCapabilities::default()
        }
    }

    #[test]
    fn comparison_keys_follow_root_case_policy() {
        let sensitive = RootIdentity::from_existing_root(
            "mods",
            PlatformId::Linux,
            tempdir().expect("tempdir").path(),
            capabilities(CaseSensitivity::Sensitive),
        )
        .expect("sensitive root");
        assert_ne!(
            sensitive
                .comparison_key(Path::new("Creator/Mod.package"))
                .expect("first sensitive key"),
            sensitive
                .comparison_key(Path::new("creator/mod.package"))
                .expect("second sensitive key")
        );

        let insensitive_dir = tempdir().expect("tempdir");
        let insensitive = RootIdentity::from_existing_root(
            "mods",
            PlatformId::Windows,
            insensitive_dir.path(),
            capabilities(CaseSensitivity::Insensitive),
        )
        .expect("insensitive root");
        assert_eq!(
            insensitive
                .comparison_key(Path::new("Creator/Mod.package"))
                .expect("first insensitive key"),
            insensitive
                .comparison_key(Path::new("creator/mod.package"))
                .expect("second insensitive key")
        );
    }

    #[test]
    fn portable_identity_is_profile_and_root_relative() {
        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("Mods");
        let creator = root.join("Creator");
        let file = creator.join("example.package");
        fs::create_dir_all(&creator).expect("creator directory");
        fs::write(&file, b"fixture").expect("fixture file");

        let root_identity = RootIdentity::from_existing_root(
            "mods",
            PlatformId::Macos,
            &root,
            capabilities(CaseSensitivity::Sensitive),
        )
        .expect("root identity");
        let identity = root_identity
            .identify_path("sims4-main", &file)
            .expect("portable identity");

        assert_eq!(identity.profile_id, "sims4-main");
        assert_eq!(identity.root_id, "mods");
        assert_eq!(
            identity.relative_path,
            PathBuf::from("Creator").join("example.package")
        );
        assert_eq!(identity.display_path, file);
        assert_eq!(identity.metadata_state, PathMetadataState::File);
        assert_eq!(
            identity.comparison_key,
            comparison_key(
                Path::new("Creator/example.package"),
                CaseSensitivity::Sensitive,
            )
            .expect("comparison key")
        );
        assert!(identity.native_identity.is_none());
    }

    #[test]
    fn portable_identity_rejects_empty_profile_and_classifies_missing_paths() {
        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("Mods");
        fs::create_dir(&root).expect("root");
        let root_identity = RootIdentity::from_existing_root(
            "mods",
            PlatformId::Linux,
            &root,
            capabilities(CaseSensitivity::Sensitive),
        )
        .expect("root identity");
        let missing = root.join("Creator").join("missing.package");

        assert!(matches!(
            root_identity.identify_path("", &missing),
            Err(PathSemanticsError::EmptyProfileId)
        ));
        assert_eq!(
            root_identity
                .identify_path("sims4-test", &missing)
                .expect("missing identity")
                .metadata_state,
            PathMetadataState::Missing
        );
    }

    #[test]
    fn metadata_state_distinguishes_directories_files_and_missing_paths() {
        let directory = tempdir().expect("tempdir");
        let file = directory.path().join("example.package");
        fs::write(&file, b"fixture").expect("fixture file");

        assert_eq!(
            inspect_path_metadata(directory.path()),
            PathMetadataState::Directory
        );
        assert_eq!(inspect_path_metadata(&file), PathMetadataState::File);
        assert_eq!(
            inspect_path_metadata(&directory.path().join("missing.package")),
            PathMetadataState::Missing
        );
    }

    #[test]
    fn unknown_case_policy_fails_closed() {
        assert!(matches!(
            comparison_key(Path::new("Creator/Mod.package"), CaseSensitivity::Unknown),
            Err(PathSemanticsError::UnknownCaseSensitivity)
        ));
    }

    #[test]
    fn comparison_keys_reject_absolute_and_parent_paths() {
        assert!(matches!(
            comparison_key(Path::new("../outside.package"), CaseSensitivity::Sensitive),
            Err(PathSemanticsError::ParentTraversal(_))
        ));

        let absolute = if cfg!(target_os = "windows") {
            PathBuf::from(r"C:\\Mods\\example.package")
        } else {
            PathBuf::from("/Mods/example.package")
        };
        assert!(matches!(
            comparison_key(&absolute, CaseSensitivity::Sensitive),
            Err(PathSemanticsError::RelativePathRequired(_))
        ));
    }

    #[test]
    fn containment_prefix_comparison_follows_case_policy() {
        let candidate = Path::new("/Volume/Mods/Creator/example.package");
        let root = Path::new("/volume/mods");

        assert!(path_starts_with_policy(candidate, root, CaseSensitivity::Insensitive)
            .expect("case-insensitive prefix"));
        assert!(!path_starts_with_policy(candidate, root, CaseSensitivity::Sensitive)
            .expect("case-sensitive prefix"));
        assert!(matches!(
            path_starts_with_policy(candidate, root, CaseSensitivity::Unknown),
            Err(PathSemanticsError::UnknownCaseSensitivity)
        ));
    }

    #[test]
    fn canonical_containment_accepts_missing_descendants_under_real_root() {
        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("Mods");
        fs::create_dir(&root).expect("root");
        let identity = RootIdentity::from_existing_root(
            "mods",
            PlatformId::Macos,
            &root,
            capabilities(CaseSensitivity::Sensitive),
        )
        .expect("root identity");

        let candidate = root.join("Creator").join("example.package");
        let evidence = identity
            .prove_containment(&candidate)
            .expect("contained missing path");
        assert_eq!(evidence.canonical_existing_ancestor, identity.canonical_root);
        assert_eq!(
            evidence.unresolved_suffix,
            PathBuf::from("Creator").join("example.package")
        );
        assert_eq!(
            evidence.resolved_candidate,
            identity
                .canonical_root
                .join("Creator")
                .join("example.package")
        );
    }

    #[test]
    fn canonical_containment_rejects_textual_prefixes_and_parent_traversal() {
        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("Mods");
        let sibling = directory.path().join("ModsBackup");
        fs::create_dir(&root).expect("root");
        fs::create_dir(&sibling).expect("sibling");
        let canonical_root = fs::canonicalize(&root).expect("canonical root");

        assert!(matches!(
            prove_canonical_containment(
                &canonical_root,
                &sibling.join("example.package"),
                CaseSensitivity::Sensitive,
            ),
            Err(PathSemanticsError::OutsideRoot { .. })
        ));
        assert!(matches!(
            prove_canonical_containment(
                &canonical_root,
                &root.join("..").join("escape.package"),
                CaseSensitivity::Sensitive,
            ),
            Err(PathSemanticsError::ParentTraversal(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_containment_rejects_symlink_escape_and_allows_internal_symlink() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().expect("tempdir");
        let root = directory.path().join("Mods");
        let inside = root.join("Inside");
        let outside = directory.path().join("Outside");
        fs::create_dir(&root).expect("root");
        fs::create_dir(&inside).expect("inside");
        fs::create_dir(&outside).expect("outside");
        symlink(&outside, root.join("OutsideLink")).expect("outside symlink");
        symlink(&inside, root.join("InsideLink")).expect("inside symlink");

        let canonical_root = fs::canonicalize(&root).expect("canonical root");
        assert!(matches!(
            prove_canonical_containment(
                &canonical_root,
                &root.join("OutsideLink").join("example.package"),
                CaseSensitivity::Sensitive,
            ),
            Err(PathSemanticsError::OutsideRoot { .. })
        ));

        let evidence = prove_canonical_containment(
            &canonical_root,
            &root.join("InsideLink").join("example.package"),
            CaseSensitivity::Sensitive,
        )
        .expect("internal symlink");
        assert!(evidence.resolved_candidate.starts_with(&canonical_root));
        assert_eq!(
            evidence.resolved_candidate,
            fs::canonicalize(&inside)
                .expect("canonical inside")
                .join("example.package")
        );
    }
}
