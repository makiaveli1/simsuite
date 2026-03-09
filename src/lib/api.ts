import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  ApplyCategoryAuditResult,
  ApplyCreatorAuditResult,
  ApplyPreviewResult,
  CategoryAuditFile,
  CategoryAuditGroup,
  CategoryAuditQuery,
  CategoryAuditResponse,
  CategoryOverrideInfo,
  CreatorAuditFile,
  CreatorAuditGroup,
  CreatorAuditQuery,
  CreatorAuditResponse,
  CreatorLearningInfo,
  DownloadInboxDetail,
  DownloadsInboxQuery,
  DownloadsInboxResponse,
  DownloadsWatcherStatus,
  DetectedLibraryPaths,
  DuplicateOverview,
  DuplicatePair,
  FileDetail,
  HomeOverview,
  OrganizationPreview,
  LibraryFacets,
  LibraryListResponse,
  LibraryQuery,
  LibrarySettings,
  RestoreSnapshotResult,
  ReviewQueueItem,
  RulePreset,
  ScanProgress,
  ScanStatus,
  ScanSummary,
  SnapshotSummary,
} from "./types";

const hasTauriRuntime =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

const DEFAULT_MODS_PATH =
  "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods";
const DEFAULT_TRAY_PATH =
  "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Tray";
const DEFAULT_DOWNLOADS_PATH = "C:\\Users\\Player\\Downloads";

let mockSettings: LibrarySettings = {
  modsPath: DEFAULT_MODS_PATH,
  trayPath: DEFAULT_TRAY_PATH,
  downloadsPath: DEFAULT_DOWNLOADS_PATH,
};

let mockLastScanAt = "2026-03-08T03:48:00.000Z";
let mockSnapshotId = 12;
let mockScanStatus: ScanStatus = {
  state: "idle",
  mode: null,
  phase: null,
  totalFiles: 0,
  processedFiles: 0,
  currentItem: null,
  startedAt: null,
  finishedAt: null,
  lastSummary: null,
  error: null,
};
const mockProgressListeners = new Set<(progress: ScanProgress) => void>();
const mockStatusListeners = new Set<(status: ScanStatus) => void>();
const mockDownloadsStatusListeners = new Set<
  (status: DownloadsWatcherStatus) => void
>();
let mockDownloadsWatcherStatus: DownloadsWatcherStatus = {
  state: "watching",
  watchedPath: DEFAULT_DOWNLOADS_PATH,
  configured: true,
  currentItem: null,
  lastRunAt: "2026-03-08T04:12:00.000Z",
  lastChangeAt: "2026-03-08T04:11:00.000Z",
  lastError: null,
  readyItems: 2,
  needsReviewItems: 1,
  activeItems: 3,
};
const emptyInsights = {
  format: null,
  resourceSummary: [],
  scriptNamespaces: [],
  embeddedNames: [],
  creatorHints: [],
};
const emptyCreatorLearning = (): CreatorLearningInfo => ({
  lockedByUser: false,
  preferredPath: null,
  learnedAliases: [],
});
const emptyCategoryOverride = (): CategoryOverrideInfo => ({
  savedByUser: false,
  kind: null,
  subtype: null,
});

const mockFiles = ([
  {
    id: 1,
    filename: "AHarris00_CozyKitchen.package",
    path: `${DEFAULT_MODS_PATH}\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package`,
    extension: ".package",
    kind: "BuildBuy",
    subtype: "Kitchen",
    confidence: 0.94,
    sourceLocation: "mods",
    size: 15_728_640,
    modifiedAt: "2026-03-07T21:15:00.000Z",
    creator: "AHarris00",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: [],
    hash: "4d8d0f-kitchen",
    createdAt: "2026-03-06T17:22:00.000Z",
    parserWarnings: [],
    insights: {
      format: "dbpf-package",
      resourceSummary: ["Catalog x6", "Definition x6", "StringTable x1"],
      scriptNamespaces: [],
      embeddedNames: ["Cozy Kitchen Counter", "Cozy Kitchen Shelf"],
      creatorHints: ["Aharris00britney"],
    },
  },
  {
    id: 2,
    filename: "TwistedMexi_BetterExceptions.ts4script",
    path: `${DEFAULT_MODS_PATH}\\TMEX\\TwistedMexi_BetterExceptions.ts4script`,
    extension: ".ts4script",
    kind: "ScriptMods",
    subtype: "Utility",
    confidence: 0.99,
    sourceLocation: "mods",
    size: 482_304,
    modifiedAt: "2026-03-06T09:20:00.000Z",
    creator: "TwistedMexi",
    bundleName: null,
    bundleType: null,
    relativeDepth: 1,
    safetyNotes: [],
    hash: "9cbb3d-tmex",
    createdAt: "2026-03-02T10:00:00.000Z",
    parserWarnings: [],
    insights: {
      format: "ts4script-zip",
      resourceSummary: ["Archive entries: 48", "Top-level namespaces: 2"],
      scriptNamespaces: ["twistedmexi", "better_exceptions"],
      embeddedNames: ["__init__", "scanner", "ui"],
      creatorHints: ["TwistedMexi"],
    },
  },
  {
    id: 3,
    filename: "NorthernSiberiaWinds_Skinblend.package",
    path: `${DEFAULT_MODS_PATH}\\CAS\\Skins\\NorthernSiberiaWinds_Skinblend.package`,
    extension: ".package",
    kind: "CAS",
    subtype: "Skinblend",
    confidence: 0.88,
    sourceLocation: "mods",
    size: 28_835_840,
    modifiedAt: "2026-03-05T16:10:00.000Z",
    creator: "NorthernSiberiaWinds",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: [],
    hash: "cc09d3-skinblend",
    createdAt: "2026-03-01T09:12:00.000Z",
    parserWarnings: [],
    insights: {
      format: "dbpf-package",
      resourceSummary: ["CASPart x2", "NameMap x1"],
      scriptNamespaces: [],
      embeddedNames: ["NSW_Skinblend", "NSW_Eyelids"],
      creatorHints: ["Northern Siberia Winds"],
    },
  },
  {
    id: 4,
    filename: "MCCC_MCCommandCenter.ts4script",
    path: `${DEFAULT_MODS_PATH}\\Gameplay\\Deaderpool\\Core\\MCCC_MCCommandCenter.ts4script`,
    extension: ".ts4script",
    kind: "ScriptMods",
    subtype: "Core",
    confidence: 0.96,
    sourceLocation: "mods",
    size: 1_275_904,
    modifiedAt: "2026-03-07T18:42:00.000Z",
    creator: "Deaderpool",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: ["Script mods should not be deeper than one subfolder."],
    hash: "mccc-core",
    createdAt: "2026-03-03T11:45:00.000Z",
    parserWarnings: ["Depth exceeds safe script placement rule."],
    insights: {
      format: "ts4script-zip",
      resourceSummary: ["Archive entries: 124", "Top-level namespaces: 2"],
      scriptNamespaces: ["deaderpool", "mccc"],
      embeddedNames: ["mc_cmd_center", "mc_settings"],
      creatorHints: ["Deaderpool"],
    },
  },
  {
    id: 5,
    filename: "OakHousehold_0x00ABCDEF.trayitem",
    path: `${DEFAULT_TRAY_PATH}\\OakHousehold_0x00ABCDEF.trayitem`,
    extension: ".trayitem",
    kind: "TrayHousehold",
    subtype: "Household",
    confidence: 0.92,
    sourceLocation: "tray",
    size: 84_992,
    modifiedAt: "2026-03-04T13:14:00.000Z",
    creator: "Oakby",
    bundleName: "OakHousehold",
    bundleType: "household",
    relativeDepth: 0,
    safetyNotes: [],
    hash: "tray-household-1",
    createdAt: "2026-03-04T13:13:00.000Z",
    parserWarnings: [],
    insights: emptyInsights,
  },
  {
    id: 6,
    filename: "LooseBlueprint.blueprint",
    path: `${DEFAULT_MODS_PATH}\\Unsorted\\LooseBlueprint.blueprint`,
    extension: ".blueprint",
    kind: "TrayLot",
    subtype: "Lot",
    confidence: 0.58,
    sourceLocation: "mods",
    size: 64_512,
    modifiedAt: "2026-03-04T15:30:00.000Z",
    creator: null,
    bundleName: "LooseBlueprint",
    bundleType: "lot",
    relativeDepth: 1,
    safetyNotes: ["Tray content detected outside the Tray folder."],
    hash: "tray-lot-unsafe",
    createdAt: "2026-03-04T15:28:00.000Z",
    parserWarnings: ["Filename confidence is low."],
    insights: emptyInsights,
  },
  {
    id: 7,
    filename: "Miiko_Eyebrows.package",
    path: `${DEFAULT_MODS_PATH}\\CAS\\Miiko\\Miiko_Eyebrows.package`,
    extension: ".package",
    kind: "CAS",
    subtype: "Eyebrows",
    confidence: 0.86,
    sourceLocation: "mods",
    size: 2_093_056,
    modifiedAt: "2026-03-07T08:10:00.000Z",
    creator: "Miiko",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: [],
    hash: "miiko-brow-1",
    createdAt: "2026-03-03T14:24:00.000Z",
    parserWarnings: [],
    insights: {
      format: "dbpf-package",
      resourceSummary: ["CASPart x1", "NameMap x1"],
      scriptNamespaces: [],
      embeddedNames: ["Miiko_Brow_03"],
      creatorHints: ["Miiko"],
    },
  },
  {
    id: 8,
    filename: "UnknownCreator_misc.package",
    path: `${DEFAULT_MODS_PATH}\\Downloads\\UnknownCreator_misc.package`,
    extension: ".package",
    kind: "Gameplay",
    subtype: "Misc",
    confidence: 0.46,
    sourceLocation: "downloads",
    size: 7_544_832,
    modifiedAt: "2026-03-08T02:11:00.000Z",
    creator: null,
    bundleName: null,
    bundleType: null,
    relativeDepth: 1,
    safetyNotes: ["Needs manual review before safe placement."],
    hash: "unknown-misc-1",
    createdAt: "2026-03-08T02:10:00.000Z",
    parserWarnings: ["Creator could not be identified."],
    insights: {
      format: "dbpf-package",
      resourceSummary: ["HotSpotControl x1", "NameMap x1"],
      scriptNamespaces: [],
      embeddedNames: ["Height Slider"],
      creatorHints: [],
    },
  },
] as Omit<FileDetail, "creatorLearning" | "categoryOverride">[]).map((file) => ({
  creatorLearning: emptyCreatorLearning(),
  categoryOverride: emptyCategoryOverride(),
  ...file,
}));

