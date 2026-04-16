export type Screen =
  | "home"
  | "downloads"
  | "library"
  | "updates"
  | "creatorAudit"
  | "categoryAudit"
  | "organize"
  | "review"
  | "duplicates"
  | "settings"
  | "staging";
export type ExperienceMode = "casual" | "seasoned" | "creator";
export type UserView = "beginner" | "standard" | "power";
export type UiTheme =
  | "plumbob"
  | "buildbuy"
  | "cas"
  | "neighborhood"
  | "debuggrid"
  | "sunroom"
  | "patchday"
  | "nightmarket";
export type UiDensity = "compact" | "balanced" | "roomy";
export type LibraryLayoutPreset = "browse" | "inspect" | "catalog" | "custom";
export type ReviewLayoutPreset = "queue" | "balanced" | "focus" | "custom";
export type DuplicatesLayoutPreset =
  | "sweep"
  | "balanced"
  | "compare"
  | "custom";

export interface LibrarySettings {
  modsPath: string | null;
  trayPath: string | null;
  downloadsPath: string | null;
  downloadRejectFolder: string | null;
}

export interface AppBehaviorSettings {
  keepRunningInBackground: boolean;
  automaticWatchChecks: boolean;
  watchCheckIntervalHours: number;
  lastWatchCheckAt: string | null;
  lastWatchCheckError: string | null;
  downloadIgnorePatterns: string[];
  silentSpecialModUpdates: boolean | null;
}

export interface WatchRefreshSummary {
  checkedSubjects: number;
  exactUpdateItems: number;
  possibleUpdateItems: number;
  unknownWatchItems: number;
  checkedAt: string;
}

export interface DetectedLibraryPaths {
  modsPath: string | null;
  trayPath: string | null;
  downloadsPath: string | null;
}

export interface HomeOverview {
  totalFiles: number;
  modsCount: number;
  trayCount: number;
  downloadsCount: number;
  scriptModsCount: number;
  creatorCount: number;
  bundlesCount: number;
  duplicatesCount: number;
  reviewCount: number;
  unsafeCount: number;
  exactUpdateItems: number;
  possibleUpdateItems: number;
  unknownWatchItems: number;
  watchReviewItems: number;
  watchSetupItems: number;
  lastScanAt: string | null;
  scanNeedsRefresh: boolean;
  readOnlyMode: boolean;
}

export type DownloadsWatcherState = "idle" | "watching" | "processing" | "error";

export interface DownloadsWatcherStatus {
  state: DownloadsWatcherState;
  watchedPath: string | null;
  configured: boolean;
  currentItem: string | null;
  lastRunAt: string | null;
  lastChangeAt: string | null;
  lastError: string | null;
  readyItems: number;
  needsReviewItems: number;
  activeItems: number;
  last_downloads_scan_at: string | null;
  progress: DownloadProgress | null;
}

export interface DownloadProgress {
  phase: string;
  currentFile: string;
  processedCount: number;
  totalCount: number;
  bytesProcessed: number;
  bytesTotal: number;
}

export interface StagingSubDirectory {
  path: string;
  name: string;
  fileCount: number;
  totalBytes: number;
  createdAt: string | null;
}

export interface StagingArea {
  itemId: string;
  subdirectories: StagingSubDirectory[];
}

export interface StagingAreasSummary {
  areas: StagingArea[];
  totalBytes: number;
  totalFileCount: number;
}

export interface CleanupResult {
  deletedCount: number;
  freedBytes: number;
  errors: string[];
}

export interface StagingCommitResult {
  committedCount: number;
  skippedCount: number;
  failedCount: number;
  errors: string[];
}

export type ScanMode = "full" | "incremental";
export type ScanPhase =
  | "collecting"
  | "hashing"
  | "classifying"
  | "bundling"
  | "duplicates"
  | "done";

export interface ScanProgress {
  totalFiles: number;
  processedFiles: number;
  currentItem: string;
  phase: ScanPhase;
}

