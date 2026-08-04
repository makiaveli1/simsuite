use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    pub mods_path: Option<String>,
    pub tray_path: Option<String>,
    pub downloads_path: Option<String>,
    #[serde(default)]
    pub download_reject_folder: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameOperatingEnvironment {
    NativeWindows,
    NativeMacos,
    NativeLinux,
    Wine,
    Proton,
    Lutris,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationProfileStatus {
    Draft,
    Valid,
    NeedsReview,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationDetectionMethod {
    Manual,
    LegacySettingsMigration,
    PlatformCandidate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationConfirmationState {
    Unconfirmed,
    Confirmed,
    ConfirmationStale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationRootValidationState {
    Unvalidated,
    Valid,
    NeedsReview,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationRoot {
    pub profile_id: String,
    pub root_id: String,
    pub root_role: String,
    pub configured_path: String,
    pub required: bool,
    pub validation_state: GameInstallationRootValidationState,
    pub filesystem_capabilities_json: String,
    pub last_validated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationProfile {
    pub profile_id: String,
    pub profile_name: String,
    pub game_id: String,
    pub operating_environment: GameOperatingEnvironment,
    pub status: GameInstallationProfileStatus,
    pub detection_method: GameInstallationDetectionMethod,
    pub detection_evidence_json: String,
    pub confirmation_state: GameInstallationConfirmationState,
    pub confirmed_at: Option<String>,
    pub last_validated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub roots: Vec<GameInstallationRoot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualGameInstallationProfileRequest {
    pub profile_name: String,
    pub user_data_path: Option<String>,
    pub mods_path: String,
    pub tray_path: String,
    pub downloads_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationProfileConfirmationResult {
    pub profile: GameInstallationProfile,
    pub validation: GameInstallationProfileValidationReport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationCandidateConfidence {
    Strong,
    Possible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationCandidateRoot {
    pub root_id: String,
    pub root_role: String,
    pub configured_path: String,
    pub required: bool,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationCandidate {
    pub candidate_id: String,
    pub rank: usize,
    pub game_id: String,
    pub operating_environment: GameOperatingEnvironment,
    pub suggested_name: String,
    pub suggested_roots: Vec<GameInstallationCandidateRoot>,
    pub confidence: GameInstallationCandidateConfidence,
    pub detection_evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationCandidateDetectionResult {
    pub current_environment: GameOperatingEnvironment,
    pub supported: bool,
    pub candidates: Vec<GameInstallationCandidate>,
    pub read_only: bool,
    pub review_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationEnvironmentCompatibility {
    Matches,
    Mismatch,
    RequiresAdapter,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationPathMetadataState {
    NotChecked,
    Missing,
    File,
    Directory,
    Symlink,
    Other,
    Unreadable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationCaseSensitivity {
    NotChecked,
    Sensitive,
    Insensitive,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationRootValidationReport {
    pub root_id: String,
    pub root_role: String,
    pub configured_path: String,
    pub required: bool,
    pub state: GameInstallationRootValidationState,
    pub absolute_path: Option<bool>,
    pub exists: Option<bool>,
    pub metadata_readable: Option<bool>,
    pub metadata_state: GameInstallationPathMetadataState,
    pub canonical_path_display: Option<String>,
    pub case_sensitivity: GameInstallationCaseSensitivity,
    pub symlink_observed: Option<bool>,
    pub blockers: Vec<String>,
    pub review_notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInstallationEvidenceStrength {
    Confirmed,
    StronglySupported,
    Possible,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationReadinessEvidence {
    pub code: String,
    pub strength: GameInstallationEvidenceStrength,
    pub summary: String,
    pub root_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationAdapterReadinessReport {
    pub adapter_id: String,
    pub complete: bool,
    pub state: GameInstallationProfileStatus,
    pub supported_mod_extensions: Vec<String>,
    pub supported_tray_extensions: Vec<String>,
    pub max_script_folder_depth: usize,
    pub max_package_folder_depth: usize,
    pub evidence: Vec<GameInstallationReadinessEvidence>,
    pub blockers: Vec<String>,
    pub review_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameInstallationProfileValidationReport {
    pub profile_id: String,
    pub profile_name: String,
    pub game_id: String,
    pub stored_environment: GameOperatingEnvironment,
    pub current_environment: GameOperatingEnvironment,
    pub environment_compatibility: GameInstallationEnvironmentCompatibility,
    pub state: GameInstallationProfileStatus,
    pub generic_root_state: GameInstallationProfileStatus,
    pub game_specific_validation_pending: bool,
    pub game_readiness: Option<GameInstallationAdapterReadinessReport>,
    pub read_only: bool,
    pub roots: Vec<GameInstallationRootValidationReport>,
    pub blockers: Vec<String>,
    pub review_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBehaviorSettings {
    pub keep_running_in_background: bool,
    pub automatic_watch_checks: bool,
    pub watch_check_interval_hours: i64,
    pub last_watch_check_at: Option<String>,
    pub last_watch_check_error: Option<String>,
    pub download_ignore_patterns: Vec<String>,
    /// If `Some(true)`, special mod (MCCC) version updates are excluded from the
    /// update counts shown in the UI and tray tooltip. If `None` or `Some(false)`,
    /// they are shown normally.
    #[serde(default)]
    pub silent_special_mod_updates: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRefreshSummary {
    pub checked_subjects: i64,
    pub exact_update_items: i64,
    pub possible_update_items: i64,
    pub check_failed_watch_items: i64,
    pub unknown_watch_items: i64,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLibraryPaths {
    pub mods_path: Option<String>,
    pub tray_path: Option<String>,
    pub downloads_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeOverview {
    pub total_files: i64,
    pub mods_count: i64,
    pub tray_count: i64,
    pub downloads_count: i64,
    pub script_mods_count: i64,
    pub creator_count: i64,
    pub bundles_count: i64,
    pub duplicates_count: i64,
    pub review_count: i64,
    pub unsafe_count: i64,
    pub exact_update_items: i64,
    pub possible_update_items: i64,
    pub check_failed_watch_items: i64,
    pub unknown_watch_items: i64,
    pub watch_review_items: i64,
    pub watch_setup_items: i64,
    pub last_scan_at: Option<String>,
    pub scan_needs_refresh: bool,
    pub read_only_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadsWatcherState {
    Idle,
    Watching,
    Processing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub phase: String,
    pub current_file: String,
    pub processed_count: usize,
    pub total_count: usize,
    pub bytes_processed: u64,
    pub bytes_total: u64,
}

/// A single timestamped staging subdirectory for one download item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingSubDirectory {
    /// Full path to the subdirectory.
    pub path: String,
    /// Human-readable name (the directory name itself).
    pub name: String,
    /// Number of files directly inside this subdirectory.
    pub file_count: usize,
    /// Total size in bytes of all files in this subdirectory (recursive).
    pub total_bytes: u64,
    /// When this subdirectory was created (extracted), as RFC3339.
    pub created_at: Option<String>,
}

/// All staging subdirectories for one download item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingArea {
    /// The item ID, or "new" for uncommitted items.
    pub item_id: String,
    /// Subdirectories belonging to this item.
    pub subdirectories: Vec<StagingSubDirectory>,
}

/// Summary of all staging areas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingAreasSummary {
    pub areas: Vec<StagingArea>,
    pub total_bytes: u64,
    pub total_file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanSource {
    Staging,
    Library,
    Inbox,
    Organize,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanStatus {
    PreviewOnly,
    Blocked,
    ReadyForReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanActionKind {
    SuggestMove,
    SuggestGroup,
    SuggestReview,
    LeaveInPlace,
    NoAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanEvidenceLevel {
    Deterministic,
    EvidenceBacked,
    Heuristic,
    ReviewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanBucket {
    ScriptMods,
    Cas,
    BuildBuy,
    Gameplay,
    PresetsSliders,
    OverridesDefaults,
    Tray,
    NeedsReview,
    UnknownLeaveInPlace,
}

impl Default for StagingPlanBucket {
    fn default() -> Self {
        Self::NeedsReview
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StagingPlanConfidenceLabel {
    Deterministic,
    EvidenceBacked,
    Heuristic,
    ReviewOnly,
}

impl Default for StagingPlanConfidenceLabel {
    fn default() -> Self {
        Self::ReviewOnly
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagingPlanCurrentRoot {
    Mods,
    Tray,
    Downloads,
    Inbox,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanFolderConfigMode {
    Default,
    Custom,
}

impl Default for ApplyPlanFolderConfigMode {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanCreatorFolderMode {
    Off,
    WhenAvailable,
}

impl Default for ApplyPlanCreatorFolderMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanCategoryFolderMode {
    BucketOnly,
    BucketAndCategory,
}

impl Default for ApplyPlanCategoryFolderMode {
    fn default() -> Self {
        Self::BucketOnly
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanFolderConfig {
    #[serde(default)]
    pub mode: ApplyPlanFolderConfigMode,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub bucket_folders: BTreeMap<String, String>,
    #[serde(default)]
    pub creator_folder_mode: ApplyPlanCreatorFolderMode,
    #[serde(default)]
    pub category_folder_mode: ApplyPlanCategoryFolderMode,
    #[serde(default)]
    pub max_depth: Option<i64>,
    #[serde(default)]
    pub exclusion_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanContextSignalStrength {
    Blocking,
    Evidence,
    Routing,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanContextSignal {
    pub source_system: String,
    pub signal_kind: String,
    pub label: String,
    pub value: Option<String>,
    pub strength: ApplyPlanContextSignalStrength,
}

impl Default for StagingPlanCurrentRoot {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingPlanItem {
    pub id: String,
    pub file_id: Option<i64>,
    pub file_name: String,
    pub current_path: Option<String>,
    pub suggested_destination_path: Option<String>,
    pub action_kind: StagingPlanActionKind,
    pub evidence_level: StagingPlanEvidenceLevel,
    pub reason: String,
    pub caveats: Vec<String>,
    #[serde(default)]
    pub source_signals: Vec<String>,
    #[serde(default)]
    pub blocked_reasons: Vec<String>,
    #[serde(default)]
    pub bucket: StagingPlanBucket,
    #[serde(default)]
    pub confidence_label: StagingPlanConfidenceLabel,
    #[serde(default)]
    pub current_root: StagingPlanCurrentRoot,
    pub would_touch_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingPlan {
    pub id: String,
    pub created_at: String,
    pub source: StagingPlanSource,
    pub status: StagingPlanStatus,
    pub title: String,
    pub summary: String,
    pub item_count: usize,
    pub would_touch_files: bool,
    pub caveats: Vec<String>,
    pub items: Vec<StagingPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersistedApplyPlanStatus {
    Draft,
    PreviewOnlySource,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersistedApplyPlanItemStatus {
    PreviewOnly,
    Blocked,
    ReviewOnly,
    DraftCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanSignal {
    pub id: i64,
    pub apply_plan_item_id: i64,
    pub signal_kind: String,
    pub signal_label: String,
    pub signal_value: Option<String>,
    pub evidence_level: Option<String>,
    pub source_system: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanBlocker {
    pub id: i64,
    pub apply_plan_item_id: i64,
    pub blocker_kind: String,
    pub reason_code: String,
    pub message: String,
    pub source_system: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanItem {
    pub id: i64,
    pub apply_plan_id: i64,
    pub source_item_id: Option<String>,
    pub file_id: Option<i64>,
    pub file_name: String,
    pub current_path: String,
    pub current_root: String,
    pub destination_path: Option<String>,
    pub destination_root: Option<String>,
    pub action_kind: String,
    pub evidence_level: String,
    pub bucket: Option<String>,
    pub confidence_label: Option<String>,
    pub item_status: PersistedApplyPlanItemStatus,
    pub blocked: bool,
    pub review_only: bool,
    pub validation_status: Option<String>,
    pub conflict_status: Option<String>,
    pub path_privacy_level: String,
    pub created_at: String,
    pub updated_at: String,
    pub signals: Vec<PersistedApplyPlanSignal>,
    pub blockers: Vec<PersistedApplyPlanBlocker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlan {
    pub id: i64,
    pub source_staging_plan_id: Option<String>,
    pub source_plan_kind: String,
    pub title: String,
    pub summary: String,
    pub status: PersistedApplyPlanStatus,
    pub would_touch_files: bool,
    pub confirmation_required: bool,
    pub backup_required: bool,
    pub restore_available: bool,
    pub total_items: i64,
    pub applyable_items: i64,
    pub blocked_items: i64,
    pub review_only_items: i64,
    pub caveats: Vec<String>,
    pub source_scope: Option<serde_json::Value>,
    pub folder_config: Option<ApplyPlanFolderConfig>,
    pub context_trail: Vec<ApplyPlanContextSignal>,
    pub plan_hash: Option<String>,
    pub plan_hash_version: String,
    pub plan_hash_algorithm: String,
    pub plan_hash_created_at: Option<String>,
    pub preview_snapshot_id: Option<i64>,
    pub preview_snapshot_hash: Option<String>,
    #[serde(skip)]
    pub plan_provenance: serde_json::Value,
    pub scan_session_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<PersistedApplyPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanListItem {
    pub id: i64,
    pub source_staging_plan_id: Option<String>,
    pub source_plan_kind: String,
    pub title: String,
    pub summary: String,
    pub status: PersistedApplyPlanStatus,
    pub would_touch_files: bool,
    pub confirmation_required: bool,
    pub backup_required: bool,
    pub restore_available: bool,
    pub total_items: i64,
    pub applyable_items: i64,
    pub blocked_items: i64,
    pub review_only_items: i64,
    pub plan_hash: Option<String>,
    pub plan_hash_version: String,
    pub plan_hash_algorithm: String,
    pub plan_hash_created_at: Option<String>,
    pub preview_snapshot_id: Option<i64>,
    pub preview_snapshot_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveApplyPlanPreviewRequest {
    pub source_plan: StagingPlan,
    #[serde(default)]
    pub source_plan_kind: Option<String>,
    #[serde(default)]
    pub source_scope: Option<serde_json::Value>,
    #[serde(default)]
    pub folder_config: Option<ApplyPlanFolderConfig>,
    #[serde(default)]
    pub context_trail: Vec<ApplyPlanContextSignal>,
    #[serde(default)]
    pub scan_session_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSortingPreviewPlanResult {
    pub plan: StagingPlan,
    pub preview_snapshot_id: i64,
    pub preview_snapshot_hash: String,
    pub preview_snapshot_hash_version: String,
    pub preview_snapshot_hash_algorithm: String,
    pub preview_snapshot_created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveApplyPlanFromPreviewSnapshotRequest {
    pub preview_snapshot_id: i64,
    pub preview_snapshot_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveApplyPlanPreviewResult {
    pub plan_id: i64,
    pub plan: ApplyPlanListItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildApplyPlanFromStagingPlanRequest {
    pub preview_request: GenerateSortingPreviewPlanRequest,
    #[serde(default)]
    pub source_plan_kind: Option<String>,
    #[serde(default)]
    pub folder_config: Option<ApplyPlanFolderConfig>,
    #[serde(default)]
    pub context_trail: Vec<ApplyPlanContextSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListSavedApplyPlansRequest {
    pub include_cancelled: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDraftApplyPlanResult {
    pub plan_id: i64,
    pub cancelled: bool,
    pub status: PersistedApplyPlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewApplyPlanValidationRequest {
    pub plan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanValidationPreviewStatus {
    NotValidated,
    ValidPreviewOnly,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanValidationStatus {
    NotValidated,
    ValidPreviewOnly,
    Blocked,
    StaleSource,
    MissingSource,
    MissingDestinationRoot,
    UnsafeDestination,
    DestinationExists,
    UnsupportedCrossRoot,
    ReviewOnlyBlocked,
    DuplicateReviewBlocked,
    BackupRequired,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanConflictStatus {
    NotChecked,
    None,
    DestinationExists,
    SameNameConflict,
    CaseConflict,
    FolderMissing,
    PermissionUnknown,
    SourceMissing,
    PathTooLong,
    CrossRootBlocked,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanValidationSummary {
    pub total_items: i64,
    pub blocked_items: i64,
    pub review_only_items: i64,
    pub conflict_items: i64,
    pub stale_items: i64,
    pub missing_source_items: i64,
    pub destination_conflict_items: i64,
    pub backup_blocked_items: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanValidationItem {
    pub item_id: i64,
    pub file_id: Option<i64>,
    pub file_name: String,
    pub validation_status: ApplyPlanValidationStatus,
    pub conflict_status: ApplyPlanConflictStatus,
    pub blocked: bool,
    pub review_only: bool,
    pub can_apply_later: bool,
    pub reasons: Vec<String>,
    pub required_next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanValidationPreview {
    pub plan_id: i64,
    pub plan_hash: Option<String>,
    pub source_plan_kind: String,
    pub status: ApplyPlanValidationPreviewStatus,
    pub can_proceed_to_confirmation: bool,
    pub checked_at: String,
    pub summary: ApplyPlanValidationSummary,
    pub caveats: Vec<String>,
    pub items: Vec<ApplyPlanValidationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewApplyPlanDryRunRequest {
    pub plan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanDryRunPreviewStatus {
    NotRun,
    Blocked,
    PreviewOnly,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanDryRunItemStatus {
    Blocked,
    WouldSkip,
    WouldRequireReview,
    WouldRequireBackup,
    WouldRequireConfirmation,
    WouldRequireDestinationReview,
    CandidateAfterFutureSafetyGates,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanDryRunActionPreview {
    WouldMoveLater,
    WouldSkip,
    NoAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanDryRunSummary {
    pub total_items: i64,
    pub candidate_items: i64,
    pub skipped_items: i64,
    pub blocked_items: i64,
    pub review_only_items: i64,
    pub conflict_items: i64,
    pub backup_required_items: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanDryRunItem {
    pub item_id: i64,
    pub file_id: Option<i64>,
    pub file_name: String,
    pub dry_run_status: ApplyPlanDryRunItemStatus,
    pub action_preview: ApplyPlanDryRunActionPreview,
    pub source_path: Option<String>,
    pub destination_path: Option<String>,
    pub reasons: Vec<String>,
    pub blockers: Vec<String>,
    pub required_before_apply: Vec<String>,
    pub can_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanDryRunPreview {
    pub plan_id: i64,
    pub status: ApplyPlanDryRunPreviewStatus,
    pub can_proceed_to_apply: bool,
    pub can_proceed_to_confirmation: bool,
    pub checked_at: String,
    pub summary: ApplyPlanDryRunSummary,
    pub caveats: Vec<String>,
    pub items: Vec<ApplyPlanDryRunItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanRunLogStatus {
    DraftLog,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanResultLogStatus {
    PendingLog,
    Skipped,
    Blocked,
    FailedBeforeChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPlanRestoreEntryStatus {
    NotAvailable,
    DesignOnly,
    NotRestored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanRun {
    pub id: i64,
    pub apply_plan_id: i64,
    pub status: ApplyPlanRunLogStatus,
    pub backup_strategy: String,
    #[serde(skip)]
    pub confirmation_token: Option<String>,
    pub confirmed_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub total_items: i64,
    pub skipped_items: i64,
    pub applied_items: i64,
    pub failed_items: i64,
    pub restored_items: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanResult {
    pub id: i64,
    pub apply_plan_run_id: i64,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: Option<i64>,
    pub operation_kind: String,
    pub result_status: ApplyPlanResultLogStatus,
    pub source_path_at_execution: Option<String>,
    pub destination_path_at_execution: Option<String>,
    pub backup_path: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub user_summary: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedApplyPlanRestoreEntry {
    pub id: i64,
    pub apply_plan_run_id: i64,
    pub apply_plan_result_id: Option<i64>,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: Option<i64>,
    pub original_source_path: String,
    pub destination_path_at_execution: Option<String>,
    pub backup_path: Option<String>,
    pub file_hash_before: Option<String>,
    pub file_size_before: Option<i64>,
    pub operation_kind: String,
    pub operation_result_status: ApplyPlanResultLogStatus,
    pub restore_status: ApplyPlanRestoreEntryStatus,
    pub restore_error_code: Option<String>,
    pub restore_error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub restored_at: Option<String>,
    pub failed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApplyPlanRunLogRequest {
    pub apply_plan_id: i64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub backup_strategy: Option<String>,
    #[serde(default)]
    pub confirmation_token: Option<String>,
    #[serde(default)]
    pub total_items: Option<i64>,
    #[serde(default)]
    pub skipped_items: Option<i64>,
    #[serde(default)]
    pub failed_items: Option<i64>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListApplyPlanRunLogsRequest {
    #[serde(default)]
    pub apply_plan_id: Option<i64>,
    #[serde(default)]
    pub include_cancelled: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanRunLogDetail {
    pub run: PersistedApplyPlanRun,
    pub results: Vec<PersistedApplyPlanResult>,
    pub restore_entries: Vec<PersistedApplyPlanRestoreEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordApplyPlanResultLogRequest {
    pub apply_plan_run_id: i64,
    #[serde(default)]
    pub apply_plan_item_id: Option<i64>,
    pub operation_kind: String,
    pub result_status: String,
    #[serde(default)]
    pub source_path_at_execution: Option<String>,
    #[serde(default)]
    pub destination_path_at_execution: Option<String>,
    #[serde(default)]
    pub backup_path: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    pub user_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListApplyPlanResultLogsRequest {
    pub apply_plan_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordApplyPlanRestoreEntryRequest {
    pub apply_plan_run_id: i64,
    #[serde(default)]
    pub apply_plan_result_id: Option<i64>,
    #[serde(default)]
    pub apply_plan_item_id: Option<i64>,
    pub original_source_path: String,
    #[serde(default)]
    pub destination_path_at_execution: Option<String>,
    #[serde(default)]
    pub backup_path: Option<String>,
    #[serde(default)]
    pub file_hash_before: Option<String>,
    #[serde(default)]
    pub file_size_before: Option<i64>,
    pub operation_kind: String,
    pub operation_result_status: String,
    pub restore_status: String,
    #[serde(default)]
    pub restore_error_code: Option<String>,
    #[serde(default)]
    pub restore_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListApplyPlanRestoreEntriesRequest {
    pub apply_plan_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateSortingPreviewPlanRequest {
    pub scope: GenerateSortingPreviewPlanScope,
    #[serde(default)]
    pub folder_config: Option<ApplyPlanFolderConfig>,
    #[serde(default)]
    pub context_trail: Vec<ApplyPlanContextSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GenerateSortingPreviewPlanScope {
    SelectedFiles {
        #[serde(default, rename = "fileIds")]
        file_ids: Vec<i64>,
    },
    LibraryFolder {
        #[serde(rename = "sourceLocation")]
        source_location: String,
        #[serde(rename = "folderPath")]
        folder_path: String,
        #[serde(default)]
        recursive: bool,
        limit: Option<i64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsWatcherStatus {
    pub state: DownloadsWatcherState,
    pub watched_path: Option<String>,
    pub configured: bool,
    pub current_item: Option<String>,
    pub last_run_at: Option<String>,
    pub last_change_at: Option<String>,
    pub last_error: Option<String>,
    pub ready_items: i64,
    pub needs_review_items: i64,
    pub active_items: i64,
    /// Timestamp (RFC3339) of the last full downloads folder scan start.
    /// Used by the fast-path to skip files not modified since the last scan.
    #[serde(default)]
    pub last_downloads_scan_at: Option<String>,
    /// Live progress of long-running operations (ZIP extraction, scanning, etc.)
    #[serde(default)]
    pub progress: Option<DownloadProgress>,
}

impl Default for DownloadsWatcherStatus {
    fn default() -> Self {
        Self {
            state: DownloadsWatcherState::Idle,
            watched_path: None,
            configured: false,
            current_item: None,
            last_run_at: None,
            last_change_at: None,
            last_error: None,
            ready_items: 0,
            needs_review_items: 0,
            active_items: 0,
            last_downloads_scan_at: None,
            progress: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScanMode {
    Full,
    Incremental,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanPhase {
    Collecting,
    Hashing,
    Classifying,
    Bundling,
    Duplicates,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub current_item: String,
    pub phase: ScanPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub session_id: i64,
    pub scan_mode: ScanMode,
    pub files_scanned: usize,
    pub reused_files: usize,
    pub new_files: usize,
    pub updated_files: usize,
    pub removed_files: usize,
    pub hashed_files: usize,
    pub review_items_created: usize,
    pub bundles_detected: usize,
    pub duplicates_detected: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScanRuntimeState {
    Idle,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub state: ScanRuntimeState,
    pub mode: Option<ScanMode>,
    pub phase: Option<ScanPhase>,
    pub total_files: usize,
    pub processed_files: usize,
    pub current_item: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub last_summary: Option<ScanSummary>,
    pub error: Option<String>,
}

impl Default for ScanStatus {
    fn default() -> Self {
        Self {
            state: ScanRuntimeState::Idle,
            mode: None,
            phase: None,
            total_files: 0,
            processed_files: 0,
            current_item: None,
            started_at: None,
            finished_at: None,
            last_summary: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceDomain {
    Home,
    Downloads,
    Library,
    Organize,
    Review,
    Duplicates,
    CreatorAudit,
    CategoryAudit,
    Snapshots,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceChange {
    pub domains: Vec<WorkspaceDomain>,
    pub reason: String,
    pub item_ids: Vec<i64>,
    pub family_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    pub search: Option<String>,
    pub kind: Option<String>,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub source: Option<String>,
    pub min_confidence: Option<f64>,
    pub watch_filter: Option<LibraryWatchFilter>,
    pub sort_by: Option<LibrarySortField>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    #[serde(default)]
    pub include_previews: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryFolderFilesQuery {
    pub folder_path: String,
    pub recursive: bool,
    pub filters: LibraryQuery,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_previews: Option<bool>,
}

/// Quick-filter for watch/state-based Library filtering.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LibraryWatchFilter {
    /// Show all files (no filter).
    #[default]
    All,
    /// Files with an update available or possible update.
    HasUpdates,
    /// Files with safety notes or other attention signals.
    NeedsAttention,
    /// Files that have no watch source set.
    NotTracked,
    /// Files that appear in any duplicate pair (exact, filename, or version).
    Duplicates,
}

/// Sort field for Library file listing.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySortField {
    /// Sort by filename alphabetically.
    #[default]
    Name,
    /// Sort by creator name alphabetically.
    Creator,
    /// Sort by last modified date, newest first.
    RecentlyModified,
    /// Sort by update priority: updates first, then not tracked.
    /// Tie-break by filename.
    HasUpdatesFirst,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionSignal {
    pub raw_value: String,
    pub normalized_value: String,
    pub source_kind: String,
    pub source_path: Option<String>,
    pub matched_by: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionCompareStatus {
    NotInstalled,
    IncomingNewer,
    SameVersion,
    IncomingOlder,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionConfidence {
    Exact,
    Strong,
    Medium,
    Weak,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VersionResolution {
    pub subject_label: Option<String>,
    pub matched_subject_label: Option<String>,
    pub matched_subject_key: Option<String>,
    pub status: VersionCompareStatus,
    pub confidence: VersionConfidence,
    pub match_score: f64,
    pub incoming_version: Option<String>,
    pub installed_version: Option<String>,
    pub incoming_signature: Option<String>,
    pub installed_signature: Option<String>,
    pub evidence: Vec<String>,
    pub incoming_evidence: Vec<String>,
    pub installed_evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstalledVersionSummary {
    pub subject_label: String,
    pub subject_key: String,
    pub version: Option<String>,
    pub signature: Option<String>,
    pub confidence: VersionConfidence,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchSourceKind {
    ExactPage,
    CreatorPage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchCapability {
    CanRefreshNow,
    #[default]
    SavedReferenceOnly,
    ProviderRequired,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchStatus {
    #[default]
    NotWatched,
    Current,
    ExactUpdateAvailable,
    PossibleUpdate,
    CheckFailed,
    ReminderOnly,
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchSourceOrigin {
    #[default]
    None,
    SavedByUser,
    BuiltInSpecial,
}

/// Which update platform the watch source is from.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    #[default]
    Unknown,
    Curseforge,
    Modrinth,
    Patreon,
    Website,
}

/// Detects update source from a URL string.
pub fn detect_update_source(url: &str) -> (UpdateSource, Option<String>) {
    if url.contains("curseforge.com") {
        (UpdateSource::Curseforge, None)
    } else if url.contains("modrinth.com") {
        (UpdateSource::Modrinth, None)
    } else if url.contains("patreon.com") {
        (UpdateSource::Patreon, Some(url.to_string()))
    } else {
        (UpdateSource::Website, None)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchResult {
    pub status: WatchStatus,
    pub source_kind: Option<WatchSourceKind>,
    pub source_origin: WatchSourceOrigin,
    pub source_label: Option<String>,
    pub source_url: Option<String>,
    pub source: Option<UpdateSource>,
    pub patreon_url: Option<String>,
    pub capability: WatchCapability,
    pub can_refresh_now: bool,
    pub provider_name: Option<String>,
    pub latest_version: Option<String>,
    pub checked_at: Option<String>,
    pub confidence: VersionConfidence,
    pub note: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileInsights {
    /// Raw PNG bytes of the CAS custom thumbnail, encoded as base64.
    /// Present only when a THUM (0x3C1AF1F2) resource was found and successfully decoded.
    /// Empty for non-CAS files, script mods, and CAS files without a custom thumbnail.
    pub thumbnail_preview: Option<String>,
    // MODMANAGER REMOVED — first-party only (Phase 5m)
    // pub modmanager_thumbnail_preview: Option<String>, // removed
    /// Base64-encoded DDS thumbnail extracted from localthumbcache.package.
    /// Derived from the Windows thumbnail cache, not embedded in the package file itself.
    /// Labeled as "cached" in the UI to distinguish from embedded/file-derived thumbnails.
    pub cached_thumbnail_preview: Option<String>,
    pub format: Option<String>,
    pub resource_summary: Vec<String>,
    pub script_namespaces: Vec<String>,
    pub embedded_names: Vec<String>,
    pub creator_hints: Vec<String>,
    pub version_hints: Vec<String>,
    pub version_signals: Vec<VersionSignal>,
    pub family_hints: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPreviewDiagnostics {
    pub total_rows: i64,
    pub rows_with_preview: i64,
    pub rows_without_preview: i64,
    pub embedded_preview_rows: i64,
    pub cached_preview_rows: i64,
    pub package_rows: i64,
    pub package_rows_with_preview: i64,
    pub package_rows_deferred_or_missing: i64,
    pub script_rows: i64,
    pub tray_rows: i64,
    pub other_unsupported_rows: i64,
    pub unsupported_rows: i64,
    pub unsupported_without_preview: i64,
    pub failed_extraction_rows: i64,
    pub stale_preview_rows: i64,
    pub failure_state_tracked: bool,
    pub stale_state_tracked: bool,
    pub deferred_extraction_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreatorLearningInfo {
    pub locked_by_user: bool,
    pub preferred_path: Option<String>,
    pub learned_aliases: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryOverrideInfo {
    pub saved_by_user: bool,
    pub kind: Option<String>,
    pub subtype: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemSignalSeverity {
    Info,
    Caution,
    Warning,
    Severe,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemSignalProofLevel {
    Confirmed,
    Detected,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemSignalDestination {
    Review,
    Duplicates,
    Updates,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemSignal {
    pub signal_type: String,
    pub severity: ProblemSignalSeverity,
    pub proof_level: ProblemSignalProofLevel,
    pub short_label: String,
    pub explanation: String,
    pub source: String,
    pub evidence: Vec<String>,
    pub destination: Option<ProblemSignalDestination>,
    pub show_in_library: bool,
    pub show_in_inspector: bool,
    pub show_in_more_details: bool,
    pub show_in_needs_review: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFileRow {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub extension: String,
    pub kind: String,
    pub subtype: Option<String>,
    pub confidence: f64,
    pub source_location: String,
    pub size: i64,
    pub modified_at: Option<String>,
    pub creator: Option<String>,
    pub bundle_name: Option<String>,
    pub bundle_type: Option<String>,
    pub grouped_file_count: Option<i64>,
    pub relative_depth: i64,
    pub safety_notes: Vec<String>,
    /// Parser warnings for this file (e.g. missing script, unusual structure).
    #[serde(default)]
    pub parser_warnings: Vec<String>,
    #[serde(default)]
    pub insights: FileInsights,
    /// Pre-computed watch status for list display. Derived from content_watch_results.
    #[serde(default)]
    pub watch_status: WatchStatus,
    /// True if this file appears in an exact deterministic duplicate pair.
    #[serde(default)]
    pub has_duplicate: bool,
    /// Installed version string for list display — populated from content_versions
    /// resolution for the detail panel; list query leaves this None.
    #[serde(skip)]
    pub installed_version: Option<String>,
    /// How many other files share the same indexed parent folder in the
    /// current filtered result set.
    #[serde(default)]
    pub same_folder_peer_count: i64,
    /// How many other files are in the same bundle (same bundle_id) in the
    /// current filtered result set.
    #[serde(default)]
    pub same_pack_peer_count: i64,
    #[serde(default)]
    pub primary_problem_signal: Option<ProblemSignal>,
}

/// Lightweight folder tree metadata — folder structure only, NO file rows.
/// The backend computes this from a single SQL aggregation query, making tree
/// rendering instant on the frontend (no need to transfer 13k+ full file records).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeMetadata {
    /// Total unique folder paths in the filtered dataset.
    pub total_folders: i64,
    /// The root folder nodes (Mods, Tray). Children are nested inline.
    pub roots: Vec<FolderTreeNode>,
}

/// A single folder node. Does NOT represent a file — only a folder/directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderTreeNode {
    /// Full normalized path from the root (e.g. "Mods/SubFolder/Nested").
    pub path: String,
    /// Just the folder name (e.g. "SubFolder").
    pub name: String,
    /// Depth from root: 0 = Mods/Tray root, 1 = first-level subfolder, etc.
    pub depth: i64,
    /// "mods" or "tray".
    pub source_location: String,
    /// Real disk folder path when SimSuite has scanner-owned folder metadata.
    pub disk_path: Option<String>,
    /// Number of files directly inside this folder.
    pub direct_file_count: i64,
    /// Number of immediate child subfolders.
    pub child_folder_count: i64,
    /// Total files in this folder AND all descendant folders.
    pub total_file_count: i64,
    /// Nested child folder nodes. Empty if no subfolders.
    #[serde(default)]
    pub children: Vec<FolderTreeNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryListResponse {
    pub total: i64,
    pub items: Vec<LibraryFileRow>,
}

/// Summary counts for the Library strip. All counts are filtered to
/// source_location <> 'downloads' (i.e. installed content only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub total: i64,
    pub tracked: i64,
    pub not_tracked: i64,
    pub has_updates: i64,
    pub needs_review: i64,
    pub duplicates: i64,
    pub disabled: i64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchListFilter {
    #[default]
    Attention,
    ExactUpdates,
    PossibleUpdates,
    Unclear,
    All,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchListItem {
    pub file_id: i64,
    pub filename: String,
    pub creator: Option<String>,
    pub subject_label: String,
    pub installed_version: Option<String>,
    pub watch_result: WatchResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchListResponse {
    pub filter: WatchListFilter,
    pub total: i64,
    pub items: Vec<LibraryWatchListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchSetupItem {
    pub file_id: i64,
    pub filename: String,
    pub creator: Option<String>,
    pub subject_label: String,
    pub installed_version: Option<String>,
    pub suggested_source_kind: WatchSourceKind,
    pub setup_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchSetupResponse {
    pub total: i64,
    pub truncated: bool,
    pub exact_page_total: i64,
    pub exact_page_truncated: bool,
    pub exact_page_items: Vec<LibraryWatchSetupItem>,
    pub items: Vec<LibraryWatchSetupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LibraryWatchReviewReason {
    ProviderNeeded,
    ReferenceOnly,
    CheckFailed,
    UnknownResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchReviewItem {
    pub file_id: i64,
    pub filename: String,
    pub creator: Option<String>,
    pub subject_label: String,
    pub installed_version: Option<String>,
    pub watch_result: WatchResult,
    pub review_reason: LibraryWatchReviewReason,
    pub review_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchReviewResponse {
    pub total: i64,
    pub provider_needed_count: i64,
    pub reference_only_count: i64,
    pub check_failed_count: i64,
    pub unknown_result_count: i64,
    pub items: Vec<LibraryWatchReviewItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveLibraryWatchSourceEntry {
    pub file_id: i64,
    pub source_kind: WatchSourceKind,
    pub source_label: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchBulkSaveItemResult {
    pub file_id: i64,
    pub saved: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWatchBulkSaveResult {
    pub saved_count: i64,
    pub failed_count: i64,
    pub results: Vec<LibraryWatchBulkSaveItemResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFacets {
    pub creators: Vec<String>,
    pub kinds: Vec<String>,
    pub subtypes: Vec<String>,
    pub sources: Vec<String>,
    pub taxonomy_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateOverview {
    pub total_pairs: i64,
    pub exact_pairs: i64,
    pub filename_pairs: i64,
    pub version_pairs: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatePair {
    pub id: i64,
    pub duplicate_type: String,
    pub detection_method: String,
    pub is_duplicate: bool,
    pub comparison_kind: String,
    pub classification: String,
    pub classification_label: String,
    pub confidence_label: String,
    pub evidence: Vec<String>,
    pub cautions: Vec<String>,
    pub primary_file_id: i64,
    pub primary_filename: String,
    pub primary_path: String,
    pub primary_creator: Option<String>,
    pub primary_hash: Option<String>,
    pub primary_modified_at: Option<String>,
    pub primary_size: i64,
    pub secondary_file_id: i64,
    pub secondary_filename: String,
    pub secondary_path: String,
    pub secondary_creator: Option<String>,
    pub secondary_hash: Option<String>,
    pub secondary_modified_at: Option<String>,
    pub secondary_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDetail {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub extension: String,
    pub kind: String,
    pub subtype: Option<String>,
    pub confidence: f64,
    pub source_location: String,
    pub size: i64,
    pub modified_at: Option<String>,
    pub creator: Option<String>,
    pub bundle_name: Option<String>,
    pub bundle_type: Option<String>,
    pub grouped_file_count: Option<i64>,
    pub relative_depth: i64,
    pub safety_notes: Vec<String>,
    pub hash: Option<String>,
    pub created_at: Option<String>,
    pub parser_warnings: Vec<String>,
    pub insights: FileInsights,
    pub installed_version_summary: Option<InstalledVersionSummary>,
    pub watch_result: Option<WatchResult>,
    pub creator_learning: CreatorLearningInfo,
    pub category_override: CategoryOverrideInfo,
    /// Number of exact deterministic duplicate pairs this file appears in.
    #[serde(default)]
    pub duplicates_count: usize,
    /// Exact duplicate evidence types this file is involved in.
    #[serde(default)]
    pub duplicate_types: Vec<String>,
    /// Installed version string, if this file has a watch source with version data.
    #[serde(default)]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub problem_signals: Vec<ProblemSignal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulePreset {
    pub name: String,
    pub template: String,
    pub priority: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSuggestion {
    pub file_id: i64,
    pub filename: String,
    pub current_path: String,
    pub suggested_relative_path: String,
    pub suggested_absolute_path: Option<String>,
    pub final_relative_path: String,
    pub final_absolute_path: Option<String>,
    pub rule_label: String,
    pub validator_notes: Vec<String>,
    pub review_required: bool,
    pub corrected: bool,
    pub confidence: f64,
    pub kind: String,
    pub creator: Option<String>,
    pub source_location: String,
    pub bundle_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewIssueSummary {
    pub code: String,
    pub label: String,
    pub count: i64,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPreview {
    pub preset_name: String,
    pub detected_structure: String,
    pub total_considered: i64,
    pub safe_count: i64,
    pub aligned_count: i64,
    pub corrected_count: i64,
    pub review_count: i64,
    pub recommended_preset: String,
    pub recommended_reason: String,
    pub issue_summary: Vec<PreviewIssueSummary>,
    pub suggestions: Vec<PreviewSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQueueItem {
    pub id: i64,
    pub file_id: i64,
    pub filename: String,
    pub path: String,
    pub reason: String,
    pub confidence: f64,
    pub kind: String,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub suggested_path: Option<String>,
    pub safety_notes: Vec<String>,
    pub source_location: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: i64,
    pub snapshot_name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPreviewResult {
    pub snapshot_id: i64,
    pub moved_count: i64,
    pub deferred_review_count: i64,
    pub skipped_count: i64,
    pub snapshot_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchApplyResult {
    pub applied_count: i64,
    pub skipped_count: i64,
    pub failed_count: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreItemsResult {
    pub ignored_count: i64,
    pub failed_count: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectResult {
    pub item_id: i64,
    pub reject_path: String,
    pub file_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedItem {
    pub item_id: i64,
    pub display_name: String,
    pub rejected_at: String,
    pub file_count: i64,
    pub reject_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSnapshotResult {
    pub snapshot_id: i64,
    pub restored_count: i64,
    pub skipped_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorAuditQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub min_group_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorAuditFile {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub kind: String,
    pub subtype: Option<String>,
    pub confidence: f64,
    pub source_location: String,
    pub current_creator: Option<String>,
    pub alias_samples: Vec<String>,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorAuditGroup {
    pub id: String,
    pub suggested_creator: String,
    pub confidence: f64,
    pub known_creator: bool,
    pub item_count: i64,
    pub dominant_kind: String,
    pub source_signals: Vec<String>,
    pub alias_samples: Vec<String>,
    pub file_ids: Vec<i64>,
    pub sample_files: Vec<CreatorAuditFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorAuditResponse {
    pub total_candidate_files: i64,
    pub grouped_files: i64,
    pub unresolved_files: i64,
    pub root_loose_files: i64,
    pub total_groups: i64,
    pub high_confidence_groups: i64,
    pub groups: Vec<CreatorAuditGroup>,
    pub unresolved_samples: Vec<CreatorAuditFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyCreatorAuditResult {
    pub creator_name: String,
    pub updated_count: i64,
    pub cleared_review_count: i64,
    pub locked_route: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAuditQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub min_group_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAuditFile {
    pub id: i64,
    pub filename: String,
    pub path: String,
    pub current_kind: String,
    pub current_subtype: Option<String>,
    pub confidence: f64,
    pub source_location: String,
    pub keyword_samples: Vec<String>,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAuditGroup {
    pub id: String,
    pub suggested_kind: String,
    pub suggested_subtype: Option<String>,
    pub confidence: f64,
    pub item_count: i64,
    pub source_signals: Vec<String>,
    pub keyword_samples: Vec<String>,
    pub file_ids: Vec<i64>,
    pub sample_files: Vec<CategoryAuditFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAuditResponse {
    pub total_candidate_files: i64,
    pub grouped_files: i64,
    pub unresolved_files: i64,
    pub unknown_files: i64,
    pub total_groups: i64,
    pub high_confidence_groups: i64,
    pub groups: Vec<CategoryAuditGroup>,
    pub unresolved_samples: Vec<CategoryAuditFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyCategoryAuditResult {
    pub kind: String,
    pub subtype: Option<String>,
    pub updated_count: i64,
    pub cleared_review_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsInboxQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadIntakeMode {
    Standard,
    Guided,
    NeedsReview,
    Blocked,
}

impl Default for DownloadIntakeMode {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadRiskLevel {
    Low,
    Medium,
    High,
}

impl Default for DownloadRiskLevel {
    fn default() -> Self {
        Self::Low
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSourceInfo {
    pub official_source_url: Option<String>,
    pub official_download_url: Option<String>,
    pub latest_check_url: Option<String>,
    pub latest_check_strategy: Option<String>,
    pub reference_source: Vec<String>,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewPlanActionKind {
    RepairSpecial,
    InstallDependency,
    OpenDependency,
    OpenRelatedItem,
    DownloadMissingFiles,
    OpenOfficialSource,
    SeparateSupportedFiles,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlanAction {
    pub kind: ReviewPlanActionKind,
    pub label: String,
    pub description: String,
    pub priority: i64,
    pub related_item_id: Option<i64>,
    pub related_item_name: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub key: String,
    pub display_name: String,
    pub status: String,
    pub summary: String,
    pub inbox_item_id: Option<i64>,
    pub inbox_item_name: Option<String>,
    pub inbox_item_intake_mode: Option<DownloadIntakeMode>,
    pub inbox_item_guided_install_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialDecisionState {
    GuidedReady,
    RepairBeforeUpdate,
    InstallDependencyFirst,
    OpenDependencyItem,
    OpenRelatedItem,
    DownloadMissingFiles,
    OpenOfficialSource,
    SeparateSupportedFiles,
    ReviewManually,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialLocalPackState {
    Complete,
    Partial,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialExistingInstallState {
    NotInstalled,
    Clean,
    Repairable,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialVersionStatus {
    NotInstalled,
    IncomingNewer,
    SameVersion,
    IncomingOlder,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecialFamilyRole {
    Primary,
    Related,
    Superseded,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadQueueLane {
    ReadyNow,
    SpecialSetup,
    WaitingOnYou,
    Blocked,
    Done,
    Rejected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsTimelineEntry {
    pub label: String,
    pub detail: Option<String>,
    pub at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialInstalledState {
    pub profile_key: String,
    pub profile_name: String,
    pub install_state: SpecialExistingInstallState,
    pub install_path: Option<String>,
    pub installed_version: Option<String>,
    pub installed_signature: Option<String>,
    pub source_item_id: Option<i64>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialOfficialLatestInfo {
    pub source_url: Option<String>,
    pub download_url: Option<String>,
    pub latest_version: Option<String>,
    pub checked_at: Option<String>,
    pub confidence: f64,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialModDecision {
    pub item_id: i64,
    pub profile_key: String,
    pub profile_name: String,
    pub special_family: String,
    pub state: SpecialDecisionState,
    pub local_pack_state: SpecialLocalPackState,
    pub existing_install_state: SpecialExistingInstallState,
    pub installed_state: SpecialInstalledState,
    pub family_role: SpecialFamilyRole,
    pub family_key: String,
    pub primary_family_item_id: Option<i64>,
    pub primary_family_item_name: Option<String>,
    pub sibling_item_ids: Vec<i64>,
    pub queue_lane: DownloadQueueLane,
    pub queue_summary: String,
    pub explanation: String,
    pub recommended_next_step: String,
    pub incoming_version: Option<String>,
    pub incoming_signature: Option<String>,
    pub incoming_version_source: Option<String>,
    pub incoming_version_evidence: Vec<String>,
    pub installed_version_source: Option<String>,
    pub installed_version_evidence: Vec<String>,
    pub comparison_source: Option<String>,
    pub comparison_evidence: Vec<String>,
    pub version_status: SpecialVersionStatus,
    pub same_version: bool,
    pub official_latest: Option<SpecialOfficialLatestInfo>,
    pub apply_ready: bool,
    pub available_actions: Vec<ReviewPlanAction>,
    pub primary_action: Option<ReviewPlanAction>,
}

/// MCCC auto-update info returned by check_mccc_update command.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McccUpdateInfo {
    /// Whether MCCC is currently installed.
    pub is_installed: bool,
    /// The currently installed version, if detected.
    pub installed_version: Option<String>,
    /// The path where MCCC is installed.
    pub install_path: Option<String>,
    /// The latest version available on the official page.
    pub latest_version: Option<String>,
    /// Direct URL to download the latest MCCC zip.
    pub download_url: Option<String>,
    /// When the latest version was checked.
    pub checked_at: Option<String>,
    /// Whether a newer version is available.
    pub update_available: bool,
    /// Confidence score of the latest version detection (0.0 to 1.0).
    pub confidence: f64,
    /// Status message or note.
    pub status: String,
    /// Error message if the check failed.
    pub error: Option<String>,
}

/// Result of applying an MCCC update.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMcccUpdateResult {
    /// The new version installed.
    pub new_version: String,
    /// Number of files installed.
    pub installed_count: i64,
    /// Number of files replaced.
    pub replaced_count: i64,
    /// Number of files preserved (.cfg files kept).
    pub preserved_count: i64,
    /// ID of the snapshot created before the update.
    pub snapshot_id: i64,
    /// Name of the snapshot.
    pub snapshot_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsInboxItem {
    pub id: i64,
    pub display_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub archive_format: Option<String>,
    pub status: String,
    pub source_size: i64,
    pub detected_file_count: i64,
    pub active_file_count: i64,
    pub applied_file_count: i64,
    pub review_file_count: i64,
    pub creator_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub updated_at: String,
    pub error_message: Option<String>,
    pub sample_files: Vec<String>,
    pub notes: Vec<String>,
    pub intake_mode: DownloadIntakeMode,
    pub risk_level: DownloadRiskLevel,
    pub matched_profile_key: Option<String>,
    pub matched_profile_name: Option<String>,
    pub special_family: Option<String>,
    pub assessment_reasons: Vec<String>,
    pub dependency_summary: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub inbox_dependencies: Vec<String>,
    pub incompatibility_warnings: Vec<String>,
    pub post_install_notes: Vec<String>,
    pub evidence_summary: Vec<String>,
    pub catalog_source: Option<CatalogSourceInfo>,
    pub existing_install_detected: bool,
    pub guided_install_available: bool,
    pub queue_lane: DownloadQueueLane,
    pub queue_summary: String,
    pub family_key: Option<String>,
    pub related_item_ids: Vec<i64>,
    pub timeline: Vec<DownloadsTimelineEntry>,
    pub special_decision: Option<SpecialModDecision>,
    pub version_resolution: Option<VersionResolution>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsInboxOverview {
    pub total_items: i64,
    pub ready_items: i64,
    pub needs_review_items: i64,
    pub applied_items: i64,
    pub error_items: i64,
    pub active_files: i64,
    pub watched_path: Option<String>,
    pub ready_now_items: i64,
    pub special_setup_items: i64,
    pub waiting_on_you_items: i64,
    pub blocked_items: i64,
    pub done_items: i64,
    pub rejected_items: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsInboxResponse {
    pub overview: DownloadsInboxOverview,
    pub items: Vec<DownloadsInboxItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsBootstrapResponse {
    pub watcher_status: DownloadsWatcherStatus,
    pub queue: Option<DownloadsInboxResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsSelectionResponse {
    pub item_id: i64,
    pub detail: Option<DownloadInboxDetail>,
    pub preview: Option<OrganizationPreview>,
    pub guided_plan: Option<GuidedInstallPlan>,
    pub review_plan: Option<SpecialReviewPlan>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInboxFile {
    pub file_id: i64,
    pub filename: String,
    pub current_path: String,
    pub origin_path: String,
    pub archive_member_path: Option<String>,
    pub kind: String,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub confidence: f64,
    pub size: i64,
    pub source_location: String,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuidedInstallFileEntry {
    pub file_id: Option<i64>,
    pub filename: String,
    pub current_path: String,
    pub target_path: Option<String>,
    pub archive_member_path: Option<String>,
    pub kind: String,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuidedInstallPlan {
    pub item_id: i64,
    pub profile_key: String,
    pub profile_name: String,
    pub special_family: Option<String>,
    pub install_target_folder: String,
    pub install_files: Vec<GuidedInstallFileEntry>,
    pub replace_files: Vec<GuidedInstallFileEntry>,
    pub preserve_files: Vec<GuidedInstallFileEntry>,
    pub review_files: Vec<GuidedInstallFileEntry>,
    pub dependencies: Vec<DependencyStatus>,
    pub incompatibility_warnings: Vec<String>,
    pub post_install_notes: Vec<String>,
    pub existing_layout_findings: Vec<String>,
    pub warnings: Vec<String>,
    pub explanation: String,
    pub evidence: Vec<String>,
    pub catalog_source: Option<CatalogSourceInfo>,
    pub existing_install_detected: bool,
    pub apply_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialReviewPlan {
    pub item_id: i64,
    pub mode: DownloadIntakeMode,
    pub profile_key: Option<String>,
    pub profile_name: Option<String>,
    pub special_family: Option<String>,
    pub explanation: String,
    pub recommended_next_step: String,
    pub dependencies: Vec<DependencyStatus>,
    pub incompatibility_warnings: Vec<String>,
    pub review_files: Vec<GuidedInstallFileEntry>,
    pub evidence: Vec<String>,
    pub existing_layout_findings: Vec<String>,
    pub post_install_notes: Vec<String>,
    pub catalog_source: Option<CatalogSourceInfo>,
    pub available_actions: Vec<ReviewPlanAction>,
    pub repair_plan_available: bool,
    pub repair_action_label: Option<String>,
    pub repair_reason: Option<String>,
    pub repair_target_folder: Option<String>,
    pub repair_move_files: Vec<GuidedInstallFileEntry>,
    pub repair_replace_files: Vec<GuidedInstallFileEntry>,
    pub repair_keep_files: Vec<GuidedInstallFileEntry>,
    pub repair_warnings: Vec<String>,
    pub repair_can_continue_install: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInboxDetail {
    pub item: DownloadsInboxItem,
    pub files: Vec<DownloadInboxFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyGuidedDownloadResult {
    pub snapshot_id: i64,
    pub installed_count: i64,
    pub replaced_count: i64,
    pub preserved_count: i64,
    pub deferred_review_count: i64,
    pub snapshot_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySpecialReviewFixResult {
    pub snapshot_id: i64,
    pub repaired_count: i64,
    pub installed_count: i64,
    pub replaced_count: i64,
    pub preserved_count: i64,
    pub deferred_review_count: i64,
    pub snapshot_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReviewPlanActionResult {
    pub action_kind: ReviewPlanActionKind,
    pub focus_item_id: i64,
    pub created_item_id: Option<i64>,
    pub opened_url: Option<String>,
    pub snapshot_id: Option<i64>,
    pub repaired_count: i64,
    pub installed_count: i64,
    pub replaced_count: i64,
    pub preserved_count: i64,
    pub deferred_review_count: i64,
    pub snapshot_name: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingCommitResult {
    pub committed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}
