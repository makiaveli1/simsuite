export type Screen =
  | "home"
  | "downloads"
  | "library"
  | "creatorAudit"
  | "categoryAudit"
  | "organize"
  | "review"
  | "duplicates"
  | "settings";
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
  lastScanAt: string | null;
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

export interface LibraryQuery {
  search?: string;
  kind?: string;
  subtype?: string;
  creator?: string;
  source?: string;
  minConfidence?: number;
  limit?: number;
  offset?: number;
}

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
  relativeDepth: number;
  safetyNotes: string[];
}

export interface LibraryListResponse {
  total: number;
  items: LibraryFileRow[];
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

export interface FileInsights {
  format: string | null;
  resourceSummary: string[];
  scriptNamespaces: string[];
  embeddedNames: string[];
  creatorHints: string[];
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
  creatorLearning: CreatorLearningInfo;
  categoryOverride: CategoryOverrideInfo;
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

export interface OrganizationPreview {
  presetName: string;
  detectedStructure: string;
  totalConsidered: number;
  correctedCount: number;
  reviewCount: number;
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
  firstSeenAt: string;
  lastSeenAt: string;
  updatedAt: string;
  errorMessage: string | null;
  sampleFiles: string[];
  notes: string[];
}

export interface DownloadsInboxOverview {
  totalItems: number;
  readyItems: number;
  needsReviewItems: number;
  appliedItems: number;
  errorItems: number;
  activeFiles: number;
  watchedPath: string | null;
}

export interface DownloadsInboxResponse {
  overview: DownloadsInboxOverview;
  items: DownloadsInboxItem[];
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