export interface ScanSummary {
  sessionId: number;
  scanMode: ScanMode;
  filesScanned: number;
  reusedFiles: number;
  newFiles: number;
  updatedFiles: number;
  removedFiles: number;
  hashedFiles: number;
  reviewItemsCreated: number;
  bundlesDetected: number;
  duplicatesDetected: number;
  errors: string[];
}

export type ScanRuntimeState = "idle" | "running" | "succeeded" | "failed";

export interface ScanStatus {
  state: ScanRuntimeState;
  mode: ScanMode | null;
  phase: ScanPhase | null;
  totalFiles: number;
  processedFiles: number;
  currentItem: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  lastSummary: ScanSummary | null;
  error: string | null;
}

export type WorkspaceDomain =
  | "home"
  | "downloads"
  | "library"
  | "updates"
  | "organize"
  | "review"
  | "duplicates"
  | "creatorAudit"
  | "categoryAudit"
  | "snapshots";

export interface WorkspaceChange {
  domains: WorkspaceDomain[];
  reason: string;
  itemIds: number[];
  familyKeys: string[];
}

export interface LibraryQuery {
  search?: string;
  kind?: string;
  subtype?: string;
  creator?: string;
  source?: string;
  minConfidence?: number;
  watchFilter?: LibraryWatchFilter;
  sortBy?: LibrarySortField;
  limit?: number;
  offset?: number;
}

export type LibraryWatchFilter = "all" | "has_updates" | "needs_attention" | "not_tracked" | "duplicates";

export type LibrarySortField = "name" | "creator" | "recently_modified" | "has_updates_first";

export interface LibraryFileRow {
  id: number;
  filename: string;
  path: string;
  extension: string;
  kind: string;
  subtype: string | null;
  confidence: number;
  sourceLocation: string;
  size: number;
  modifiedAt: string | null;
  creator: string | null;
  bundleName: string | null;
  bundleType: string | null;
  groupedFileCount?: number | null;
  relativeDepth: number;
  safetyNotes: string[];
  insights?: FileInsights;
  /** Watch status for this file, if tracked. */
  watchStatus?: WatchStatus;
  /** Parser warnings for this file (e.g. missing script, unusual structure). */
  parserWarnings: string[];
  /** True if this file appears in any duplicate pair (exact, filename, or version). */
  hasDuplicate?: boolean;
  /** Installed version string, if this file has a watch source with version data. */
  installedVersion?: string | null;
}

export interface LibraryListResponse {
  total: number;
  items: LibraryFileRow[];
}

export interface LibrarySummary {
  total: number;
  tracked: number;
  notTracked: number;
  hasUpdates: number;
  needsReview: number;
  duplicates: number;
  disabled: number;
}

export type WatchListFilter =
  | "attention"
  | "exact_updates"
  | "possible_updates"
  | "unclear"
  | "all";

export type LibraryWatchFocusTarget =
  | "tracked_attention"
  | "tracked_exact_updates"
  | "tracked_possible_updates"
  | "tracked_unclear"
  | "tracked_all"
  | "setup";

export interface LibraryWatchFocusRequest {
  id: number;
  target: LibraryWatchFocusTarget;
}

export interface LibraryWatchListItem {
  fileId: number;
  filename: string;
  creator: string | null;
  subjectLabel: string;
  installedVersion: string | null;
  watchResult: WatchResult;
}

export interface LibraryWatchListResponse {
  filter: WatchListFilter;
  total: number;
  items: LibraryWatchListItem[];
}

export interface LibraryWatchSetupItem {
  fileId: number;
  filename: string;
  creator: string | null;
  subjectLabel: string;
  installedVersion: string | null;
  suggestedSourceKind: WatchSourceKind;
  setupHint: string;
}

export interface LibraryWatchSetupResponse {
  total: number;
  truncated: boolean;
  exactPageTotal: number;
  exactPageTruncated: boolean;
  exactPageItems: LibraryWatchSetupItem[];
  items: LibraryWatchSetupItem[];
}