const mockReviewQueue: ReviewQueueItem[] = [
  {
    id: 301,
    fileId: 4,
    filename: "MCCC_MCCommandCenter.ts4script",
    path: `${DEFAULT_MODS_PATH}\\Gameplay\\Deaderpool\\Core\\MCCC_MCCommandCenter.ts4script`,
    reason: "unsafe_script_depth",
    confidence: 0.96,
    kind: "ScriptMods",
    subtype: "Core",
    creator: "Deaderpool",
    suggestedPath: "Mods\\Script Mods\\Deaderpool\\MCCC_MCCommandCenter.ts4script",
    safetyNotes: ["Script mods should not be deeper than one subfolder."],
    sourceLocation: "mods",
  },
  {
    id: 302,
    fileId: 6,
    filename: "LooseBlueprint.blueprint",
    path: `${DEFAULT_MODS_PATH}\\Unsorted\\LooseBlueprint.blueprint`,
    reason: "tray_content_in_mods",
    confidence: 0.58,
    kind: "TrayLot",
    subtype: "Lot",
    creator: null,
    suggestedPath: "Tray\\LooseBlueprint.blueprint",
    safetyNotes: ["Tray content detected outside the Tray folder."],
    sourceLocation: "mods",
  },
  {
    id: 303,
    fileId: 8,
    filename: "UnknownCreator_misc.package",
    path: `${DEFAULT_MODS_PATH}\\Downloads\\UnknownCreator_misc.package`,
    reason: "low_confidence_name",
    confidence: 0.46,
    kind: "Gameplay",
    subtype: "Misc",
    creator: null,
    suggestedPath: "Mods\\Gameplay\\Review\\UnknownCreator_misc.package",
    safetyNotes: ["Needs manual review before safe placement."],
    sourceLocation: "downloads",
  },
];

let mockDownloadsItems = [
  {
    id: 41,
    displayName: "SpringRefreshPack.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\SpringRefreshPack.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "partial",
    sourceSize: 42_884_096,
    detectedFileCount: 14,
    activeFileCount: 14,
    appliedFileCount: 0,
    reviewFileCount: 3,
    firstSeenAt: "2026-03-08T04:02:00.000Z",
    lastSeenAt: "2026-03-08T04:05:00.000Z",
    updatedAt: "2026-03-08T04:06:00.000Z",
    errorMessage: null,
    notes: ["Ignored 6 unsupported archive entries."],
    sampleFiles: [
      "CharlyPancakes_SunroomSofa.package",
      "CharlyPancakes_SunroomChair.package",
      "CharlyPancakes_SunroomLamp.package",
    ],
  },
  {
    id: 42,
    displayName: "TwistedMexi_BetterExceptions.ts4script",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\TwistedMexi_BetterExceptions.ts4script`,
    sourceKind: "file",
    archiveFormat: null,
    status: "ready",
    sourceSize: 482_304,
    detectedFileCount: 1,
    activeFileCount: 1,
    appliedFileCount: 0,
    reviewFileCount: 0,
    firstSeenAt: "2026-03-08T03:58:00.000Z",
    lastSeenAt: "2026-03-08T03:58:00.000Z",
    updatedAt: "2026-03-08T03:59:00.000Z",
    errorMessage: null,
    notes: [],
    sampleFiles: ["TwistedMexi_BetterExceptions.ts4script"],
  },
  {
    id: 43,
    displayName: "UnknownCreator_misc.package",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\UnknownCreator_misc.package`,
    sourceKind: "file",
    archiveFormat: null,
    status: "needs_review",
    sourceSize: 7_544_832,
    detectedFileCount: 1,
    activeFileCount: 1,
    appliedFileCount: 0,
    reviewFileCount: 1,
    firstSeenAt: "2026-03-08T02:10:00.000Z",
    lastSeenAt: "2026-03-08T02:10:00.000Z",
    updatedAt: "2026-03-08T02:11:00.000Z",
    errorMessage: null,
    notes: ["Needs a human check before it moves."],
    sampleFiles: ["UnknownCreator_misc.package"],
  },
];

const mockRulePresets: RulePreset[] = [
  {
    name: "Category First",
    template: "Mods/{kind}/{creator}/{filename}",
    priority: 1,
    description: "Sort by category, then creator when recognized.",
  },
  {
    name: "Creator First",
    template: "Mods/{creator}/{kind}/{filename}",
    priority: 2,
    description: "Keep creator work grouped before category.",
  },
  {
    name: "Mirror Mode",
    template: "Mirror current structure when already safe",
    priority: 3,
    description: "Respect the present library shape unless validation intervenes.",
  },
];

let mockSnapshots: SnapshotSummary[] = [
  {
    id: 11,
    snapshotName: "pre_patch_cleanup",
    description: "Safe re-sort after March scan",
    createdAt: "2026-03-07T18:05:00.000Z",
    itemCount: 26,
  },
  {
    id: 10,
    snapshotName: "tray_recovery_pass",
    description: "Tray bundle correction batch",
    createdAt: "2026-03-05T12:44:00.000Z",
    itemCount: 8,
  },
];

