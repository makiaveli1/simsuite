use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    pub mods_path: Option<String>,
    pub tray_path: Option<String>,
    pub downloads_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBehaviorSettings {
    pub keep_running_in_background: bool,
    pub automatic_watch_checks: bool,
    pub watch_check_interval_hours: i64,
    pub last_watch_check_at: Option<String>,
    pub last_watch_check_error: Option<String>,
    pub curseforge_api_key: Option<String>,
    pub github_api_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRefreshSummary {
    pub checked_subjects: i64,
    pub exact_update_items: i64,
    pub possible_update_items: i64,
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
    Updates,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    pub search: Option<String>,
    pub kind: Option<String>,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub source: Option<String>,
    pub min_confidence: Option<f64>,
    pub unsafe_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchResult {
    pub status: WatchStatus,
    pub source_kind: Option<WatchSourceKind>,
    pub source_origin: WatchSourceOrigin,
    pub source_label: Option<String>,
    pub source_url: Option<String>,
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
    pub format: Option<String>,
    pub resource_summary: Vec<String>,
    pub script_namespaces: Vec<String>,
    pub embedded_names: Vec<String>,
    pub creator_hints: Vec<String>,
    pub version_hints: Vec<String>,
    pub version_signals: Vec<VersionSignal>,
    pub family_hints: Vec<String>,
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
    pub relative_depth: i64,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryListResponse {
    pub total: i64,
    pub items: Vec<LibraryFileRow>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    CurseForge,
    GitHub,
    Nexus,
    Feed,
    StructuredPage,
    GenericPage,
    EaBrokenMods,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessTier {
    #[default]
    Public,
    PatronOnly,
    EarlyAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingMode {
    #[default]
    DetectedOnly,
    Auto,
    Manual,
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    #[default]
    Untracked,
    UpToDate,
    ConfirmedUpdate,
    ProbableUpdate,
    SourceActivity,
    SourceUnreachable,
    NeedsGameUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalMod {
    pub id: String,
    pub display_name: String,
    pub normalized_name: String,
    pub creator_name: Option<String>,
    pub category: Option<String>,
    pub local_root_path: String,
    pub tracking_mode: TrackingMode,
    pub source_confidence: f64,
    pub confirmed_source_id: Option<String>,
    pub current_status: UpdateStatus,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFile {
    pub id: String,
    pub local_mod_id: String,
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub file_size: i64,
    pub sha256: Option<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTrackingPrefs {
    pub local_mod_id: String,
    pub ignore_updates: bool,
    pub ignore_versions: Vec<String>,
    pub notify_on_probable: bool,
    pub notify_on_source_activity: bool,
    pub manual_source_url: Option<String>,
    pub pinned_source_kind: Option<SourceKind>,
    pub custom_check_interval_hours: Option<i32>,
    pub fingerprint_enabled: bool,
    pub ea_broken_mods_enabled: bool,
    pub ea_broken_mods_custom_url: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

impl Default for UserTrackingPrefs {
    fn default() -> Self {
        Self {
            local_mod_id: String::new(),
            ignore_updates: false,
            ignore_versions: Vec::new(),
            notify_on_probable: false,
            notify_on_source_activity: false,
            manual_source_url: None,
            pinned_source_kind: None,
            custom_check_interval_hours: None,
            fingerprint_enabled: false,
            ea_broken_mods_enabled: true,
            ea_broken_mods_custom_url: None,
            custom_headers: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBinding {
    pub id: String,
    pub local_mod_id: String,
    pub source_kind: SourceKind,
    pub source_url: String,
    pub provider_mod_id: Option<String>,
    pub provider_file_id: Option<String>,
    pub provider_repo: Option<String>,
    pub bind_method: String,
    pub is_primary: bool,
    pub custom_headers_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