export type LibraryWatchReviewReason =
  | "provider_needed"
  | "reference_only"
  | "unknown_result";

export interface LibraryWatchReviewItem {
  fileId: number;
  filename: string;
  creator: string | null;
  subjectLabel: string;
  installedVersion: string | null;
  watchResult: WatchResult;
  reviewReason: LibraryWatchReviewReason;
  reviewHint: string;
}

export interface LibraryWatchReviewResponse {
  total: number;
  providerNeededCount: number;
  referenceOnlyCount: number;
  unknownResultCount: number;
  items: LibraryWatchReviewItem[];
}

export interface SaveLibraryWatchSourceEntry {
  fileId: number;
  sourceKind: WatchSourceKind;
  sourceLabel?: string;
  sourceUrl: string;
}

export interface LibraryWatchBulkSaveItemResult {
  fileId: number;
  saved: boolean;
  message: string;
}

export interface LibraryWatchBulkSaveResult {
  savedCount: number;
  failedCount: number;
  results: LibraryWatchBulkSaveItemResult[];
}

export interface LibraryFacets {
  creators: string[];
  kinds: string[];
  subtypes: string[];
  sources: string[];
  taxonomyKinds: string[];
}

export interface DuplicateOverview {
  totalPairs: number;
  exactPairs: number;
  filenamePairs: number;
  versionPairs: number;
}

export interface DuplicatePair {
  id: number;
  duplicateType: string;
  detectionMethod: string;
  primaryFileId: number;
  primaryFilename: string;
  primaryPath: string;
  primaryCreator: string | null;
  primaryHash: string | null;
  primaryModifiedAt: string | null;
  primarySize: number;
  secondaryFileId: number;
  secondaryFilename: string;
  secondaryPath: string;
  secondaryCreator: string | null;
  secondaryHash: string | null;
  secondaryModifiedAt: string | null;
  secondarySize: number;
}

export interface VersionSignal {
  rawValue: string;
  normalizedValue: string;
  sourceKind: string;
  sourcePath: string | null;
  matchedBy: string | null;
  confidence: number;
}

export type VersionCompareStatus =
  | "not_installed"
  | "incoming_newer"
  | "same_version"
  | "incoming_older"
  | "unknown";

export type VersionConfidence =
  | "exact"
  | "strong"
  | "medium"
  | "weak"
  | "unknown";

export interface VersionResolution {
  subjectLabel: string | null;
  matchedSubjectLabel: string | null;
  matchedSubjectKey: string | null;
  status: VersionCompareStatus;
  confidence: VersionConfidence;
  matchScore: number;
  incomingVersion: string | null;
  installedVersion: string | null;
  incomingSignature: string | null;
  installedSignature: string | null;
  evidence: string[];
  incomingEvidence: string[];
  installedEvidence: string[];
}

export interface LibraryVersionInfo {
  libraryLabel: string | null;
  installedVersion: string | null;
  incomingVersion: string | null;
}

export interface InstalledVersionSummary {
  subjectLabel: string;
  subjectKey: string;
  version: string | null;
  signature: string | null;
  confidence: VersionConfidence;
  evidence: string[];
}

export type WatchSourceKind = "exact_page" | "creator_page";
export type WatchSourceOrigin = "none" | "saved_by_user" | "built_in_special";
export type WatchCapability =
  | "can_refresh_now"
  | "saved_reference_only"
  | "provider_required";

export type WatchStatus =
  | "not_watched"
  | "current"
  | "exact_update_available"
  | "possible_update"
  | "unknown";

export type UpdateSource = "curseforge" | "modrinth" | "patreon" | "website" | "unknown";

export interface WatchResult {
  status: WatchStatus;
  sourceKind: WatchSourceKind | null;
  sourceOrigin: WatchSourceOrigin;
  sourceLabel: string | null;
  sourceUrl: string | null;
  source?: UpdateSource | null;
  patreonUrl?: string | null;
  capability: WatchCapability;
  canRefreshNow: boolean;
  providerName: string | null;
  latestVersion: string | null;
  checkedAt: string | null;
  confidence: VersionConfidence;
  note: string | null;
  evidence: string[];
}