const mockSuggestions: OrganizationPreview["suggestions"] = [
  {
    fileId: 1,
    filename: "AHarris00_CozyKitchen.package",
    currentPath: `${DEFAULT_MODS_PATH}\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package`,
    suggestedRelativePath: "Mods\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package",
    suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package`,
    finalRelativePath: "Mods\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package",
    finalAbsolutePath: `${DEFAULT_MODS_PATH}\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package`,
    ruleLabel: "Category First",
    validatorNotes: [],
    reviewRequired: false,
    corrected: false,
    confidence: 0.94,
    kind: "BuildBuy",
    creator: "AHarris00",
    sourceLocation: "mods",
    bundleName: null,
  },
  {
    fileId: 4,
    filename: "MCCC_MCCommandCenter.ts4script",
    currentPath: `${DEFAULT_MODS_PATH}\\Gameplay\\Deaderpool\\Core\\MCCC_MCCommandCenter.ts4script`,
    suggestedRelativePath: "Mods\\ScriptMods\\Deaderpool\\Core\\MCCC_MCCommandCenter.ts4script",
    suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\ScriptMods\\Deaderpool\\Core\\MCCC_MCCommandCenter.ts4script`,
    finalRelativePath: "Mods\\Script Mods\\Deaderpool\\MCCC_MCCommandCenter.ts4script",
    finalAbsolutePath: `${DEFAULT_MODS_PATH}\\Script Mods\\Deaderpool\\MCCC_MCCommandCenter.ts4script`,
    ruleLabel: "Category First",
    validatorNotes: ["Script depth corrected to one subfolder."],
    reviewRequired: false,
    corrected: true,
    confidence: 0.96,
    kind: "ScriptMods",
    creator: "Deaderpool",
    sourceLocation: "mods",
    bundleName: null,
  },
  {
    fileId: 6,
    filename: "LooseBlueprint.blueprint",
    currentPath: `${DEFAULT_MODS_PATH}\\Unsorted\\LooseBlueprint.blueprint`,
    suggestedRelativePath: "Mods\\Tray\\LooseBlueprint.blueprint",
    suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\Tray\\LooseBlueprint.blueprint`,
    finalRelativePath: "Tray\\LooseBlueprint.blueprint",
    finalAbsolutePath: `${DEFAULT_TRAY_PATH}\\LooseBlueprint.blueprint`,
    ruleLabel: "Category First",
    validatorNotes: ["Tray content rerouted to the Tray root."],
    reviewRequired: true,
    corrected: true,
    confidence: 0.58,
    kind: "TrayLot",
    creator: null,
    sourceLocation: "mods",
    bundleName: "LooseBlueprint",
  },
  {
    fileId: 7,
    filename: "Miiko_Eyebrows.package",
    currentPath: `${DEFAULT_MODS_PATH}\\CAS\\Miiko\\Miiko_Eyebrows.package`,
    suggestedRelativePath: "Mods\\CAS\\Miiko\\Miiko_Eyebrows.package",
    suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\CAS\\Miiko\\Miiko_Eyebrows.package`,
    finalRelativePath: "Mods\\CAS\\Miiko\\Miiko_Eyebrows.package",
    finalAbsolutePath: `${DEFAULT_MODS_PATH}\\CAS\\Miiko\\Miiko_Eyebrows.package`,
    ruleLabel: "Category First",
    validatorNotes: [],
    reviewRequired: false,
    corrected: false,
    confidence: 0.86,
    kind: "CAS",
    creator: "Miiko",
    sourceLocation: "mods",
    bundleName: null,
  },
  {
    fileId: 8,
    filename: "UnknownCreator_misc.package",
    currentPath: `${DEFAULT_MODS_PATH}\\Downloads\\UnknownCreator_misc.package`,
    suggestedRelativePath: "Mods\\Gameplay\\Unknown\\UnknownCreator_misc.package",
    suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\Gameplay\\Unknown\\UnknownCreator_misc.package`,
    finalRelativePath: "Mods\\Gameplay\\Review\\UnknownCreator_misc.package",
    finalAbsolutePath: `${DEFAULT_MODS_PATH}\\Gameplay\\Review\\UnknownCreator_misc.package`,
    ruleLabel: "Category First",
    validatorNotes: ["Low confidence classification requires review."],
    reviewRequired: true,
    corrected: true,
    confidence: 0.46,
    kind: "Gameplay",
    creator: null,
    sourceLocation: "downloads",
    bundleName: null,
  },
];

const mockDuplicatePairs: DuplicatePair[] = [
  {
    id: 501,
    duplicateType: "exact",
    detectionMethod: "sha256",
    primaryFileId: 1,
    primaryFilename: "AHarris00_CozyKitchen.package",
    primaryPath: `${DEFAULT_MODS_PATH}\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package`,
    primaryCreator: "AHarris00",
    primaryHash: "4d8d0f-kitchen",
    primaryModifiedAt: "2026-03-07T21:15:00.000Z",
    primarySize: 15_728_640,
    secondaryFileId: 101,
    secondaryFilename: "AHarris00_CozyKitchen Copy.package",
    secondaryPath: `${DEFAULT_MODS_PATH}\\Backups\\AHarris00_CozyKitchen Copy.package`,
    secondaryCreator: "AHarris00",
    secondaryHash: "4d8d0f-kitchen",
    secondaryModifiedAt: "2026-03-07T21:10:00.000Z",
    secondarySize: 15_728_640,
  },
  {
    id: 502,
    duplicateType: "filename",
    detectionMethod: "filename_match",
    primaryFileId: 7,
    primaryFilename: "Miiko_Eyebrows.package",
    primaryPath: `${DEFAULT_MODS_PATH}\\CAS\\Miiko\\Miiko_Eyebrows.package`,
    primaryCreator: "Miiko",
    primaryHash: "miiko-brow-1",
    primaryModifiedAt: "2026-03-07T08:10:00.000Z",
    primarySize: 2_093_056,
    secondaryFileId: 107,
    secondaryFilename: "Miiko_Eyebrows.package",
    secondaryPath: `${DEFAULT_MODS_PATH}\\Downloads\\Miiko_Eyebrows.package`,
    secondaryCreator: "Miiko",
    secondaryHash: "miiko-brow-2",
    secondaryModifiedAt: "2026-02-25T07:40:00.000Z",
    secondarySize: 2_101_248,
  },
  {
    id: 503,
    duplicateType: "version",
    detectionMethod: "version_token_strip",
    primaryFileId: 201,
    primaryFilename: "SnootySims_Counter_v1.package",
    primaryPath: `${DEFAULT_MODS_PATH}\\BuildBuy\\SnootySims\\SnootySims_Counter_v1.package`,
    primaryCreator: "SnootySims",
    primaryHash: "counter-v1",
    primaryModifiedAt: "2026-02-21T10:12:00.000Z",
    primarySize: 5_242_880,
    secondaryFileId: 202,
    secondaryFilename: "SnootySims_Counter_v2.package",
    secondaryPath: `${DEFAULT_MODS_PATH}\\BuildBuy\\SnootySims\\SnootySims_Counter_v2.package`,
    secondaryCreator: "SnootySims",
    secondaryHash: "counter-v2",
    secondaryModifiedAt: "2026-03-02T12:32:00.000Z",
    secondarySize: 5_320_704,
  },
];

let mockCreatorAuditState: {
  totalCandidateFiles: number;
  rootLooseFiles: number;
  groups: CreatorAuditGroup[];
  unresolvedSamples: CreatorAuditFile[];
} = {
  totalCandidateFiles: 1_422,
  rootLooseFiles: 922,
  groups: [
    {
      id: "babyboo",
      suggestedCreator: "BabyBoo",
      confidence: 0.91,
      knownCreator: false,
      itemCount: 38,
      dominantKind: "BuildBuy",
      sourceSignals: ["Filename pattern", "Folder path"],
      aliasSamples: ["BabyBoo"],
      fileIds: [9101, 9102, 9103, 9104, 9105, 9106, 9107, 9108],
      sampleFiles: [
        {
          id: 9101,
          filename: "BabyBooCrib.package",
          path: `${DEFAULT_MODS_PATH}\\BabyBooCrib.package`,
          kind: "BuildBuy",
          subtype: "Furniture",
          confidence: 0.44,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["BabyBoo"],
          matchReasons: ["Filename pattern", "Shared family prefix"],
        },
        {
          id: 9102,
          filename: "BabyBooDresser.package",
          path: `${DEFAULT_MODS_PATH}\\BabyBooDresser.package`,
          kind: "BuildBuy",
          subtype: "Furniture",
          confidence: 0.46,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["BabyBoo"],
          matchReasons: ["Filename pattern", "Shared family prefix"],
        },
        {
          id: 9103,
          filename: "BabyBooBookshelf.package",
          path: `${DEFAULT_MODS_PATH}\\Nursery\\BabyBooBookshelf.package`,
          kind: "BuildBuy",
          subtype: "Furniture",
          confidence: 0.52,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["BabyBoo"],
          matchReasons: ["Filename pattern", "Folder path"],
        },
      ],
    },
    {
      id: "ss",
      suggestedCreator: "SS",
      confidence: 0.87,
      knownCreator: false,
      itemCount: 24,
      dominantKind: "Gameplay",
      sourceSignals: ["Filename pattern"],
      aliasSamples: ["[SS]"],
      fileIds: [9201, 9202, 9203, 9204, 9205],
      sampleFiles: [
        {
          id: 9201,
          filename: "1_[SS] Recipes_Toddler_Porridge_SPA_ES_laura.package",
          path: `${DEFAULT_MODS_PATH}\\Gameplay\\Food\\1_[SS] Recipes_Toddler_Porridge_SPA_ES_laura.package`,
          kind: "Gameplay",
          subtype: "Recipe",
          confidence: 0.51,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["[SS]"],
          matchReasons: ["Bracket tag", "Shared alias"],
        },
        {
          id: 9202,
          filename: "[SS] EA Override Menu.package",
          path: `${DEFAULT_MODS_PATH}\\Overrides\\[SS] EA Override Menu.package`,
          kind: "OverridesAndDefaults",
          subtype: "Menu Override",
          confidence: 0.55,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["[SS]"],
          matchReasons: ["Bracket tag", "Filename pattern"],
        },
      ],
    },
    {
      id: "dogsill",
      suggestedCreator: "dogsill",
      confidence: 0.83,
      knownCreator: false,
      itemCount: 17,
      dominantKind: "CAS",
      sourceSignals: ["Filename pattern", "Inspection metadata"],
      aliasSamples: ["[dogsill]"],
      fileIds: [9301, 9302, 9303, 9304],
      sampleFiles: [
        {
          id: 9301,
          filename: "[dogsill]abigail_hair.package",
          path: `${DEFAULT_MODS_PATH}\\CAS\\Hair\\[dogsill]abigail_hair.package`,
          kind: "CAS",
          subtype: "Hair",
          confidence: 0.62,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["[dogsill]"],
          matchReasons: ["Bracket tag", "Inspection metadata"],
        },
        {
          id: 9302,
          filename: "[dogsill]mila_hair.package",
          path: `${DEFAULT_MODS_PATH}\\CAS\\Hair\\[dogsill]mila_hair.package`,
          kind: "CAS",
          subtype: "Hair",
          confidence: 0.61,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["[dogsill]"],
          matchReasons: ["Bracket tag", "Shared alias"],
        },
      ],
    },
    {
      id: "gabymelove-sims",
      suggestedCreator: "Gabymelove Sims",
      confidence: 0.79,
      knownCreator: false,
      itemCount: 12,
      dominantKind: "CAS",
      sourceSignals: ["Filename pattern"],
      aliasSamples: ["Gabymelove Sims"],
      fileIds: [9401, 9402, 9403],
      sampleFiles: [
        {
          id: 9401,
          filename:
            "[Gabymelove Sims] Aesthetic butterfly tattoo set - Butterfly Garden (Series I).package",
          path: `${DEFAULT_MODS_PATH}\\CAS\\Tattoos\\[Gabymelove Sims] Aesthetic butterfly tattoo set - Butterfly Garden (Series I).package`,
          kind: "CAS",
          subtype: "Tattoo",
          confidence: 0.57,
          sourceLocation: "mods",
          currentCreator: null,
          aliasSamples: ["Gabymelove Sims"],
          matchReasons: ["Bracket tag", "Filename pattern"],
        },
      ],
    },
  ],
  unresolvedSamples: [
    {
      id: 9801,
      filename: "SERAWIS - Alive ( skin undertones - freckles ).package",
      path: `${DEFAULT_MODS_PATH}\\CAS\\Skins\\SERAWIS - Alive ( skin undertones - freckles ).package`,
      kind: "CAS",
      subtype: "Skin Details",
      confidence: 0.41,
      sourceLocation: "mods",
      currentCreator: null,
      aliasSamples: [],
      matchReasons: ["Single weak prefix signal"],
    },
    {
      id: 9802,
      filename: "08eva bottom by LUCKYEIGHT.package",
      path: `${DEFAULT_MODS_PATH}\\CAS\\Bottoms\\08eva bottom by LUCKYEIGHT.package`,
      kind: "CAS",
      subtype: "Bottom",
      confidence: 0.43,
      sourceLocation: "mods",
      currentCreator: null,
      aliasSamples: [],
      matchReasons: ["Byline found but cluster not confirmed"],
    },
    {
      id: 9803,
      filename: "Andirz_SmartCoreScript_v.2.9.0.ts4script",
      path: `${DEFAULT_MODS_PATH}\\Gameplay\\Andirz_SmartCoreScript_v.2.9.0.ts4script`,
      kind: "ScriptMods",
      subtype: "Core",
      confidence: 0.48,
      sourceLocation: "mods",
      currentCreator: null,
      aliasSamples: [],
      matchReasons: ["Single filename prefix only"],
    },
  ],
};

let mockCategoryAuditState: {
  totalCandidateFiles: number;
  unknownFiles: number;
  groups: CategoryAuditGroup[];
  unresolvedSamples: CategoryAuditFile[];
} = {
  totalCandidateFiles: 684,
  unknownFiles: 291,
  groups: [
    {
      id: "cas:hair",
      suggestedKind: "CAS",
      suggestedSubtype: "Hair",
      confidence: 0.9,
      itemCount: 46,
      sourceSignals: ["Filename keywords", "Folder path"],
      keywordSamples: ["hair", "bangs", "ponytail"],
      fileIds: [9601, 9602, 9603, 9604],
      sampleFiles: [
        {
          id: 9601,
          filename: "[dogsill]abigail_hair.package",
          path: `${DEFAULT_MODS_PATH}\\CAS\\Hair\\[dogsill]abigail_hair.package`,
          currentKind: "Unknown",
          currentSubtype: null,
          confidence: 0.44,
          sourceLocation: "mods",
          keywordSamples: ["hair"],
          matchReasons: ["Filename keywords", "Folder path"],
        },
        {
          id: 9602,
          filename: "simstrouble_breezy_hair_v2.package",
          path: `${DEFAULT_MODS_PATH}\\CAS\\Hair\\simstrouble_breezy_hair_v2.package`,
          currentKind: "CAS",
          currentSubtype: "Unknown",
          confidence: 0.59,
          sourceLocation: "mods",
          keywordSamples: ["hair", "breezy"],
          matchReasons: ["Filename keywords"],
        },
      ],
    },
    {
      id: "buildbuy:furniture",
      suggestedKind: "BuildBuy",
      suggestedSubtype: "Furniture",
      confidence: 0.86,
      itemCount: 31,
      sourceSignals: ["Inspection metadata", "Filename keywords"],
      keywordSamples: ["chair", "dresser", "crib", "table"],
      fileIds: [9701, 9702, 9703, 9704],
      sampleFiles: [
        {
          id: 9701,
          filename: "BabyBooCrib.package",
          path: `${DEFAULT_MODS_PATH}\\BuildBuy\\Nursery\\BabyBooCrib.package`,
          currentKind: "Unknown",
          currentSubtype: null,
          confidence: 0.38,
          sourceLocation: "mods",
          keywordSamples: ["crib"],
          matchReasons: ["Filename keywords", "Inspection metadata"],
        },
        {
          id: 9702,
          filename: "BabyBooDresser.package",
          path: `${DEFAULT_MODS_PATH}\\BuildBuy\\Nursery\\BabyBooDresser.package`,
          currentKind: "BuildBuy",
          currentSubtype: null,
          confidence: 0.57,
          sourceLocation: "mods",
          keywordSamples: ["dresser"],
          matchReasons: ["Filename keywords", "Inspection metadata"],
        },
      ],
    },
    {
      id: "overrideanddefaults:defaults",
      suggestedKind: "OverridesAndDefaults",
      suggestedSubtype: "Defaults",
      confidence: 0.84,
      itemCount: 19,
      sourceSignals: ["Filename keywords", "Folder path"],
      keywordSamples: ["override", "default", "replacement"],
      fileIds: [9801, 9802, 9803],
      sampleFiles: [
        {
          id: 9801,
          filename: "[SS] EA Override Menu.package",
          path: `${DEFAULT_MODS_PATH}\\Overrides\\[SS] EA Override Menu.package`,
          currentKind: "Gameplay",
          currentSubtype: "Utility",
          confidence: 0.51,
          sourceLocation: "mods",
          keywordSamples: ["override"],
          matchReasons: ["Filename keywords", "Folder path"],
        },
        {
          id: 9802,
          filename: "lighting_override.package",
          path: `${DEFAULT_MODS_PATH}\\Defaults\\lighting_override.package`,
          currentKind: "Unknown",
          currentSubtype: null,
          confidence: 0.36,
          sourceLocation: "mods",
          keywordSamples: ["lighting", "override"],
          matchReasons: ["Filename keywords", "Folder path"],
        },
      ],
    },
    {
      id: "scriptmods:utilities",
      suggestedKind: "ScriptMods",
      suggestedSubtype: "Utilities",
      confidence: 0.82,
      itemCount: 14,
      sourceSignals: ["Inspection metadata"],
      keywordSamples: ["ts4script", "script"],
      fileIds: [9901, 9902],
      sampleFiles: [
        {
          id: 9901,
          filename: "Andirz_SmartCoreScript_v.2.9.0.ts4script",
          path: `${DEFAULT_MODS_PATH}\\Scripts\\Andirz_SmartCoreScript_v.2.9.0.ts4script`,
          currentKind: "Unknown",
          currentSubtype: null,
          confidence: 0.48,
          sourceLocation: "mods",
          keywordSamples: ["ts4script", "script"],
          matchReasons: ["Inspection metadata"],
        },
      ],
    },
  ],
  unresolvedSamples: [
    {
      id: 9951,
      filename: "SERAWIS - Alive ( skin undertones - freckles ).package",
      path: `${DEFAULT_MODS_PATH}\\CAS\\Skins\\SERAWIS - Alive ( skin undertones - freckles ).package`,
      currentKind: "CAS",
      currentSubtype: "Unknown",
      confidence: 0.41,
      sourceLocation: "mods",
      keywordSamples: ["skin", "freckles"],
      matchReasons: ["Mixed CAS signals"],
    },
    {
      id: 9952,
      filename: "AdrienPastel x Natasha Skirt.package",
      path: `${DEFAULT_MODS_PATH}\\CAS\\AdrienPastel x Natasha Skirt.package`,
      currentKind: "CAS",
      currentSubtype: null,
      confidence: 0.47,
      sourceLocation: "mods",
      keywordSamples: ["skirt"],
      matchReasons: ["One weak clothing token"],
    },
  ],
};

function createMockOverview(): HomeOverview {
  return {
    totalFiles: 1_834,
    modsCount: 1_622,
    trayCount: 84,
    downloadsCount: mockDownloadsItems.reduce(
      (count, item) => count + item.activeFileCount,
      0,
    ),
    scriptModsCount: 43,
    creatorCount: 67,
    bundlesCount: 31,
    duplicatesCount: 18,
    reviewCount: mockReviewQueue.length,
    unsafeCount: mockFiles.filter((file) => file.safetyNotes.length > 0).length,
    lastScanAt: mockLastScanAt,
    readOnlyMode: true,
  };
}

function createMockDownloadsOverview() {
  return {
    totalItems: mockDownloadsItems.length,
    readyItems: mockDownloadsItems.filter((item) =>
      ["ready", "partial"].includes(item.status),
    ).length,
    needsReviewItems: mockDownloadsItems.filter(
      (item) => item.status === "needs_review",
    ).length,
    appliedItems: mockDownloadsItems.filter((item) => item.status === "applied")
      .length,
    errorItems: mockDownloadsItems.filter((item) => item.status === "error").length,
    activeFiles: mockDownloadsItems.reduce(
      (count, item) => count + item.activeFileCount,
      0,
    ),
    watchedPath: mockSettings.downloadsPath,
  };
}

function buildMockDownloadDetail(itemId: number): DownloadInboxDetail | null {
  const item = mockDownloadsItems.find((entry) => entry.id === itemId);
  if (!item) {
    return null;
  }

  if (itemId === 41) {
    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 8,
          filename: "UnknownCreator_misc.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\SpringRefreshPack\\UnknownCreator_misc.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\SpringRefreshPack.zip`,
          archiveMemberPath: "Gameplay/UnknownCreator_misc.package",
          kind: "Gameplay",
          subtype: "Misc",
          creator: null,
          confidence: 0.46,
          size: 7_544_832,
          sourceLocation: "downloads",
          safetyNotes: ["Needs manual review before safe placement."],
        },
        {
          fileId: 1,
          filename: "AHarris00_CozyKitchen.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\SpringRefreshPack\\AHarris00_CozyKitchen.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\SpringRefreshPack.zip`,
          archiveMemberPath: "BuildBuy/AHarris00_CozyKitchen.package",
          kind: "BuildBuy",
          subtype: "Kitchen",
          creator: "AHarris00",
          confidence: 0.94,
          size: 15_728_640,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 7,
          filename: "Miiko_Eyebrows.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\SpringRefreshPack\\Miiko_Eyebrows.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\SpringRefreshPack.zip`,
          archiveMemberPath: "CAS/Miiko_Eyebrows.package",
          kind: "CAS",
          subtype: "Eyebrows",
          creator: "Miiko",
          confidence: 0.86,
          size: 2_093_056,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
      ],
    };
  }

  if (itemId === 42) {
    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 2,
          filename: "TwistedMexi_BetterExceptions.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\TwistedMexi_BetterExceptions.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\TwistedMexi_BetterExceptions.ts4script`,
          archiveMemberPath: null,
          kind: "ScriptMods",
          subtype: "Utility",
          creator: "TwistedMexi",
          confidence: 0.99,
          size: 482_304,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
      ],
    };
  }

  return {
    item: structuredClone(item),
    files: [
      {
        fileId: 8,
        filename: "UnknownCreator_misc.package",
        currentPath: `${DEFAULT_DOWNLOADS_PATH}\\UnknownCreator_misc.package`,
        originPath: `${DEFAULT_DOWNLOADS_PATH}\\UnknownCreator_misc.package`,
        archiveMemberPath: null,
        kind: "Gameplay",
        subtype: "Misc",
        creator: null,
        confidence: 0.46,
        size: 7_544_832,
        sourceLocation: "downloads",
        safetyNotes: ["Needs manual review before safe placement."],
      },
    ],
  };
}

