use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySettings {
    pub mods_path: Option<String>,
    pub tray_path: Option<String>,
    pub downloads_path: Option<String>,
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
    pub last_scan_at: Option<String>,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    pub search: Option<String>,
    pub kind: Option<String>,
    pub subtype: Option<String>,
    pub creator: Option<String>,
    pub source: Option<String>,
    pub min_confidence: Option<f64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileInsights {
    pub format: Option<String>,
    pub resource_summary: Vec<String>,
    pub script_namespaces: Vec<String>,
    pub embedded_names: Vec<String>,
    pub creator_hints: Vec<String>,
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
pub struct OrganizationPreview {
    pub preset_name: String,
    pub detected_structure: String,
    pub total_considered: i64,
    pub corrected_count: i64,
    pub review_count: i64,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsInboxResponse {
    pub overview: DownloadsInboxOverview,
    pub items: Vec<DownloadsInboxItem>,
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
pub struct DownloadInboxDetail {
    pub item: DownloadsInboxItem,
    pub files: Vec<DownloadInboxFile>,
}