/** Source of the thumbnail preview image */
export type PreviewSource = 'embedded' | 'cache' | 'external' | 'fallback';

export interface FileInsights {
  format: string | null;
  resourceSummary: string[];
  scriptNamespaces: string[];
  embeddedNames: string[];
  creatorHints: string[];
  versionHints: string[];
  versionSignals: VersionSignal[];
  familyHints: string[];
  /** Base64 PNG of the CAS custom thumbnail (THUM 0x3C1AF1F2), if available. */
  thumbnailPreview?: string | null;
  /**
   * Base64 PNG from localthumbcache.package — the game's own thumbnail cache.
   * This is typically larger/higher quality than embedded THUM.
   * Unavailable if Sims 4 is not installed or the package has no cached thumbnail.
   */
  cachedThumbnailPreview?: string | null;
  /** Where the thumbnail came from — embedded THUM, localthumbcache, external, or fallback */
  previewSource?: PreviewSource;
}

export interface CreatorLearningInfo {
  lockedByUser: boolean;
  preferredPath: string | null;
  learnedAliases: string[];
}

export interface CategoryOverrideInfo {
  savedByUser: boolean;
  kind: string | null;
  subtype: string | null;
}

export interface FileDetail extends LibraryFileRow {
  hash: string | null;
  createdAt: string | null;
  parserWarnings: string[];
  insights: FileInsights;
  installedVersionSummary: InstalledVersionSummary | null;
  watchResult: WatchResult | null;
  creatorLearning: CreatorLearningInfo;
  categoryOverride: CategoryOverrideInfo;
  /** How many duplicate pairs this file appears in. */
  duplicatesCount: number;
  /** Types of duplicates: 'exact', 'filename', 'version'. */
  duplicateTypes: string[];
}

export interface RulePreset {
  name: string;
  template: string;
  priority: number;
  description: string;
}

export interface PreviewSuggestion {
  fileId: number;
  filename: string;
  currentPath: string;
  suggestedRelativePath: string;
  suggestedAbsolutePath: string | null;
  finalRelativePath: string;
  finalAbsolutePath: string | null;
  ruleLabel: string;
  validatorNotes: string[];
  reviewRequired: boolean;
  corrected: boolean;
  confidence: number;
  kind: string;
  creator: string | null;
  sourceLocation: string;
  bundleName: string | null;
}

export interface PreviewIssueSummary {
  code: string;
  label: string;
  count: number;
  tone: string;
}

export interface OrganizationPreview {
  presetName: string;
  detectedStructure: string;
  totalConsidered: number;
  safeCount: number;
  alignedCount: number;
  correctedCount: number;
  reviewCount: number;
  recommendedPreset: string;
  recommendedReason: string;
  issueSummary: PreviewIssueSummary[];
  suggestions: PreviewSuggestion[];
}

export interface ReviewQueueItem {
  id: number;
  fileId: number;
  filename: string;
  path: string;
  reason: string;
  confidence: number;
  kind: string;
  subtype: string | null;
  creator: string | null;
  suggestedPath: string | null;
  safetyNotes: string[];
  sourceLocation: string;
}

export interface SnapshotSummary {
  id: number;
  snapshotName: string;
  description: string | null;
  createdAt: string;
  itemCount: number;
}

export interface ApplyPreviewResult {
  snapshotId: number;
  movedCount: number;
  deferredReviewCount: number;
  skippedCount: number;
  snapshotName: string;
}

export interface BatchApplyResult {
  appliedCount: number;
  skippedCount: number;
  failedCount: number;
  errors: string[];
}

export interface IgnoreItemsResult {
  ignoredCount: number;
  failedCount: number;
  errors: string[];
}

export interface RejectResult {
  itemId: number;
  rejectPath: string;
  fileCount: number;
}

export interface RejectedItem {
  itemId: number;
  displayName: string;
  rejectedAt: string;
  fileCount: number;
  rejectPath: string;
}