function emitMockDownloadsStatus(status: DownloadsWatcherStatus) {
  mockDownloadsWatcherStatus = status;
  for (const listener of mockDownloadsStatusListeners) {
    listener(status);
  }
}

function emitMockProgress(progress: ScanProgress) {
  for (const listener of mockProgressListeners) {
    listener(progress);
  }
}

function emitMockStatus(status: ScanStatus) {
  mockScanStatus = status;
  for (const listener of mockStatusListeners) {
    listener(status);
  }
}

function queueMockScan() {
  const totalFiles = createMockOverview().totalFiles;
  const startedAt = new Date().toISOString();
  emitMockStatus({
    state: "running",
    mode: "incremental",
    phase: "collecting",
    totalFiles: 0,
    processedFiles: 0,
    currentItem: "Walking configured library folders",
    startedAt,
    finishedAt: null,
    lastSummary: null,
    error: null,
  });

  const progressSteps: Array<{ delay: number; progress: ScanProgress }> = [
    {
      delay: 120,
      progress: {
        phase: "collecting",
        totalFiles: 420,
        processedFiles: 0,
        currentItem: "BuildBuy",
      },
    },
    {
      delay: 260,
      progress: {
        phase: "hashing",
        totalFiles: 34,
        processedFiles: 11,
        currentItem: "AHarris00_CozyKitchen.package",
      },
    },
    {
      delay: 420,
      progress: {
        phase: "classifying",
        totalFiles,
        processedFiles: 950,
        currentItem: "TwistedMexi_BetterExceptions.ts4script",
      },
    },
    {
      delay: 620,
      progress: {
        phase: "duplicates",
        totalFiles,
        processedFiles: totalFiles,
        currentItem: "Rebuilding duplicate map",
      },
    },
  ];

  for (const step of progressSteps) {
    globalThis.setTimeout(() => {
      emitMockProgress(step.progress);
      emitMockStatus({
        ...mockScanStatus,
        state: "running",
        phase: step.progress.phase,
        totalFiles: step.progress.totalFiles,
        processedFiles: step.progress.processedFiles,
        currentItem: step.progress.currentItem,
      });
    }, step.delay);
  }

  globalThis.setTimeout(() => {
    mockLastScanAt = new Date().toISOString();
    const summary: ScanSummary = {
      sessionId: 999,
      scanMode: "incremental",
      filesScanned: totalFiles,
      reusedFiles: 1_502,
      newFiles: 38,
      updatedFiles: 17,
      removedFiles: 4,
      hashedFiles: 34,
      reviewItemsCreated: mockReviewQueue.length,
      bundlesDetected: 31,
      duplicatesDetected: 18,
      errors: [],
    };
    emitMockProgress({
      phase: "done",
      totalFiles,
      processedFiles: totalFiles,
      currentItem: "Scan complete",
    });
    emitMockStatus({
      state: "succeeded",
      mode: "incremental",
      phase: "done",
      totalFiles,
      processedFiles: totalFiles,
      currentItem: "Scan complete",
      startedAt,
      finishedAt: new Date().toISOString(),
      lastSummary: summary,
      error: null,
    });
  }, 860);
}