export interface RestoreSnapshotResult {
  snapshotId: number;
  restoredCount: number;
  skippedCount: number;
}

export interface CreatorAuditQuery {
  search?: string;
  limit?: number;
  minGroupSize?: number;
}

export interface CreatorAuditFile {
  id: number;
  filename: string;
  path: string;
  kind: string;
  subtype: string | null;
  confidence: number;
  sourceLocation: string;
  currentCreator: string | null;
  aliasSamples: string[];
  matchReasons: string[];
}

export interface CreatorAuditGroup {
  id: string;
  suggestedCreator: string;
  confidence: number;
  knownCreator: boolean;
  itemCount: number;
  dominantKind: string;
  sourceSignals: string[];
  aliasSamples: string[];
  fileIds: number[];
  sampleFiles: CreatorAuditFile[];
}

export interface CreatorAuditResponse {
  totalCandidateFiles: number;
  groupedFiles: number;
  unresolvedFiles: number;
  rootLooseFiles: number;
  totalGroups: number;
  highConfidenceGroups: number;
  groups: CreatorAuditGroup[];
  unresolvedSamples: CreatorAuditFile[];
}

export interface ApplyCreatorAuditResult {
  creatorName: string;
  updatedCount: number;
  clearedReviewCount: number;
  lockedRoute: boolean;
}

export interface CategoryAuditQuery {
  search?: string;
  limit?: number;
  minGroupSize?: number;
}

export interface CategoryAuditFile {
  id: number;
  filename: string;
  path: string;
  currentKind: string;
  currentSubtype: string | null;
  confidence: number;
  sourceLocation: string;
  keywordSamples: string[];
  matchReasons: string[];
}

export interface CategoryAuditGroup {
  id: string;
  suggestedKind: string;
  suggestedSubtype: string | null;
  confidence: number;
  itemCount: number;
  sourceSignals: string[];
  keywordSamples: string[];
  fileIds: number[];
  sampleFiles: CategoryAuditFile[];
}

export interface CategoryAuditResponse {
  totalCandidateFiles: number;
  groupedFiles: number;
  unresolvedFiles: number;
  unknownFiles: number;
  totalGroups: number;
  highConfidenceGroups: number;
  groups: CategoryAuditGroup[];
  unresolvedSamples: CategoryAuditFile[];
}

export interface ApplyCategoryAuditResult {
  kind: string;
  subtype: string | null;
  updatedCount: number;
  clearedReviewCount: number;
}

export interface DownloadsInboxQuery {
  search?: string;
  status?: string;
  limit?: number;
}

export interface DownloadsInboxItem {
  id: number;
  displayName: string;
  sourcePath: string;
  sourceKind: string;
  archiveFormat: string | null;
  status: string;
  sourceSize: number;
  detectedFileCount: number;
  activeFileCount: number;
  appliedFileCount: number;
  reviewFileCount: number;
  creatorName: string | null;
  firstSeenAt: string;
  lastSeenAt: string;
  updatedAt: string;
  errorMessage: string | null;
  sampleFiles: string[];
  notes: string[];
  intakeMode: DownloadIntakeMode;
  riskLevel: DownloadRiskLevel;
  matchedProfileKey: string | null;
  matchedProfileName: string | null;
  specialFamily: string | null;
  assessmentReasons: string[];
  dependencySummary: string[];
  missingDependencies: string[];
  inboxDependencies: string[];
  incompatibilityWarnings: string[];
  postInstallNotes: string[];
  evidenceSummary: string[];
  catalogSource: CatalogSourceInfo | null;
  existingInstallDetected: boolean;
  guidedInstallAvailable: boolean;
  queueLane?: DownloadQueueLane;
  queueSummary?: string;
  familyKey?: string | null;
  relatedItemIds?: number[];
  timeline?: DownloadsTimelineEntry[];
  specialDecision?: SpecialModDecision | null;
  versionResolution?: VersionResolution | null;
}

export type DownloadQueueLane =
  | "ready_now"
  | "special_setup"
  | "waiting_on_you"
  | "blocked"
  | "done"
  | "rejected";

export interface DownloadsTimelineEntry {
  label: string;
  detail: string | null;
  at: string | null;
}

export interface DownloadsInboxOverview {
  totalItems: number;
  readyItems: number;
  needsReviewItems: number;
  appliedItems: number;
  errorItems: number;
  activeFiles: number;
  watchedPath: string | null;
  readyNowItems?: number;
  specialSetupItems?: number;
  waitingOnYouItems?: number;
  blockedItems?: number;
  doneItems?: number;
  rejectedItems?: number;
}

export interface DownloadsInboxResponse {
  overview: DownloadsInboxOverview;
  items: DownloadsInboxItem[];
}

export interface DownloadsBootstrapResponse {
  watcherStatus: DownloadsWatcherStatus;
  queue: DownloadsInboxResponse | null;
}

export interface DownloadsSelectionResponse {
  itemId: number;
  detail: DownloadInboxDetail | null;
  preview: OrganizationPreview | null;
  guidedPlan: GuidedInstallPlan | null;
  reviewPlan: SpecialReviewPlan | null;
}

export interface DownloadInboxFile {
  fileId: number;
  filename: string;
  currentPath: string;
  originPath: string;
  archiveMemberPath: string | null;
  kind: string;
  subtype: string | null;
  creator: string | null;
  confidence: number;
  size: number;
  sourceLocation: string;
  safetyNotes: string[];
}

export interface DownloadInboxDetail {
  item: DownloadsInboxItem;
  files: DownloadInboxFile[];
}

export type DownloadIntakeMode =
  | "standard"
  | "guided"
  | "needs_review"
  | "blocked";

export type DownloadRiskLevel = "low" | "medium" | "high";

export interface CatalogSourceInfo {
  officialSourceUrl: string | null;
  officialDownloadUrl: string | null;
  latestCheckUrl: string | null;
  latestCheckStrategy: string | null;
  referenceSource: string[];
  reviewedAt: string | null;
}

export type ReviewPlanActionKind =
  | "repair_special"
  | "install_dependency"
  | "open_dependency"
  | "open_related_item"
  | "download_missing_files"
  | "open_official_source"
  | "separate_supported_files";

export interface ReviewPlanAction {
  kind: ReviewPlanActionKind;
  label: string;
  description: string;
  priority: number;
  relatedItemId: number | null;
  relatedItemName: string | null;
  url: string | null;
}

export interface DependencyStatus {
  key: string;
  displayName: string;
  status: string;
  summary: string;
  inboxItemId: number | null;
  inboxItemName: string | null;
  inboxItemIntakeMode: DownloadIntakeMode | null;
  inboxItemGuidedInstallAvailable: boolean;
}

export type SpecialDecisionState =
  | "guided_ready"
  | "repair_before_update"
  | "install_dependency_first"
  | "open_dependency_item"
  | "open_related_item"
  | "download_missing_files"
  | "open_official_source"
  | "separate_supported_files"
  | "review_manually";

export type SpecialLocalPackState = "complete" | "partial" | "mixed" | "unknown";

export type SpecialExistingInstallState =
  | "not_installed"
  | "clean"
  | "repairable"
  | "blocked";

export type SpecialVersionStatus =
  | "not_installed"
  | "incoming_newer"
  | "same_version"
  | "incoming_older"
  | "unknown";

export type SpecialFamilyRole = "primary" | "related" | "superseded";

export interface SpecialInstalledState {
  profileKey: string;
  profileName: string;
  installState: SpecialExistingInstallState;
  installPath: string | null;
  installedVersion: string | null;
  installedSignature: string | null;
  sourceItemId: number | null;
  checkedAt: string | null;
}

export interface SpecialOfficialLatestInfo {
  sourceUrl: string | null;
  downloadUrl: string | null;
  latestVersion: string | null;
  checkedAt: string | null;
  confidence: number;
  status: string;
  note: string | null;
}