function filterMockFiles(query: LibraryQuery) {
  const search = query.search?.trim().toLowerCase();
  let items = [...mockFiles];

  if (search) {
    items = items.filter((item) =>
      [item.filename, item.kind, item.subtype, item.creator]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(search)),
    );
  }

  if (query.kind) {
    items = items.filter((item) => item.kind === query.kind);
  }

  if (query.creator) {
    items = items.filter((item) => item.creator === query.creator);
  }

  if (query.source) {
    items = items.filter((item) => item.sourceLocation === query.source);
  }

  const minConfidence = query.minConfidence;
  if (typeof minConfidence === "number") {
    items = items.filter((item) => item.confidence >= minConfidence);
  }

  const total = items.length;
  const offset = query.offset ?? 0;
  const limit = query.limit ?? total;

  return {
    total,
    items: items.slice(offset, offset + limit),
  };
}

function normalizeMockAlias(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "");
}

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values.filter(Boolean)));
}

function deriveMockRelativeParent(path: string, sourceLocation: string) {
  const root = sourceLocation === "tray" ? mockSettings.trayPath : mockSettings.modsPath;
  if (!root) {
    return null;
  }

  const normalizedRoot = root.replace(/\//g, "\\").toLowerCase();
  const normalizedPath = path.replace(/\//g, "\\");
  if (!normalizedPath.toLowerCase().startsWith(normalizedRoot)) {
    return null;
  }

  const relative = normalizedPath.slice(root.length).replace(/^\\+/, "");
  const parent = relative.split("\\").slice(0, -1).join("/");
  return parent || null;
}

function buildMockCreatorAuditResponse(
  query?: CreatorAuditQuery,
): CreatorAuditResponse {
  const search = query?.search?.trim().toLowerCase() ?? "";
  const minGroupSize = Math.max(query?.minGroupSize ?? 2, 1);
  const limit = Math.max(query?.limit ?? 48, 1);
  const groupedFiles = mockCreatorAuditState.groups.reduce(
    (total, group) => total + group.itemCount,
    0,
  );

  let groups = mockCreatorAuditState.groups.filter(
    (group) => group.itemCount >= minGroupSize,
  );

  if (search) {
    groups = groups.filter((group) =>
      [
        group.suggestedCreator,
        ...group.aliasSamples,
        ...group.sampleFiles.map((file) => file.filename),
      ].some((value) => value.toLowerCase().includes(search)),
    );
  }

  const totalGroups = groups.length;
  const highConfidenceGroups = groups.filter(
    (group) => group.confidence >= 0.86,
  ).length;

  return {
    totalCandidateFiles: mockCreatorAuditState.totalCandidateFiles,
    groupedFiles,
    unresolvedFiles: Math.max(
      0,
      mockCreatorAuditState.totalCandidateFiles - groupedFiles,
    ),
    rootLooseFiles: mockCreatorAuditState.rootLooseFiles,
    totalGroups,
    highConfidenceGroups,
    groups: structuredClone(groups.slice(0, limit)),
    unresolvedSamples: structuredClone(mockCreatorAuditState.unresolvedSamples),
  };
}

function buildMockCategoryAuditResponse(
  query?: CategoryAuditQuery,
): CategoryAuditResponse {
  const search = query?.search?.trim().toLowerCase() ?? "";
  const minGroupSize = Math.max(query?.minGroupSize ?? 2, 1);
  const limit = Math.max(query?.limit ?? 48, 1);
  const groupedFiles = mockCategoryAuditState.groups.reduce(
    (total, group) => total + group.itemCount,
    0,
  );

  let groups = mockCategoryAuditState.groups.filter(
    (group) => group.itemCount >= minGroupSize,
  );

  if (search) {
    groups = groups.filter((group) =>
      [
        group.suggestedKind,
        group.suggestedSubtype ?? "",
        ...group.keywordSamples,
        ...group.sampleFiles.map((file) => file.filename),
      ].some((value) => value.toLowerCase().includes(search)),
    );
  }

  const totalGroups = groups.length;
  const highConfidenceGroups = groups.filter(
    (group) => group.confidence >= 0.84,
  ).length;

  return {
    totalCandidateFiles: mockCategoryAuditState.totalCandidateFiles,
    groupedFiles,
    unresolvedFiles: Math.max(
      0,
      mockCategoryAuditState.totalCandidateFiles - groupedFiles,
    ),
    unknownFiles: mockCategoryAuditState.unknownFiles,
    totalGroups,
    highConfidenceGroups,
    groups: structuredClone(groups.slice(0, limit)),
    unresolvedSamples: structuredClone(mockCategoryAuditState.unresolvedSamples),
  };
}

async function mockInvoke<T>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  switch (command) {
    case "get_library_settings":
      return structuredClone(mockSettings) as T;
    case "save_library_paths":
      mockSettings = structuredClone(payload?.settings as LibrarySettings);
      return structuredClone(mockSettings) as T;
    case "detect_default_library_paths":
      return {
        modsPath: DEFAULT_MODS_PATH,
        trayPath: DEFAULT_TRAY_PATH,
        downloadsPath: DEFAULT_DOWNLOADS_PATH,
      } as T;
    case "pick_folder":
      return null as T;
    case "get_home_overview":
      return createMockOverview() as T;
    case "scan_library":
      mockLastScanAt = new Date().toISOString();
      return {
        sessionId: 999,
        scanMode: "full",
        filesScanned: createMockOverview().totalFiles,
        reusedFiles: 0,
        newFiles: createMockOverview().totalFiles,
        updatedFiles: 0,
        removedFiles: 0,
        hashedFiles: 18,
        reviewItemsCreated: mockReviewQueue.length,
        bundlesDetected: 31,
        duplicatesDetected: 18,
        errors: [],
      } as T;
    case "start_scan":
      if (mockScanStatus.state !== "running") {
        queueMockScan();
      }
      return structuredClone(mockScanStatus) as T;
    case "get_scan_status":
      return structuredClone(mockScanStatus) as T;
    case "get_downloads_watcher_status":
      return structuredClone(mockDownloadsWatcherStatus) as T;
    case "refresh_downloads_inbox":
      emitMockDownloadsStatus({
        ...mockDownloadsWatcherStatus,
        state: "processing",
        currentItem: "Manual inbox refresh",
        lastRunAt: new Date().toISOString(),
      });
      globalThis.setTimeout(() => {
        emitMockDownloadsStatus({
          ...mockDownloadsWatcherStatus,
          state: "watching",
          currentItem: null,
          lastRunAt: new Date().toISOString(),
          lastChangeAt: new Date().toISOString(),
        });
      }, 180);
      return structuredClone(mockDownloadsWatcherStatus) as T;
    case "get_downloads_inbox": {
      const query = (payload?.query as DownloadsInboxQuery | undefined) ?? {};
      const search = query.search?.trim().toLowerCase() ?? "";
      const status = query.status?.trim() ?? "";
      const limit = query.limit ?? 120;
      const items = mockDownloadsItems.filter((item) => {
        const matchesSearch =
          !search ||
          [item.displayName, item.sourcePath, ...item.sampleFiles].some((value) =>
            value.toLowerCase().includes(search),
          );
        const matchesStatus = !status || item.status === status;
        return matchesSearch && matchesStatus;
      });

      return {
        overview: createMockDownloadsOverview(),
        items: structuredClone(items.slice(0, limit)),
      } as T;
    }
    case "get_download_item_detail":
      return structuredClone(
        buildMockDownloadDetail(payload?.itemId as number),
      ) as T;
    case "preview_download_item": {
      const itemId = payload?.itemId as number;
      const detail = buildMockDownloadDetail(itemId);
      const suggestions = mockSuggestions.filter((item) =>
        detail?.files.some((file) => file.fileId === item.fileId),
      );
      return {
        presetName: (payload?.presetName as string | undefined) ?? "Category First",
        detectedStructure: "Downloads inbox batch ready for a safe hand-off.",
        totalConsidered: suggestions.length,
        correctedCount: suggestions.filter((item) => item.corrected).length,
        reviewCount: suggestions.filter((item) => item.reviewRequired).length,
        suggestions: structuredClone(suggestions),
      } as T;
    }
    case "get_library_facets":
      return {
        creators: Array.from(
          new Set(mockFiles.map((item) => item.creator).filter(Boolean)),
        ).sort(),
        kinds: Array.from(new Set(mockFiles.map((item) => item.kind))).sort(),
        subtypes: Array.from(
          new Set(mockFiles.map((item) => item.subtype).filter(Boolean)),
        ).sort(),
        sources: Array.from(
          new Set(mockFiles.map((item) => item.sourceLocation)),
        ).sort(),
        taxonomyKinds: [
          "CAS",
          "BuildBuy",
          "Gameplay",
          "ScriptMods",
          "OverridesAndDefaults",
          "PosesAndAnimation",
          "PresetsAndSliders",
          "TrayHousehold",
          "TrayLot",
          "TrayRoom",
          "TrayItem",
          "Unknown",
        ],
      } as T;
    case "get_duplicate_overview":
      return {
        totalPairs: mockDuplicatePairs.length,
        exactPairs: mockDuplicatePairs.filter((item) => item.duplicateType === "exact")
          .length,
        filenamePairs: mockDuplicatePairs.filter(
          (item) => item.duplicateType === "filename",
        ).length,
        versionPairs: mockDuplicatePairs.filter(
          (item) => item.duplicateType === "version",
        ).length,
      } as T;
    case "list_duplicate_pairs":
      return structuredClone(mockDuplicatePairs)
        .filter((item) =>
          payload?.duplicateType
            ? item.duplicateType === payload.duplicateType
            : true,
        )
        .slice(0, (payload?.limit as number | undefined) ?? mockDuplicatePairs.length) as T;
    case "list_rule_presets":
      return structuredClone(mockRulePresets) as T;
    case "preview_organization":
      return {
        presetName: (payload?.presetName as string | undefined) ?? "Category First",
        detectedStructure: "Mixed creator/category tree with script depth issues.",
        totalConsidered: 412,
        correctedCount: mockSuggestions.filter((item) => item.corrected).length,
        reviewCount: mockSuggestions.filter((item) => item.reviewRequired).length,
        suggestions: structuredClone(mockSuggestions).slice(
          0,
          (payload?.limit as number | undefined) ?? mockSuggestions.length,
        ),
      } as T;
    case "get_review_queue":
      return structuredClone(mockReviewQueue).slice(
        0,
        (payload?.limit as number | undefined) ?? mockReviewQueue.length,
      ) as T;
    case "get_creator_audit":
      return buildMockCreatorAuditResponse(
        payload?.query as CreatorAuditQuery | undefined,
      ) as T;
    case "get_category_audit":
      return buildMockCategoryAuditResponse(
        payload?.query as CategoryAuditQuery | undefined,
      ) as T;
    case "get_creator_audit_group_files": {
      const groupId = payload?.groupId as string | undefined;
      const group = mockCreatorAuditState.groups.find((item) => item.id === groupId);
      return structuredClone(group?.sampleFiles ?? []) as T;
    }
    case "get_category_audit_group_files": {
      const groupId = payload?.groupId as string | undefined;
      const group = mockCategoryAuditState.groups.find((item) => item.id === groupId);
      return structuredClone(group?.sampleFiles ?? []) as T;
    }
    case "list_snapshots":
      return structuredClone(mockSnapshots).slice(
        0,
        (payload?.limit as number | undefined) ?? mockSnapshots.length,
      ) as T;
    case "apply_preview_organization": {
      const movedCount = mockSuggestions.filter(
        (item) => !item.reviewRequired && item.finalAbsolutePath !== item.currentPath,
      ).length;
      const snapshotName = `browser_preview_${mockSnapshotId}`;
      mockSnapshots = [
        {
          id: mockSnapshotId,
          snapshotName,
          description: "Browser preview batch",
          createdAt: new Date().toISOString(),
          itemCount: movedCount,
        },
        ...mockSnapshots,
      ];
      mockSnapshotId += 1;

      return {
        snapshotId: mockSnapshotId - 1,
        movedCount,
        deferredReviewCount: mockReviewQueue.length,
        skippedCount: 0,
        snapshotName,
      } as T;
    }
    case "restore_snapshot":
      return {
        snapshotId: payload?.snapshotId as number,
        restoredCount: 5,
        skippedCount: 0,
      } as T;
    case "apply_download_item": {
      const itemId = payload?.itemId as number;
      mockDownloadsItems = mockDownloadsItems.map((item) =>
        item.id === itemId
          ? {
              ...item,
              status: item.reviewFileCount > 0 ? "needs_review" : "applied",
              appliedFileCount: item.detectedFileCount - item.reviewFileCount,
              activeFileCount: item.reviewFileCount,
              updatedAt: new Date().toISOString(),
            }
          : item,
      );
      return {
        snapshotId: mockSnapshotId++,
        movedCount:
          buildMockDownloadDetail(itemId)?.files.filter(
            (file) =>
              !file.safetyNotes.length &&
              file.creator !== null,
          ).length ?? 0,
        deferredReviewCount:
          mockDownloadsItems.find((item) => item.id === itemId)?.reviewFileCount ?? 0,
        skippedCount: 0,
        snapshotName: `downloads_batch_${mockSnapshotId - 1}`,
      } as T;
    }
    case "ignore_download_item": {
      const itemId = payload?.itemId as number;
      mockDownloadsItems = mockDownloadsItems.map((item) =>
        item.id === itemId
          ? {
              ...item,
              status: "ignored",
              activeFileCount: 0,
              updatedAt: new Date().toISOString(),
            }
          : item,
      );
      return true as T;
    }
    case "list_library_files": {
      const query = (payload?.query as LibraryQuery | undefined) ?? {};
      return filterMockFiles(query) as T;
    }
    case "get_file_detail": {
      const fileId = payload?.fileId as number;
      return (
        structuredClone(mockFiles.find((item) => item.id === fileId) ?? null) as T
      );
    }
    case "save_creator_learning": {
      const fileId = payload?.fileId as number;
      const creatorName = String(payload?.creatorName ?? "").trim();
      const aliasInput = String(payload?.aliasName ?? "").trim();
      const lockPreference = Boolean(payload?.lockPreference);
      const preferredPath = String(payload?.preferredPath ?? "").trim();
      const fileIndex = mockFiles.findIndex((item) => item.id === fileId);

      if (fileIndex === -1 || !creatorName) {
        return null as T;
      }

      const next = structuredClone(mockFiles[fileIndex]);
      const normalizedAlias = aliasInput ? normalizeMockAlias(aliasInput) : "";
      const derivedPath = deriveMockRelativeParent(next.path, next.sourceLocation);

      next.creator = creatorName;
      next.confidence = Math.max(next.confidence, 0.92);
      next.parserWarnings = next.parserWarnings.filter(
        (warning) =>
          warning !== "conflicting_creator_signals" &&
          warning !== "Creator could not be identified.",
      );
      next.insights.creatorHints = uniqueStrings([
        creatorName,
        ...next.insights.creatorHints,
      ]);
      next.creatorLearning = {
        lockedByUser: lockPreference || next.creatorLearning.lockedByUser,
        preferredPath:
          lockPreference
            ? preferredPath || next.creatorLearning.preferredPath || derivedPath
            : next.creatorLearning.preferredPath,
        learnedAliases: uniqueStrings([
          ...next.creatorLearning.learnedAliases,
          normalizedAlias,
        ]),
      };

      mockFiles[fileIndex] = next;
      return structuredClone(next) as T;
    }
    case "apply_creator_audit": {
      const creatorName = String(payload?.creatorName ?? "").trim();
      const aliasInput = String(payload?.aliasName ?? "").trim();
      const lockPreference = Boolean(payload?.lockPreference);
      const fileIds = Array.isArray(payload?.fileIds)
        ? (payload?.fileIds as number[])
        : [];

      if (!creatorName || fileIds.length === 0) {
        return {
          creatorName,
          updatedCount: 0,
          clearedReviewCount: 0,
          lockedRoute: lockPreference,
        } as T;
      }

      const idSet = new Set(fileIds);
      let updatedCount = 0;

      mockCreatorAuditState.groups = mockCreatorAuditState.groups
        .map((group) => {
          const remainingIds = group.fileIds.filter((id) => !idSet.has(id));
          const removedCount = group.fileIds.length - remainingIds.length;
          if (removedCount === 0) {
            return group;
          }

          updatedCount += removedCount;
          return {
            ...group,
            itemCount: remainingIds.length,
            fileIds: remainingIds,
            sampleFiles: group.sampleFiles.filter((file) => !idSet.has(file.id)),
          };
        })
        .filter((group) => group.itemCount > 0);

      mockCreatorAuditState.unresolvedSamples =
        mockCreatorAuditState.unresolvedSamples.filter((file) => !idSet.has(file.id));
      mockCreatorAuditState.totalCandidateFiles = Math.max(
        0,
        mockCreatorAuditState.totalCandidateFiles - updatedCount,
      );

      const normalizedAlias = aliasInput ? normalizeMockAlias(aliasInput) : "";
      const clearedReviewCount = mockReviewQueue.filter((item) =>
        idSet.has(item.fileId),
      ).length;

      mockReviewQueue.splice(
        0,
        mockReviewQueue.length,
        ...mockReviewQueue.filter((item) => !idSet.has(item.fileId)),
      );

      for (let index = 0; index < mockFiles.length; index += 1) {
        if (!idSet.has(mockFiles[index].id)) {
          continue;
        }

        const next = structuredClone(mockFiles[index]);
        next.creator = creatorName;
        next.confidence = Math.max(next.confidence, 0.92);
        next.insights.creatorHints = uniqueStrings([
          creatorName,
          ...next.insights.creatorHints,
        ]);
        next.creatorLearning = {
          lockedByUser: lockPreference || next.creatorLearning.lockedByUser,
          preferredPath:
            next.creatorLearning.preferredPath ??
            deriveMockRelativeParent(next.path, next.sourceLocation),
          learnedAliases: uniqueStrings([
            ...next.creatorLearning.learnedAliases,
            normalizedAlias,
          ]),
        };
        mockFiles[index] = next;
      }

      return {
        creatorName,
        updatedCount,
        clearedReviewCount,
        lockedRoute: lockPreference,
      } as T;
    }
    case "apply_category_audit": {
      const kind = String(payload?.kind ?? "").trim();
      const subtypeInput = String(payload?.subtype ?? "").trim();
      const fileIds = Array.isArray(payload?.fileIds)
        ? (payload?.fileIds as number[])
        : [];

      if (!kind || fileIds.length === 0) {
        return {
          kind,
          subtype: subtypeInput || null,
          updatedCount: 0,
          clearedReviewCount: 0,
        } as T;
      }

      const idSet = new Set(fileIds);
      let updatedCount = 0;

      mockCategoryAuditState.groups = mockCategoryAuditState.groups
        .map((group) => {
          const remainingIds = group.fileIds.filter((id) => !idSet.has(id));
          const removedCount = group.fileIds.length - remainingIds.length;
          if (removedCount === 0) {
            return group;
          }

          updatedCount += removedCount;
          return {
            ...group,
            itemCount: remainingIds.length,
            fileIds: remainingIds,
            sampleFiles: group.sampleFiles.filter((file) => !idSet.has(file.id)),
          };
        })
        .filter((group) => group.itemCount > 0);

      mockCategoryAuditState.unresolvedSamples =
        mockCategoryAuditState.unresolvedSamples.filter(
          (file) => !idSet.has(file.id),
        );
      mockCategoryAuditState.totalCandidateFiles = Math.max(
        0,
        mockCategoryAuditState.totalCandidateFiles - updatedCount,
      );
      mockCategoryAuditState.unknownFiles = Math.max(
        0,
        mockCategoryAuditState.unknownFiles - updatedCount,
      );

      const clearedReviewCount = mockReviewQueue.filter((item) =>
        idSet.has(item.fileId),
      ).length;
      mockReviewQueue.splice(
        0,
        mockReviewQueue.length,
        ...mockReviewQueue.filter((item) => !idSet.has(item.fileId)),
      );

      for (let index = 0; index < mockFiles.length; index += 1) {
        if (!idSet.has(mockFiles[index].id)) {
          continue;
        }

        const next = structuredClone(mockFiles[index]);
        next.kind = kind;
        next.subtype = subtypeInput || null;
        next.confidence = Math.max(next.confidence, 0.84);
        next.parserWarnings = next.parserWarnings.filter(
          (warning) =>
            warning !== "no_category_detected" &&
            warning !== "conflicting_category_signals",
        );
        next.categoryOverride = {
          savedByUser: true,
          kind,
          subtype: subtypeInput || null,
        };
        mockFiles[index] = next;
      }

      return {
        kind,
        subtype: subtypeInput || null,
        updatedCount,
        clearedReviewCount,
      } as T;
    }
    case "save_category_override": {
      const fileId = payload?.fileId as number;
      const kind = String(payload?.kind ?? "").trim();
      const subtype = String(payload?.subtype ?? "").trim();
      const fileIndex = mockFiles.findIndex((item) => item.id === fileId);

      if (fileIndex === -1 || !kind) {
        return null as T;
      }

      const next = structuredClone(mockFiles[fileIndex]);
      next.kind = kind;
      next.subtype = subtype || null;
      next.confidence = Math.max(next.confidence, 0.82);
      next.parserWarnings = next.parserWarnings.filter(
        (warning) => warning !== "no_category_detected",
      );
      next.categoryOverride = {
        savedByUser: true,
        kind,
        subtype: subtype || null,
      };

      mockFiles[fileIndex] = next;
      return structuredClone(next) as T;
    }
    default:
      throw new Error(`Mock API does not implement '${command}'.`);
  }
}

function invoke<T>(command: string, payload?: Record<string, unknown>) {
  return hasTauriRuntime
    ? tauriInvoke<T>(command, payload)
    : mockInvoke<T>(command, payload);
}

function listenToScanProgress(handler: (progress: ScanProgress) => void) {
  if (hasTauriRuntime) {
    return tauriListen<ScanProgress>("scan-progress", (event) =>
      handler(event.payload),
    );
  }

  mockProgressListeners.add(handler);
  return Promise.resolve(() => {
    mockProgressListeners.delete(handler);
  });
}

function listenToScanStatus(handler: (status: ScanStatus) => void) {
  if (hasTauriRuntime) {
    return tauriListen<ScanStatus>("scan-status", (event) => handler(event.payload));
  }

  mockStatusListeners.add(handler);
  return Promise.resolve(() => {
    mockStatusListeners.delete(handler);
  });
}

function listenToDownloadsStatus(
  handler: (status: DownloadsWatcherStatus) => void,
) {
  if (hasTauriRuntime) {
    return tauriListen<DownloadsWatcherStatus>("downloads-status", (event) =>
      handler(event.payload),
    );
  }

  mockDownloadsStatusListeners.add(handler);
  return Promise.resolve(() => {
    mockDownloadsStatusListeners.delete(handler);
  });
}

export const api = {
  getLibrarySettings: () => invoke<LibrarySettings>("get_library_settings"),
  saveLibraryPaths: (settings: LibrarySettings) =>
    invoke<LibrarySettings>("save_library_paths", { settings }),
  detectDefaultLibraryPaths: () =>
    invoke<DetectedLibraryPaths>("detect_default_library_paths"),
  pickFolder: (title?: string) =>
    invoke<string | null>("pick_folder", { title }),
  getHomeOverview: () => invoke<HomeOverview>("get_home_overview"),
  scanLibrary: () => invoke<ScanSummary>("scan_library"),
  startScan: () => invoke<ScanStatus>("start_scan"),
  getScanStatus: () => invoke<ScanStatus>("get_scan_status"),
  getDownloadsWatcherStatus: () =>
    invoke<DownloadsWatcherStatus>("get_downloads_watcher_status"),
  refreshDownloadsInbox: () =>
    invoke<DownloadsWatcherStatus>("refresh_downloads_inbox"),
  listenToScanProgress,
  listenToScanStatus,
  listenToDownloadsStatus,
  getDownloadsInbox: (query?: DownloadsInboxQuery) =>
    invoke<DownloadsInboxResponse>("get_downloads_inbox", { query }),
  getDownloadItemDetail: (itemId: number) =>
    invoke<DownloadInboxDetail | null>("get_download_item_detail", { itemId }),
  previewDownloadItem: (itemId: number, presetName?: string) =>
    invoke<OrganizationPreview>("preview_download_item", { itemId, presetName }),
  getLibraryFacets: () => invoke<LibraryFacets>("get_library_facets"),
  getDuplicateOverview: () => invoke<DuplicateOverview>("get_duplicate_overview"),
  listDuplicatePairs: (duplicateType?: string, limit?: number) =>
    invoke<DuplicatePair[]>("list_duplicate_pairs", { duplicateType, limit }),
  listRulePresets: () => invoke<RulePreset[]>("list_rule_presets"),
  previewOrganization: (presetName?: string, limit?: number) =>
    invoke<OrganizationPreview>("preview_organization", { presetName, limit }),
  getReviewQueue: (presetName?: string, limit?: number) =>
    invoke<ReviewQueueItem[]>("get_review_queue", { presetName, limit }),
  getCreatorAudit: (query?: CreatorAuditQuery) =>
    invoke<CreatorAuditResponse>("get_creator_audit", { query }),
  getCategoryAudit: (query?: CategoryAuditQuery) =>
    invoke<CategoryAuditResponse>("get_category_audit", { query }),
  getCreatorAuditGroupFiles: (groupId: string) =>
    invoke<CreatorAuditFile[]>("get_creator_audit_group_files", { groupId }),
  getCategoryAuditGroupFiles: (groupId: string) =>
    invoke<CategoryAuditFile[]>("get_category_audit_group_files", { groupId }),
  listSnapshots: (limit?: number) =>
    invoke<SnapshotSummary[]>("list_snapshots", { limit }),
  applyPreviewOrganization: (
    presetName?: string,
    limit?: number,
    approved = false,
  ) =>
    invoke<ApplyPreviewResult>("apply_preview_organization", {
      presetName,
      limit,
      approved,
    }),
  restoreSnapshot: (snapshotId: number, approved = false) =>
    invoke<RestoreSnapshotResult>("restore_snapshot", { snapshotId, approved }),
  applyDownloadItem: (
    itemId: number,
    presetName?: string,
    approved = false,
  ) =>
    invoke<ApplyPreviewResult>("apply_download_item", {
      itemId,
      presetName,
      approved,
    }),
  ignoreDownloadItem: (itemId: number) =>
    invoke<boolean>("ignore_download_item", { itemId }),
  listLibraryFiles: (query: LibraryQuery) =>
    invoke<LibraryListResponse>("list_library_files", { query }),
  getFileDetail: (fileId: number) =>
    invoke<FileDetail | null>("get_file_detail", { fileId }),
  saveCategoryOverride: (fileId: number, kind: string, subtype?: string) =>
    invoke<FileDetail | null>("save_category_override", {
      fileId,
      kind,
      subtype,
    }),
  saveCreatorLearning: (
    fileId: number,
    creatorName: string,
    aliasName?: string,
    lockPreference?: boolean,
    preferredPath?: string,
  ) =>
    invoke<FileDetail | null>("save_creator_learning", {
      fileId,
      creatorName,
      aliasName,
      lockPreference,
      preferredPath,
    }),
  applyCreatorAudit: (
    fileIds: number[],
    creatorName: string,
    aliasName?: string,
    lockPreference?: boolean,
    preferredPath?: string,
  ) =>
    invoke<ApplyCreatorAuditResult>("apply_creator_audit", {
      fileIds,
      creatorName,
      aliasName,
      lockPreference,
      preferredPath,
    }),
  applyCategoryAudit: (fileIds: number[], kind: string, subtype?: string) =>
    invoke<ApplyCategoryAuditResult>("apply_category_audit", {
      fileIds,
      kind,
      subtype,
    }),
};