export interface SpecialModDecision {
  itemId: number;
  profileKey: string;
  profileName: string;
  specialFamily: string;
  state: SpecialDecisionState;
  localPackState: SpecialLocalPackState;
  existingInstallState: SpecialExistingInstallState;
  installedState: SpecialInstalledState;
  familyRole: SpecialFamilyRole;
  familyKey: string;
  primaryFamilyItemId: number | null;
  primaryFamilyItemName: string | null;
  siblingItemIds: number[];
  queueLane: DownloadQueueLane;
  queueSummary: string;
  explanation: string;
  recommendedNextStep: string;
  incomingVersion: string | null;
  incomingSignature: string | null;
  incomingVersionSource: string | null;
  incomingVersionEvidence: string[];
  installedVersionSource: string | null;
  installedVersionEvidence: string[];
  comparisonSource: string | null;
  comparisonEvidence: string[];
  versionStatus: SpecialVersionStatus;
  sameVersion: boolean;
  officialLatest: SpecialOfficialLatestInfo | null;
  applyReady: boolean;
  availableActions: ReviewPlanAction[];
  primaryAction: ReviewPlanAction | null;
}

export interface GuidedInstallFileEntry {
  fileId: number | null;
  filename: string;
  currentPath: string;
  targetPath: string | null;
  archiveMemberPath: string | null;
  kind: string;
  subtype: string | null;
  creator: string | null;
  notes: string[];
}

export interface GuidedInstallPlan {
  itemId: number;
  profileKey: string;
  profileName: string;
  specialFamily: string | null;
  installTargetFolder: string;
  installFiles: GuidedInstallFileEntry[];
  replaceFiles: GuidedInstallFileEntry[];
  preserveFiles: GuidedInstallFileEntry[];
  reviewFiles: GuidedInstallFileEntry[];
  dependencies: DependencyStatus[];
  incompatibilityWarnings: string[];
  postInstallNotes: string[];
  existingLayoutFindings: string[];
  warnings: string[];
  explanation: string;
  evidence: string[];
  catalogSource: CatalogSourceInfo | null;
  existingInstallDetected: boolean;
  applyReady: boolean;
}

export interface SpecialReviewPlan {
  itemId: number;
  mode: DownloadIntakeMode;
  profileKey: string | null;
  profileName: string | null;
  specialFamily: string | null;
  explanation: string;
  recommendedNextStep: string;
  dependencies: DependencyStatus[];
  incompatibilityWarnings: string[];
  reviewFiles: GuidedInstallFileEntry[];
  evidence: string[];
  existingLayoutFindings: string[];
  postInstallNotes: string[];
  catalogSource: CatalogSourceInfo | null;
  availableActions: ReviewPlanAction[];
  repairPlanAvailable: boolean;
  repairActionLabel: string | null;
  repairReason: string | null;
  repairTargetFolder: string | null;
  repairMoveFiles: GuidedInstallFileEntry[];
  repairReplaceFiles: GuidedInstallFileEntry[];
  repairKeepFiles: GuidedInstallFileEntry[];
  repairWarnings: string[];
  repairCanContinueInstall: boolean;
}

export interface ApplyGuidedDownloadResult {
  snapshotId: number;
  installedCount: number;
  replacedCount: number;
  preservedCount: number;
  deferredReviewCount: number;
  snapshotName: string;
}

export interface ApplySpecialReviewFixResult {
  snapshotId: number;
  repairedCount: number;
  installedCount: number;
  replacedCount: number;
  preservedCount: number;
  deferredReviewCount: number;
  snapshotName: string;
}

export interface ApplyReviewPlanActionResult {
  actionKind: ReviewPlanActionKind;
  focusItemId: number;
  createdItemId: number | null;
  openedUrl: string | null;
  snapshotId: number | null;
  repairedCount: number;
  installedCount: number;
  replacedCount: number;
  preservedCount: number;
  deferredReviewCount: number;
  snapshotName: string | null;
  message: string;
}
