import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type {
  ApplyReviewPlanActionResult,
  ApplyCategoryAuditResult,
  ApplyCreatorAuditResult,
  ApplyGuidedDownloadResult,
  ApplyPreviewResult,
  ApplySpecialReviewFixResult,
  CatalogSourceInfo,
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
  DependencyStatus,
  DownloadInboxDetail,
  DownloadsInboxItem,
  DownloadsInboxQuery,
  DownloadsInboxResponse,
  DownloadsWatcherStatus,
  DetectedLibraryPaths,
  DuplicateOverview,
  DuplicatePair,
  FileDetail,
  GuidedInstallPlan,
  HomeOverview,
  OrganizationPreview,
  LibraryFacets,
  LibraryListResponse,
  LibraryQuery,
  LibrarySettings,
  RestoreSnapshotResult,
  ReviewPlanAction,
  ReviewQueueItem,
  RulePreset,
  ScanProgress,
  ScanStatus,
  ScanSummary,
  SpecialReviewPlan,
  SnapshotSummary,
} from "./types";

export const hasTauriRuntime =
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
const buildMockCatalogSource = (
  officialSourceUrl: string | null,
  officialDownloadUrl: string | null,
  referenceSource: string[],
  reviewedAt: string | null,
  latestCheckUrl: string | null = officialSourceUrl,
  latestCheckStrategy: string | null = null,
): CatalogSourceInfo => ({
  officialSourceUrl,
  officialDownloadUrl,
  latestCheckUrl,
  latestCheckStrategy,
  referenceSource,
  reviewedAt,
});

const buildMockDependencyStatus = (
  key: string,
  displayName: string,
  status: string,
  summary: string,
): DependencyStatus => ({
  key,
  displayName,
  status,
  summary,
  inboxItemId: null,
  inboxItemName: null,
  inboxItemIntakeMode: null,
  inboxItemGuidedInstallAvailable: false,
});

const mockMcccCatalogSource = buildMockCatalogSource(
  "https://deaderpool-mccc.com/downloads.html",
  "https://drive.google.com/uc?export=download&id=1J_yt-tu8vvHGPI2B6VMoMEjudoSfmHQt",
  ["official_docs", "mod_hound_reference"],
  "2026-03-11",
);

const mockXmlInjectorCatalogSource = buildMockCatalogSource(
  "https://www.curseforge.com/sims4/mods/xml-injector",
  null,
  ["official_docs", "mod_hound_reference"],
  "2026-03-09",
);

const buildMockReviewAction = (
  kind: ReviewPlanAction["kind"],
  label: string,
  description: string,
  priority: number,
  relatedItemId: number | null = null,
  relatedItemName: string | null = null,
  url: string | null = null,
): ReviewPlanAction => ({
  kind,
  label,
  description,
  priority,
  relatedItemId,
  relatedItemName,
  url,
});

function syncMockDownloadsWatcherStatus() {
  const activeItems = mockDownloadsItems.filter((item) => item.status !== "ignored").length;
  const needsReviewItems = mockDownloadsItems.filter((item) =>
    ["needs_review", "error"].includes(item.status),
  ).length;
  const readyItems = mockDownloadsItems.filter((item) =>
    ["ready", "partial"].includes(item.status),
  ).length;

  mockDownloadsWatcherStatus = {
    ...mockDownloadsWatcherStatus,
    activeItems,
    needsReviewItems,
    readyItems,
    lastRunAt: new Date().toISOString(),
  };
}

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

let mockDownloadsItems: DownloadsInboxItem[] = [
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
    intakeMode: "standard",
    riskLevel: "low",
    matchedProfileKey: null,
    matchedProfileName: null,
    specialFamily: null,
    assessmentReasons: ["No special setup rules were matched."],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [],
    evidenceSummary: ["This looks like a normal download, so it can use the standard safe hand-off preview."],
    catalogSource: null,
    existingInstallDetected: false,
    guidedInstallAvailable: false,
    sampleFiles: [
      "CharlyPancakes_SunroomSofa.package",
      "CharlyPancakes_SunroomChair.package",
      "CharlyPancakes_SunroomLamp.package",
    ],
  },
  {
    id: 42,
    displayName: "MC_Command_Center_2026.3.0.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_2026.3.0.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "ready",
    sourceSize: 5_282_304,
    detectedFileCount: 4,
    activeFileCount: 4,
    appliedFileCount: 0,
    reviewFileCount: 0,
    firstSeenAt: "2026-03-08T03:58:00.000Z",
    lastSeenAt: "2026-03-08T03:58:00.000Z",
    updatedAt: "2026-03-08T03:59:00.000Z",
    errorMessage: null,
    notes: ["Install notes were read from the archive."],
    intakeMode: "guided",
    riskLevel: "medium",
    matchedProfileKey: "mccc",
    matchedProfileName: "MC Command Center",
    specialFamily: "Script suite",
    assessmentReasons: [
      "Download name matches the MC Command Center profile.",
      "4 supported files match the MC Command Center file pattern.",
      "Readme or text notes mention MC Command Center clues.",
      "Existing MC Command Center package/script files were found and can be updated safely.",
    ],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Keep the MCCC script and package files together in one shallow Mods folder.",
      "Restart The Sims 4 after updating script mods.",
    ],
    evidenceSummary: [
      "SimSuite found a complete MCCC set and a safe update path.",
      "An older MCCC install is already present and can be replaced without touching settings files.",
    ],
    catalogSource: mockMcccCatalogSource,
    existingInstallDetected: true,
    guidedInstallAvailable: true,
    sampleFiles: [
      "mc_cmd_center.ts4script",
      "mc_cmd_center.package",
      "mc_woohoo.package",
    ],
  },
  {
    id: 43,
    displayName: "MC_Command_Center_partial.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_partial.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "needs_review",
    sourceSize: 1_544_832,
    detectedFileCount: 1,
    activeFileCount: 1,
    appliedFileCount: 0,
    reviewFileCount: 1,
    firstSeenAt: "2026-03-08T02:10:00.000Z",
    lastSeenAt: "2026-03-08T02:10:00.000Z",
    updatedAt: "2026-03-08T02:11:00.000Z",
    errorMessage: null,
    notes: ["SimSuite stopped because the MCCC core script is missing from this download."],
    intakeMode: "blocked",
    riskLevel: "high",
    matchedProfileKey: "mccc",
    matchedProfileName: "MC Command Center",
    specialFamily: "Script suite",
    assessmentReasons: [
      "SimSuite found MC Command Center clues, but the required core script file was not found.",
    ],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "MCCC needs its core script file before SimSuite can install it safely.",
    ],
    evidenceSummary: [
      "This looks like part of an MCCC set, but the core script file is missing.",
    ],
    catalogSource: mockMcccCatalogSource,
    existingInstallDetected: false,
    guidedInstallAvailable: false,
    sampleFiles: ["mc_woohoo.package"],
  },
  {
    id: 44,
    displayName: "MCCC_mixed_folder.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "needs_review",
    sourceSize: 6_144_000,
    detectedFileCount: 6,
    activeFileCount: 6,
    appliedFileCount: 0,
    reviewFileCount: 6,
    firstSeenAt: "2026-03-08T01:45:00.000Z",
    lastSeenAt: "2026-03-08T01:45:00.000Z",
    updatedAt: "2026-03-08T01:46:00.000Z",
    errorMessage: "The download mixes a full MCCC set with extra script files that do not belong to it.",
    notes: ["SimSuite can split the clean MCCC files out before anything moves."],
    intakeMode: "blocked",
    riskLevel: "high",
    matchedProfileKey: "mccc",
    matchedProfileName: "MC Command Center",
    specialFamily: "Script suite",
    assessmentReasons: [
      "A full MCCC set was found inside this download.",
      "Extra script files that do not belong to MCCC were also found in the same archive.",
      "SimSuite stopped so it can split the clean supported set from the extra files first.",
    ],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Split mixed special-mod archives into clean sets before installing them.",
    ],
    evidenceSummary: [
      "SimSuite found a usable MCCC set mixed with unrelated files, so it queued a safe split action.",
    ],
    catalogSource: mockMcccCatalogSource,
    existingInstallDetected: false,
    guidedInstallAvailable: false,
    sampleFiles: [
      "mc_cmd_center.ts4script",
      "mc_cmd_center.package",
      "mc_woohoo.package",
    ],
  },
  {
    id: 45,
    displayName: "Adeepindigo_Healthcare_Redux_Addon.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\Adeepindigo_Healthcare_Redux_Addon.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "needs_review",
    sourceSize: 2_412_544,
    detectedFileCount: 2,
    activeFileCount: 2,
    appliedFileCount: 0,
    reviewFileCount: 2,
    firstSeenAt: "2026-03-09T09:10:00.000Z",
    lastSeenAt: "2026-03-09T09:10:00.000Z",
    updatedAt: "2026-03-09T09:12:00.000Z",
    errorMessage: null,
    notes: ["A required support library is already waiting in the Inbox."],
    intakeMode: "needs_review",
    riskLevel: "medium",
    matchedProfileKey: null,
    matchedProfileName: null,
    specialFamily: "Dependency check",
    assessmentReasons: [
      "This download mentions XML Injector as a required library.",
      "XML Injector is also waiting in the Inbox.",
    ],
    dependencySummary: ["XML Injector is also waiting in the Inbox."],
    missingDependencies: [],
    inboxDependencies: ["XML Injector"],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Install XML Injector before moving mods that depend on it.",
    ],
    evidenceSummary: [
      "SimSuite found a required-library note and matched XML Injector in the Inbox.",
    ],
    catalogSource: mockXmlInjectorCatalogSource,
    existingInstallDetected: false,
    guidedInstallAvailable: false,
    sampleFiles: ["HealthcareRedux_Addon.package", "README_requires_xml_injector.txt"],
  },
  {
    id: 46,
    displayName: "XML_Injector_v4.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\XML_Injector_v4.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "ready",
    sourceSize: 1_128_448,
    detectedFileCount: 2,
    activeFileCount: 2,
    appliedFileCount: 0,
    reviewFileCount: 0,
    firstSeenAt: "2026-03-09T09:08:00.000Z",
    lastSeenAt: "2026-03-09T09:08:00.000Z",
    updatedAt: "2026-03-09T09:09:00.000Z",
    errorMessage: null,
    notes: ["Required library detected and ready for guided setup."],
    intakeMode: "guided",
    riskLevel: "medium",
    matchedProfileKey: "xml_injector",
    matchedProfileName: "XML Injector",
    specialFamily: "Support library",
    assessmentReasons: [
      "Download name matches the XML Injector profile.",
      "The core XML Injector script was found.",
    ],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Keep one current XML Injector install in a shallow Mods folder.",
    ],
    evidenceSummary: [
      "SimSuite found a full XML Injector set and a safe install path.",
    ],
    catalogSource: mockXmlInjectorCatalogSource,
    existingInstallDetected: false,
    guidedInstallAvailable: true,
    sampleFiles: ["XmlInjector_Script_v4.ts4script", "XMLInjector_Test.package"],
  },
  {
    id: 47,
    displayName: "McCmdCenter_AllModules_2026_1_1.zip",
    sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\McCmdCenter_AllModules_2026_1_1.zip`,
    sourceKind: "archive",
    archiveFormat: "zip",
    status: "needs_review",
    sourceSize: 5_644_288,
    detectedFileCount: 14,
    activeFileCount: 14,
    appliedFileCount: 0,
    reviewFileCount: 14,
    firstSeenAt: "2026-03-10T11:24:00.000Z",
    lastSeenAt: "2026-03-10T11:24:00.000Z",
    updatedAt: "2026-03-10T11:30:00.000Z",
    errorMessage: "Older MC Command Center files are spread around Mods and need a safe repair first.",
    notes: [
      "Rechecked with newer SimSuite rules on Mar 10, 2026.",
      "SimSuite found a safe repair path for the older MC Command Center setup.",
    ],
    intakeMode: "blocked",
    riskLevel: "high",
    matchedProfileKey: "mccc",
    matchedProfileName: "MC Command Center",
    specialFamily: "Script suite",
    assessmentReasons: [
      "The new archive looks like a full MC Command Center set.",
      "Older MC Command Center files were found loose in Mods instead of one safe folder.",
      "SimSuite can fix that layout before the update runs.",
    ],
    dependencySummary: [],
    missingDependencies: [],
    inboxDependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Keep every MCCC script and package file together in one shallow MCCC folder.",
      "Keep .cfg settings files when you update MCCC.",
    ],
    evidenceSummary: [
      "SimSuite found a valid MC Command Center update and a repairable old setup.",
    ],
    catalogSource: mockMcccCatalogSource,
    existingInstallDetected: true,
    guidedInstallAvailable: false,
    sampleFiles: [
      "mc_cmd_center.ts4script",
      "mc_cmd_center.package",
      "mc_settings.cfg",
    ],
  },
];

const mockRulePresets: RulePreset[] = [
  {
    name: "Category First",
    template: "{kind}/{subtype}/{creator}",
    priority: 100,
    description: "Puts content type first, then narrows down by subtype and creator.",
  },
  {
    name: "Mirror Mode",
    template: "keep-current-safe-shape",
    priority: 150,
    description: "Keeps safe folders that already make sense and only fixes risky paths.",
  },
  {
    name: "Creator First",
    template: "Creators/{creator}/{kind}/{subtype}",
    priority: 200,
    description: "Keeps each creator together before splitting by content type.",
  },
  {
    name: "Hybrid",
    template: "{kind}/{creator}/{subtype}",
    priority: 300,
    description: "Balances type-first sorting with creator grouping underneath.",
  },
  {
    name: "Minimal Safe",
    template: "ScriptMods/{creator}",
    priority: 400,
    description: "Uses a conservative layout that focuses on safe folder depth first.",
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
    validatorNotes: ["validator_flattened_script_depth"],
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
    validatorNotes: [
      "tray_file_will_be_relocated_from_mods",
      "validator_routed_tray_content_to_tray_root",
    ],
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
    validatorNotes: ["low_confidence_requires_review"],
    reviewRequired: true,
    corrected: true,
    confidence: 0.46,
    kind: "Gameplay",
    creator: null,
    sourceLocation: "downloads",
    bundleName: null,
  },
];

function previewStateFromSuggestion(
  suggestion: OrganizationPreview["suggestions"][number],
) {
  if (suggestion.reviewRequired) {
    return "review";
  }

  if (suggestion.finalAbsolutePath === suggestion.currentPath) {
    return "aligned";
  }

  return "safe";
}

function buildMockPreviewIssueSummary(
  suggestions: OrganizationPreview["suggestions"],
): OrganizationPreview["issueSummary"] {
  const counts = new Map<string, number>();

  for (const suggestion of suggestions) {
    const seen = new Set<string>();
    for (const note of suggestion.validatorNotes) {
      if (!seen.has(note)) {
        seen.add(note);
        counts.set(note, (counts.get(note) ?? 0) + 1);
      }
    }
  }

  const priority = [
    "low_confidence_requires_review",
    "unknown_kind_requires_review",
    "existing_path_collision_detected",
    "preview_path_collision_detected",
    "tray_file_will_be_relocated_from_mods",
    "validator_routed_tray_content_to_tray_root",
    "validator_flattened_script_depth",
    "validator_limited_package_depth",
    "missing_target_root",
  ];

  const summary: OrganizationPreview["issueSummary"] = [];
  for (const code of priority) {
    const count = counts.get(code);
    if (!count) {
      continue;
    }
    summary.push(mockPreviewIssueEntry(code, count));
    counts.delete(code);
  }

  for (const [code, count] of counts) {
    summary.push(mockPreviewIssueEntry(code, count));
  }

  return summary;
}

function mockPreviewIssueEntry(code: string, count: number) {
  const labels: Record<string, { label: string; tone: string }> = {
    low_confidence_requires_review: {
      label: "Names or types still look uncertain",
      tone: "review",
    },
    unknown_kind_requires_review: {
      label: "Some files still have an unknown type",
      tone: "review",
    },
    existing_path_collision_detected: {
      label: "A destination is already occupied",
      tone: "review",
    },
    preview_path_collision_detected: {
      label: "Two files want the same destination",
      tone: "review",
    },
    tray_file_will_be_relocated_from_mods: {
      label: "Tray files were found inside Mods",
      tone: "warn",
    },
    validator_routed_tray_content_to_tray_root: {
      label: "Tray files were rerouted back to Tray",
      tone: "warn",
    },
    validator_flattened_script_depth: {
      label: "Script mods were flattened to a safe depth",
      tone: "warn",
    },
    validator_limited_package_depth: {
      label: "Deep folder paths were shortened",
      tone: "warn",
    },
    missing_target_root: {
      label: "A required root folder is missing",
      tone: "review",
    },
  };

  return {
    code,
    label: labels[code]?.label ?? "Extra checks were raised in this pass",
    tone: labels[code]?.tone ?? "neutral",
    count,
  };
}

function buildMockOrganizationPreview({
  presetName,
  suggestions,
  totalConsidered,
  detectedStructure,
  recommendedPreset,
  recommendedReason,
}: {
  presetName: string;
  suggestions: OrganizationPreview["suggestions"];
  totalConsidered?: number;
  detectedStructure: string;
  recommendedPreset: string;
  recommendedReason: string;
}): OrganizationPreview {
  return {
    presetName,
    detectedStructure,
    totalConsidered: totalConsidered ?? suggestions.length,
    safeCount: suggestions.filter(
      (item) => previewStateFromSuggestion(item) === "safe",
    ).length,
    alignedCount: suggestions.filter(
      (item) => previewStateFromSuggestion(item) === "aligned",
    ).length,
    correctedCount: suggestions.filter((item) => item.corrected).length,
    reviewCount: suggestions.filter((item) => item.reviewRequired).length,
    recommendedPreset,
    recommendedReason,
    issueSummary: buildMockPreviewIssueSummary(suggestions),
    suggestions: structuredClone(suggestions),
  };
}

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
          fileId: 4201,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center\\mc_cmd_center.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_2026.3.0.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.99,
          size: 1_275_904,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4202,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center\\mc_cmd_center.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_2026.3.0.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.97,
          size: 1_942_528,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4203,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center\\mc_woohoo.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_2026.3.0.zip`,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          confidence: 0.97,
          size: 1_183_744,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4204,
          filename: "mc_tuner.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center\\mc_tuner.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_2026.3.0.zip`,
          archiveMemberPath: "MCCC/mc_tuner.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          confidence: 0.95,
          size: 879_488,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
      ],
    };
  }

  if (itemId === 43) {
    if (item.intakeMode === "guided") {
      return {
        item: structuredClone(item),
        files: [
          {
            fileId: 4301,
            filename: "mc_cmd_center.ts4script",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center_partial\\mc_cmd_center.ts4script`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_partial.zip`,
            archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            confidence: 0.99,
            size: 1_275_904,
            sourceLocation: "downloads",
            safetyNotes: [],
          },
          {
            fileId: 4302,
            filename: "mc_cmd_center.package",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center_partial\\mc_cmd_center.package`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_partial.zip`,
            archiveMemberPath: "MCCC/mc_cmd_center.package",
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            confidence: 0.97,
            size: 1_942_528,
            sourceLocation: "downloads",
            safetyNotes: [],
          },
          {
            fileId: 4303,
            filename: "mc_woohoo.package",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center_partial\\mc_woohoo.package`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_partial.zip`,
            archiveMemberPath: "MCCC/mc_woohoo.package",
            kind: "ScriptMods",
            subtype: "Module",
            creator: "Deaderpool",
            confidence: 0.96,
            size: 1_183_744,
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
          fileId: 4301,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center_partial\\mc_woohoo.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MC_Command_Center_partial.zip`,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          confidence: 0.83,
          size: 1_183_744,
          sourceLocation: "downloads",
          safetyNotes: [
            "Special setup needs review because the core script file is missing.",
          ],
        },
      ],
    };
  }

  if (itemId === 44) {
    if (item.intakeMode === "guided") {
      return {
        item: structuredClone(item),
        files: [
          {
            fileId: 4401,
            filename: "mc_cmd_center.ts4script",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder_clean\\mc_cmd_center.ts4script`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
            archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            confidence: 0.95,
            size: 1_275_904,
            sourceLocation: "downloads",
            safetyNotes: [],
          },
          {
            fileId: 4403,
            filename: "mc_cmd_center.package",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder_clean\\mc_cmd_center.package`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
            archiveMemberPath: "MCCC/mc_cmd_center.package",
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            confidence: 0.96,
            size: 1_942_528,
            sourceLocation: "downloads",
            safetyNotes: [],
          },
          {
            fileId: 4404,
            filename: "mc_woohoo.package",
            currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder_clean\\mc_woohoo.package`,
            originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
            archiveMemberPath: "MCCC/mc_woohoo.package",
            kind: "ScriptMods",
            subtype: "Module",
            creator: "Deaderpool",
            confidence: 0.94,
            size: 1_183_744,
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
          fileId: 4401,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_cmd_center.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.95,
          size: 1_275_904,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4403,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_cmd_center.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.96,
          size: 1_942_528,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4404,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_woohoo.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          confidence: 0.94,
          size: 1_183_744,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4402,
          filename: "othermod.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\othermod.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "Other/othermod.ts4script",
          kind: "ScriptMods",
          subtype: "Utility",
          creator: null,
          confidence: 0.51,
          size: 544_128,
          sourceLocation: "downloads",
          safetyNotes: [
            "This file does not fit the guided MC Command Center install set.",
          ],
        },
        {
          fileId: 4405,
          filename: "notes.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\notes.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "Bonus/notes.package",
          kind: "Gameplay",
          subtype: "Utility",
          creator: null,
          confidence: 0.42,
          size: 64_512,
          sourceLocation: "downloads",
          safetyNotes: [
            "This extra file should stay out of the clean MCCC batch.",
          ],
        },
      ],
    };
  }

  if (itemId === 45) {
    const dependencyResolved = item.intakeMode === "standard";

    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 4501,
          filename: "HealthcareRedux_Addon.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\Healthcare_Redux_Addon\\HealthcareRedux_Addon.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\Adeepindigo_Healthcare_Redux_Addon.zip`,
          archiveMemberPath: "HealthcareRedux/HealthcareRedux_Addon.package",
          kind: "Gameplay",
          subtype: "Utility",
          creator: "adeepindigo",
          confidence: 0.9,
          size: 1_842_112,
          sourceLocation: "downloads",
          safetyNotes: dependencyResolved
            ? ["XML Injector was installed first, so this file can use the normal safe hand-off."]
            : ["This batch is waiting on XML Injector before it can move safely."],
        },
        {
          fileId: 4502,
          filename: "README_requires_xml_injector.txt",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\Healthcare_Redux_Addon\\README_requires_xml_injector.txt`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\Adeepindigo_Healthcare_Redux_Addon.zip`,
          archiveMemberPath: "HealthcareRedux/README_requires_xml_injector.txt",
          kind: "Gameplay",
          subtype: "Readme",
          creator: "adeepindigo",
          confidence: 0.72,
          size: 4_096,
          sourceLocation: "downloads",
          safetyNotes: ["This note says XML Injector is required."],
        },
      ],
    };
  }

  if (itemId === 46) {
    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 4601,
          filename: "XmlInjector_Script_v4.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\XML_Injector\\XmlInjector_Script_v4.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\XML_Injector_v4.zip`,
          archiveMemberPath: "XML Injector/XmlInjector_Script_v4.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Scumbumbo / Triplis",
          confidence: 0.98,
          size: 684_032,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
        {
          fileId: 4602,
          filename: "XMLInjector_Test.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\XML_Injector\\XMLInjector_Test.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\XML_Injector_v4.zip`,
          archiveMemberPath: "XML Injector/XMLInjector_Test.package",
          kind: "ScriptMods",
          subtype: "Support",
          creator: "Scumbumbo / Triplis",
          confidence: 0.91,
          size: 442_368,
          sourceLocation: "downloads",
          safetyNotes: [],
        },
      ],
    };
  }

  if (itemId === 47) {
    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 4701,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\McCmdCenter_AllModules_2026_1_1.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.99,
          size: 1_275_904,
          sourceLocation: "downloads",
          safetyNotes: ["This is the core MCCC script file."],
        },
        {
          fileId: 4702,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\McCmdCenter_AllModules_2026_1_1.zip`,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          confidence: 0.97,
          size: 1_942_528,
          sourceLocation: "downloads",
          safetyNotes: ["This is part of the main MCCC suite."],
        },
        {
          fileId: 4703,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_woohoo.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\McCmdCenter_AllModules_2026_1_1.zip`,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          confidence: 0.97,
          size: 1_183_744,
          sourceLocation: "downloads",
          safetyNotes: ["This module should stay with the main MCCC files."],
        },
        {
          fileId: 4704,
          filename: "mc_settings.cfg",
          currentPath: `${DEFAULT_MODS_PATH}\\mc_settings.cfg`,
          originPath: `${DEFAULT_MODS_PATH}\\mc_settings.cfg`,
          archiveMemberPath: null,
          kind: "ScriptMods",
          subtype: "Settings",
          creator: "Deaderpool",
          confidence: 0.94,
          size: 12_288,
          sourceLocation: "mods",
          safetyNotes: ["This settings file should be kept during the repair."],
        },
      ],
    };
  }

  if (itemId === 48) {
    return {
      item: structuredClone(item),
      files: [
        {
          fileId: 4801,
          filename: "othermod.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder_extras\\othermod.ts4script`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "Other/othermod.ts4script",
          kind: "ScriptMods",
          subtype: "Utility",
          creator: null,
          confidence: 0.51,
          size: 544_128,
          sourceLocation: "downloads",
          safetyNotes: ["This leftover file still needs its own review."],
        },
        {
          fileId: 4802,
          filename: "notes.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder_extras\\notes.package`,
          originPath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          archiveMemberPath: "Bonus/notes.package",
          kind: "Gameplay",
          subtype: "Utility",
          creator: null,
          confidence: 0.42,
          size: 64_512,
          sourceLocation: "downloads",
          safetyNotes: ["This extra package was kept separate from the clean MCCC batch."],
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

function buildMockGuidedPlan(itemId: number): GuidedInstallPlan | null {
  const item = mockDownloadsItems.find((entry) => entry.id === itemId);
  if (!item) {
    return null;
  }

  if (itemId === 46) {
    return {
      itemId: 46,
      profileKey: "xml_injector",
      profileName: "XML Injector",
      specialFamily: "Support library",
      installTargetFolder: `${DEFAULT_MODS_PATH}\\XML Injector`,
      installFiles: [
        {
          fileId: 4601,
          filename: "XmlInjector_Script_v4.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\XML_Injector\\XmlInjector_Script_v4.ts4script`,
          targetPath: `${DEFAULT_MODS_PATH}\\XML Injector\\XmlInjector_Script_v4.ts4script`,
          archiveMemberPath: "XML Injector/XmlInjector_Script_v4.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Scumbumbo / Triplis",
          notes: [],
        },
        {
          fileId: 4602,
          filename: "XMLInjector_Test.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\XML_Injector\\XMLInjector_Test.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\XML Injector\\XMLInjector_Test.package`,
          archiveMemberPath: "XML Injector/XMLInjector_Test.package",
          kind: "ScriptMods",
          subtype: "Support",
          creator: "Scumbumbo / Triplis",
          notes: [],
        },
      ],
      replaceFiles: [],
      preserveFiles: [],
      reviewFiles: [],
      dependencies: [],
      incompatibilityWarnings: [],
      postInstallNotes: [
        "Keep one current XML Injector install in one shallow folder.",
        "After the library is installed, refresh any waiting mods that depend on it.",
      ],
      existingLayoutFindings: [],
      warnings: [],
      explanation:
        "XML Injector is a support library, so SimSuite keeps it together in one shallow Mods folder and installs it before dependent mods.",
      evidence: [
        "Download name matches the XML Injector profile.",
        "The XML Injector script file was found.",
      ],
      catalogSource: mockXmlInjectorCatalogSource,
      existingInstallDetected: false,
      applyReady: true,
    };
  }

  if (
    itemId !== 42 &&
    itemId !== 43 &&
    itemId !== 44 &&
    itemId !== 47
  ) {
    return null;
  }

  if (item.intakeMode !== "guided") {
    return null;
  }

  const isRepairFollowUp = itemId === 47;
  const isFreshInstall = itemId === 43 || itemId === 44;
  const baseInboxFolder = isRepairFollowUp
    ? "McCmdCenter_AllModules"
    : itemId === 43
      ? "MC_Command_Center_partial"
      : itemId === 44
        ? "MCCC_mixed_folder_clean"
        : "MC_Command_Center";
  const baseOriginName = isRepairFollowUp
    ? "McCmdCenter_AllModules_2026_1_1.zip"
    : itemId === 43
      ? "MC_Command_Center_partial.zip"
      : itemId === 44
        ? "MCCC_mixed_folder.zip"
        : "MC_Command_Center_2026.3.0.zip";

  return {
    itemId,
    profileKey: "mccc",
    profileName: "MC Command Center",
    specialFamily: "Script suite",
    installTargetFolder: `${DEFAULT_MODS_PATH}\\MCCC`,
    installFiles: [
      {
        fileId: isRepairFollowUp ? 4701 : itemId === 43 ? 4301 : 4401,
        filename: "mc_cmd_center.ts4script",
        currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\${baseInboxFolder}\\mc_cmd_center.ts4script`,
        targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.ts4script`,
        archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
        kind: "ScriptMods",
        subtype: "Core",
        creator: "Deaderpool",
        notes: [],
      },
      {
        fileId: isRepairFollowUp ? 4702 : itemId === 43 ? 4302 : 4403,
        filename: "mc_cmd_center.package",
        currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\${baseInboxFolder}\\mc_cmd_center.package`,
        targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
        archiveMemberPath: "MCCC/mc_cmd_center.package",
        kind: "ScriptMods",
        subtype: "Core",
        creator: "Deaderpool",
        notes: [],
      },
      {
        fileId: isRepairFollowUp ? 4703 : itemId === 43 ? 4303 : 4404,
        filename: "mc_woohoo.package",
        currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\${baseInboxFolder}\\mc_woohoo.package`,
        targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_woohoo.package`,
        archiveMemberPath: "MCCC/mc_woohoo.package",
        kind: "ScriptMods",
        subtype: "Module",
        creator: "Deaderpool",
        notes: [],
      },
    ],
    replaceFiles: isFreshInstall
      ? []
      : [
          {
            fileId: 44021,
            filename: "mc_cmd_center.ts4script",
            currentPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.ts4script`,
            targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.ts4script`,
            archiveMemberPath: null,
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            notes: ["Old MC Command Center package or script file that will be replaced."],
          },
          {
            fileId: 44022,
            filename: "mc_cmd_center.package",
            currentPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
            targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
            archiveMemberPath: null,
            kind: "ScriptMods",
            subtype: "Core",
            creator: "Deaderpool",
            notes: ["Old MC Command Center package or script file that will be replaced."],
          },
          {
            fileId: 44023,
            filename: "mc_woohoo.package",
            currentPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_woohoo.package`,
            targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_woohoo.package`,
            archiveMemberPath: null,
            kind: "ScriptMods",
            subtype: "Module",
            creator: "Deaderpool",
            notes: ["Old MC Command Center package or script file that will be replaced."],
          },
        ],
    preserveFiles: isFreshInstall
      ? []
      : [
          {
            fileId: null,
            filename: "mc_settings.cfg",
            currentPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_settings.cfg`,
            targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_settings.cfg`,
            archiveMemberPath: null,
            kind: "Config",
            subtype: null,
            creator: "Deaderpool",
            notes: ["Settings file that will be kept during the update."],
          },
        ],
    reviewFiles: [],
    dependencies: [],
    incompatibilityWarnings: [],
    postInstallNotes: [
      "Keep every MCCC package and script file together in the same folder.",
      "Restart The Sims 4 after updating script mods.",
    ],
    existingLayoutFindings: isFreshInstall
      ? []
      : [
          "A matching MCCC install was found in Mods\\MCCC.",
          "Settings files already in that folder will stay in place.",
        ],
    warnings: isFreshInstall
      ? []
      : ["Existing MC Command Center settings files will stay in place."],
    explanation:
      "MC Command Center is a script suite, so SimSuite keeps the main script and module files together in one safe folder under Mods.",
    evidence: [
      isRepairFollowUp
        ? "The repaired batch still matches the MC Command Center profile."
        : isFreshInstall
          ? `The staged ${baseOriginName} batch now matches the MC Command Center profile.`
          : "Download name matches the MC Command Center profile.",
      isRepairFollowUp
        ? "The repaired install now points to one safe MCCC folder."
        : isFreshInstall
          ? "The clean staged batch now contains the supported core-and-module MCCC files."
          : "4 supported files match the MC Command Center file pattern.",
      isFreshInstall
        ? "This batch is ready to install as a clean special-mod set."
        : "Existing MC Command Center package/script files were found and will be replaced.",
    ],
    catalogSource: mockMcccCatalogSource,
    existingInstallDetected: !isFreshInstall,
    applyReady: true,
  };
}

function buildMockReviewPlan(itemId: number): SpecialReviewPlan | null {
  if (itemId === 45) {
    return {
      itemId: 45,
      mode: "needs_review",
      profileKey: null,
      profileName: null,
      specialFamily: "Dependency check",
      explanation:
        "This download looks safe, but it says XML Injector is required first. SimSuite found XML Injector in the Inbox and can set it up before checking this download again.",
      recommendedNextStep:
        "Install XML Injector first, then let SimSuite re-check this download.",
      dependencies: [
        {
          key: "xml_injector",
          displayName: "XML Injector",
          status: "inbox",
          summary:
            "XML Injector is in the Inbox and ready for safe setup as XML_Injector_v4.zip.",
          inboxItemId: 46,
          inboxItemName: "XML_Injector_v4.zip",
          inboxItemIntakeMode: "guided",
          inboxItemGuidedInstallAvailable: true,
        },
      ],
      incompatibilityWarnings: [],
      reviewFiles: [
        {
          fileId: 4501,
          filename: "HealthcareRedux_Addon.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\Healthcare_Redux_Addon\\HealthcareRedux_Addon.package`,
          targetPath: null,
          archiveMemberPath: "HealthcareRedux/HealthcareRedux_Addon.package",
          kind: "Gameplay",
          subtype: "Utility",
          creator: "adeepindigo",
          notes: ["This mod is waiting for XML Injector."],
        },
      ],
      evidence: [
        "The included readme says XML Injector is required.",
        "SimSuite matched an XML Injector archive already waiting in the Inbox.",
      ],
      existingLayoutFindings: [],
      postInstallNotes: [
        "After XML Injector is installed, refresh this batch so SimSuite can continue.",
      ],
      catalogSource: mockXmlInjectorCatalogSource,
      availableActions: [
        buildMockReviewAction(
          "install_dependency",
          "Install XML Injector first",
          "SimSuite can set up XML Injector from the Inbox, then re-check this waiting mod.",
          100,
          46,
          "XML Injector",
        ),
      ],
      repairPlanAvailable: false,
      repairActionLabel: null,
      repairReason: null,
      repairTargetFolder: null,
      repairMoveFiles: [],
      repairReplaceFiles: [],
      repairKeepFiles: [],
      repairWarnings: [],
      repairCanContinueInstall: false,
    };
  }

  if (itemId === 43) {
    return {
      itemId: 43,
      mode: "blocked",
      profileKey: "mccc",
      profileName: "MC Command Center",
      specialFamily: "Script suite",
      explanation:
        "This looks like part of an MC Command Center update, but the core script file is missing, so SimSuite will not guess how to install it.",
      recommendedNextStep:
        "Download the missing official MCCC files, then let SimSuite re-check the full set.",
      dependencies: [],
      incompatibilityWarnings: [],
      reviewFiles: [
        {
          fileId: 4301,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MC_Command_Center_partial\\mc_woohoo.package`,
          targetPath: null,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          notes: ["The core script file is missing from this set."],
        },
      ],
      evidence: [
        "The archive contains MC Command Center module files.",
        "The required core script file mc_cmd_center.ts4script was not found.",
      ],
      existingLayoutFindings: [],
      postInstallNotes: [
        "MCCC needs its core script file before SimSuite can install it safely.",
      ],
      catalogSource: mockMcccCatalogSource,
      availableActions: [
        buildMockReviewAction(
          "download_missing_files",
          "Download missing MCCC files",
          "SimSuite can grab the trusted official MCCC archive, stage it in the Inbox, and re-check the full set.",
          100,
          null,
          "MC Command Center",
          mockMcccCatalogSource.officialDownloadUrl,
        ),
      ],
      repairPlanAvailable: false,
      repairActionLabel: null,
      repairReason: null,
      repairTargetFolder: null,
      repairMoveFiles: [],
      repairReplaceFiles: [],
      repairKeepFiles: [],
      repairWarnings: [],
      repairCanContinueInstall: false,
    };
  }

  if (itemId === 44) {
    return {
      itemId: 44,
      mode: "blocked",
      profileKey: "mccc",
      profileName: "MC Command Center",
      specialFamily: "Script suite",
      explanation:
        "This archive mixes MC Command Center files with unrelated script content, so SimSuite blocked it instead of moving a risky batch.",
      recommendedNextStep:
        "Let SimSuite split the clean MCCC set away from the extra files, then continue with the MCCC recipe.",
      dependencies: [],
      incompatibilityWarnings: [
        "Mixed special-mod archives can overwrite the wrong files if they are installed together.",
      ],
      reviewFiles: [
        {
          fileId: 4401,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_cmd_center.ts4script`,
          targetPath: null,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["This file looks like MCCC."],
        },
        {
          fileId: 4403,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_cmd_center.package`,
          targetPath: null,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["This file belongs with the main MCCC set."],
        },
        {
          fileId: 4404,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\mc_woohoo.package`,
          targetPath: null,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          notes: ["This module can stay with the clean MCCC set after the split."],
        },
        {
          fileId: 4402,
          filename: "othermod.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\othermod.ts4script`,
          targetPath: null,
          archiveMemberPath: "Other/othermod.ts4script",
          kind: "ScriptMods",
          subtype: "Utility",
          creator: null,
          notes: ["This file does not belong to the MCCC suite."],
        },
        {
          fileId: 4405,
          filename: "notes.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\MCCC_mixed_folder\\notes.package`,
          targetPath: null,
          archiveMemberPath: "Bonus/notes.package",
          kind: "Gameplay",
          subtype: "Utility",
          creator: null,
          notes: ["This extra file should stay behind for its own review."],
        },
      ],
      evidence: [
        "The archive contains a complete MCCC core-and-module set.",
        "The archive also contains unrelated script content.",
        "SimSuite can split the supported MCCC files into a clean batch first.",
      ],
      existingLayoutFindings: [],
      postInstallNotes: [
        "Keep special script suites in separate clean downloads whenever possible.",
      ],
      catalogSource: mockMcccCatalogSource,
      availableActions: [
        buildMockReviewAction(
          "separate_supported_files",
          "Separate the MCCC files",
          "SimSuite can pull the clean MCCC files into their own batch and leave the extras behind for review.",
          100,
          null,
          "MC Command Center",
        ),
      ],
      repairPlanAvailable: false,
      repairActionLabel: null,
      repairReason: null,
      repairTargetFolder: null,
      repairMoveFiles: [],
      repairReplaceFiles: [],
      repairKeepFiles: [],
      repairWarnings: [],
      repairCanContinueInstall: false,
    };
  }

  if (itemId === 47) {
    return {
      itemId: 47,
      mode: "blocked",
      profileKey: "mccc",
      profileName: "MC Command Center",
      specialFamily: "Script suite",
      explanation:
        "SimSuite found a full MC Command Center update, but your older MCCC files are still spread around Mods. The safe fix is to clear that old setup out of the way before the update runs.",
      recommendedNextStep:
        "Fix the old MCCC setup first, then let SimSuite finish the update in the same safe run.",
      dependencies: [],
      incompatibilityWarnings: [],
      reviewFiles: [
        {
          fileId: 4701,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.ts4script`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.ts4script`,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["The new core script is ready, but the older MCCC layout needs a quick tidy first."],
        },
        {
          fileId: 4702,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["This will replace the older main MCCC package after the repair."],
        },
      ],
      evidence: [
        "The download name and staged files match the MC Command Center profile.",
        "The required core script file mc_cmd_center.ts4script was found in the new download.",
        "Older MCCC files were found loose in Mods instead of one shallow MCCC folder.",
      ],
      existingLayoutFindings: [
        "Older MCCC files were found in the Mods root.",
        "A matching mc_settings.cfg settings file was found and can be kept.",
      ],
      postInstallNotes: [
        "MCCC works best when the script and package files stay together in one MCCC folder.",
        "Your .cfg settings file will stay in place during the repair.",
      ],
      catalogSource: mockMcccCatalogSource,
      availableActions: [
        buildMockReviewAction(
          "repair_special",
          "Fix old MCCC setup",
          "SimSuite can move the older MCCC files out of the way, keep your settings, and finish the update.",
          100,
          null,
          "MC Command Center",
        ),
      ],
      repairPlanAvailable: true,
      repairActionLabel: "Fix old MCCC setup",
      repairReason:
        "SimSuite can move the older MCCC files out of the way, keep your settings, and then finish the update.",
      repairTargetFolder: `${DEFAULT_MODS_PATH}\\MCCC`,
      repairMoveFiles: [
        {
          fileId: 47041,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_MODS_PATH}\\mc_cmd_center.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
          archiveMemberPath: null,
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["The older package will be moved out of the way before replacement."],
        },
        {
          fileId: 47042,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_MODS_PATH}\\mc_woohoo.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_woohoo.package`,
          archiveMemberPath: null,
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          notes: ["This older module belongs in the same MCCC folder."],
        },
      ],
      repairReplaceFiles: [
        {
          fileId: 4701,
          filename: "mc_cmd_center.ts4script",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.ts4script`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.ts4script`,
          archiveMemberPath: "MCCC/mc_cmd_center.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["The new core script will replace the older one."],
        },
        {
          fileId: 4702,
          filename: "mc_cmd_center.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_cmd_center.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_cmd_center.package`,
          archiveMemberPath: "MCCC/mc_cmd_center.package",
          kind: "ScriptMods",
          subtype: "Core",
          creator: "Deaderpool",
          notes: ["The new main package will replace the older one."],
        },
        {
          fileId: 4703,
          filename: "mc_woohoo.package",
          currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\McCmdCenter_AllModules\\mc_woohoo.package`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_woohoo.package`,
          archiveMemberPath: "MCCC/mc_woohoo.package",
          kind: "ScriptMods",
          subtype: "Module",
          creator: "Deaderpool",
          notes: ["The new module will replace the older one."],
        },
      ],
      repairKeepFiles: [
        {
          fileId: 47043,
          filename: "mc_settings.cfg",
          currentPath: `${DEFAULT_MODS_PATH}\\mc_settings.cfg`,
          targetPath: `${DEFAULT_MODS_PATH}\\MCCC\\mc_settings.cfg`,
          archiveMemberPath: null,
          kind: "ScriptMods",
          subtype: "Settings",
          creator: "Deaderpool",
          notes: ["Settings stay safe and move into the repaired MCCC folder."],
        },
      ],
      repairWarnings: [
        "Only the files that clearly belong to MC Command Center are part of this repair.",
      ],
      repairCanContinueInstall: true,
    };
  }

  return null;
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
      syncMockDownloadsWatcherStatus();
      return structuredClone(mockDownloadsWatcherStatus) as T;
    case "refresh_downloads_inbox":
      syncMockDownloadsWatcherStatus();
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
      if (detail?.item.intakeMode !== "standard") {
        throw new Error(
          "This inbox item needs a guided special setup flow instead of the normal hand-off preview.",
        );
      }
      const suggestions =
        itemId === 45
          ? [
              {
                fileId: 4501,
                filename: "HealthcareRedux_Addon.package",
                currentPath: `${DEFAULT_DOWNLOADS_PATH}\\Inbox\\Healthcare_Redux_Addon\\HealthcareRedux_Addon.package`,
                suggestedRelativePath:
                  "Mods\\Gameplay\\Utility\\adeepindigo\\HealthcareRedux_Addon.package",
                suggestedAbsolutePath: `${DEFAULT_MODS_PATH}\\Gameplay\\Utility\\adeepindigo\\HealthcareRedux_Addon.package`,
                finalRelativePath:
                  "Mods\\Gameplay\\Utility\\adeepindigo\\HealthcareRedux_Addon.package",
                finalAbsolutePath: `${DEFAULT_MODS_PATH}\\Gameplay\\Utility\\adeepindigo\\HealthcareRedux_Addon.package`,
                ruleLabel:
                  (payload?.presetName as string | undefined) ?? "Category First",
                validatorNotes: [],
                reviewRequired: false,
                corrected: false,
                confidence: 0.9,
                kind: "Gameplay",
                creator: "adeepindigo",
                sourceLocation: "downloads",
                bundleName: null,
              },
            ]
          : mockSuggestions.filter((item) =>
              detail?.files.some((file) => file.fileId === item.fileId),
            );
      return buildMockOrganizationPreview({
        presetName: (payload?.presetName as string | undefined) ?? "Category First",
        detectedStructure:
          itemId === 45
            ? "Dependency resolved. This batch can now use the normal safe hand-off."
            : "Downloads inbox batch ready for a safe hand-off.",
        totalConsidered: suggestions.length,
        recommendedPreset: "Minimal Safe",
        recommendedReason:
          "Start with the safest cleanup style when files are arriving from Downloads.",
        suggestions,
      }) as T;
    }
    case "get_download_item_guided_plan":
      return structuredClone(
        buildMockGuidedPlan(payload?.itemId as number),
      ) as T;
    case "get_download_item_review_plan":
      return structuredClone(
        buildMockReviewPlan(payload?.itemId as number),
      ) as T;
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
      const previewLimit = payload?.limit as number | undefined;
      return buildMockOrganizationPreview({
        presetName: (payload?.presetName as string | undefined) ?? "Category First",
        detectedStructure:
          "Current library looks mixed, so a conservative pass is safest.",
        totalConsidered: 412,
        recommendedPreset: "Minimal Safe",
        recommendedReason:
          "Minimal Safe is the easiest first cleanup when the current folder shape is inconsistent.",
        suggestions:
          previewLimit != null && previewLimit > 0
            ? structuredClone(mockSuggestions).slice(0, previewLimit)
            : structuredClone(mockSuggestions),
      }) as T;
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
      const selectedItem = mockDownloadsItems.find((item) => item.id === itemId);
      if (selectedItem?.intakeMode !== "standard") {
        throw new Error(
          "This inbox item uses a special setup flow. Open its guided preview instead.",
        );
      }
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
      syncMockDownloadsWatcherStatus();
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
    case "apply_guided_download_item": {
      const itemId = payload?.itemId as number;
      const plan = buildMockGuidedPlan(itemId);
      if (!plan) {
        throw new Error(
          "This inbox item does not have a guided special setup plan.",
        );
      }

      const snapshotName = `guided_${plan.profileKey}_${mockSnapshotId}`;
      mockSnapshots = [
        {
          id: mockSnapshotId,
          snapshotName,
          description: `Guided ${plan.profileName} install`,
          createdAt: new Date().toISOString(),
          itemCount: plan.installFiles.length + plan.replaceFiles.length,
        },
        ...mockSnapshots,
      ];
        mockDownloadsItems = mockDownloadsItems.map((item) =>
          item.id === itemId
            ? {
                ...item,
                status: "applied",
              activeFileCount: 0,
              appliedFileCount: plan.installFiles.length,
              reviewFileCount: plan.reviewFiles.length,
              updatedAt: new Date().toISOString(),
              }
            : item,
        );

        if (itemId === 46) {
          mockDownloadsItems = mockDownloadsItems.map((item) =>
            item.id === 45
              ? {
                  ...item,
                  status: "ready",
                  intakeMode: "standard",
                  riskLevel: "low",
                  matchedProfileKey: null,
                  matchedProfileName: null,
                  specialFamily: null,
                  assessmentReasons: ["XML Injector was installed first, so this batch can use the normal safe hand-off."],
                  dependencySummary: [],
                  missingDependencies: [],
                  inboxDependencies: [],
                  reviewFileCount: 0,
                  notes: ["Dependency resolved. This batch is ready for a normal safe hand-off."],
                  postInstallNotes: [],
                  evidenceSummary: ["The required XML Injector library was installed from the Inbox."],
                  catalogSource: null,
                  updatedAt: new Date().toISOString(),
                }
              : item,
          );
        }

        syncMockDownloadsWatcherStatus();
        mockSnapshotId += 1;
      return {
        snapshotId: mockSnapshotId - 1,
        installedCount: plan.installFiles.length,
        replacedCount: plan.replaceFiles.length,
        preservedCount: plan.preserveFiles.length,
        deferredReviewCount: plan.reviewFiles.length,
        snapshotName,
      } as T;
    }
    case "apply_special_review_fix": {
      const itemId = payload?.itemId as number;
      const plan = buildMockReviewPlan(itemId);
      if (!plan?.repairPlanAvailable) {
        throw new Error("This inbox item does not have a safe repair plan.");
      }

      const repairedCount = plan.repairMoveFiles.length;
      const installedCount = plan.repairReplaceFiles.length;
      const preservedCount = plan.repairKeepFiles.length;
      const snapshotName = `repair_${plan.profileKey ?? "special"}_${mockSnapshotId}`;

      mockSnapshots = [
        {
          id: mockSnapshotId,
          snapshotName,
          description: `Repair ${plan.profileName ?? "special mod"} layout`,
          createdAt: new Date().toISOString(),
          itemCount: repairedCount + installedCount + preservedCount,
        },
        ...mockSnapshots,
      ];

      mockDownloadsItems = mockDownloadsItems.map((item) =>
        item.id === itemId
          ? {
              ...item,
              intakeMode: "guided",
              status: "ready",
              riskLevel: "medium",
              reviewFileCount: 0,
              activeFileCount: 3,
              detectedFileCount: 3,
              errorMessage: null,
              notes: [
                "Rechecked with newer SimSuite rules on Mar 10, 2026.",
                "Older MC Command Center files were cleared out of the way safely.",
                "This batch is now ready for guided special setup.",
              ],
              updatedAt: new Date().toISOString(),
            }
          : item,
      );

      syncMockDownloadsWatcherStatus();
      mockSnapshotId += 1;
      return {
        snapshotId: mockSnapshotId - 1,
        repairedCount,
        installedCount,
        replacedCount: installedCount,
        preservedCount,
        deferredReviewCount: 0,
        snapshotName,
      } as T;
    }
    case "apply_review_plan_action": {
      const itemId = payload?.itemId as number;
      const actionKind = payload?.actionKind as ReviewPlanAction["kind"];
      const relatedItemId = (payload?.relatedItemId as number | null | undefined) ?? null;
      const url = (payload?.url as string | null | undefined) ?? null;
      const approved = Boolean(payload?.approved);
      const reviewPlan = buildMockReviewPlan(itemId);
      const action = reviewPlan?.availableActions.find(
        (candidate) =>
          candidate.kind === actionKind &&
          candidate.relatedItemId === relatedItemId &&
          (url == null || candidate.url === url),
      );

      if (!reviewPlan || !action) {
        throw new Error("This review action is no longer available for the selected inbox item.");
      }

      if (action.kind === "repair_special") {
        if (!approved) {
          throw new Error("Repair was blocked because approval was not confirmed.");
        }
        const result = await mockInvoke<ApplySpecialReviewFixResult>(
          "apply_special_review_fix",
          { itemId, approved: true },
        );
        return {
          actionKind: action.kind,
          focusItemId: itemId,
          createdItemId: null,
          openedUrl: null,
          snapshotId: result.snapshotId,
          repairedCount: result.repairedCount,
          installedCount: result.installedCount,
          replacedCount: result.replacedCount,
          preservedCount: result.preservedCount,
          deferredReviewCount: result.deferredReviewCount,
          snapshotName: result.snapshotName,
          message:
            "Old MCCC setup fixed. SimSuite cleared the older files out of the way, kept your settings safe, and queued the guided update.",
        } as T;
      }

      if (action.kind === "install_dependency") {
        if (!approved || action.relatedItemId == null) {
          throw new Error("Dependency install was blocked because approval was not confirmed.");
        }
        const result = await mockInvoke<ApplyGuidedDownloadResult>(
          "apply_guided_download_item",
          { itemId: action.relatedItemId, approved: true },
        );
        syncMockDownloadsWatcherStatus();
        return {
          actionKind: action.kind,
          focusItemId: itemId,
          createdItemId: null,
          openedUrl: null,
          snapshotId: result.snapshotId,
          repairedCount: 0,
          installedCount: result.installedCount,
          replacedCount: result.replacedCount,
          preservedCount: result.preservedCount,
          deferredReviewCount: result.deferredReviewCount,
          snapshotName: result.snapshotName,
          message:
            "Dependency installed first. SimSuite re-checked the waiting mod and moved it onto the safer path.",
        } as T;
      }

      if (action.kind === "open_dependency") {
        return {
          actionKind: action.kind,
          focusItemId: action.relatedItemId ?? itemId,
          createdItemId: null,
          openedUrl: null,
          snapshotId: null,
          repairedCount: 0,
          installedCount: 0,
          replacedCount: 0,
          preservedCount: 0,
          deferredReviewCount: 0,
          snapshotName: null,
          message: `Opened ${action.relatedItemName ?? "the dependency"} in the Inbox.`,
        } as T;
      }

      if (action.kind === "open_related_item") {
        return {
          actionKind: action.kind,
          focusItemId: action.relatedItemId ?? itemId,
          createdItemId: null,
          openedUrl: null,
          snapshotId: null,
          repairedCount: 0,
          installedCount: 0,
          replacedCount: 0,
          preservedCount: 0,
          deferredReviewCount: 0,
          snapshotName: null,
          message: `Opened ${action.relatedItemName ?? "the fuller special-mod pack"} in the Inbox.`,
        } as T;
      }

      if (action.kind === "open_official_source") {
        return {
          actionKind: action.kind,
          focusItemId: itemId,
          createdItemId: null,
          openedUrl: action.url,
          snapshotId: null,
          repairedCount: 0,
          installedCount: 0,
          replacedCount: 0,
          preservedCount: 0,
          deferredReviewCount: 0,
          snapshotName: null,
          message: `Opened the official ${action.relatedItemName ?? "download"} page in your browser.`,
        } as T;
      }

      if (action.kind === "download_missing_files") {
        if (!approved) {
          throw new Error("Download was blocked because approval was not confirmed.");
        }
        mockDownloadsItems = mockDownloadsItems.map((item) =>
          item.id === itemId
            ? {
                ...item,
                displayName: "MC_Command_Center_2026.3.0.zip",
                status: "ready",
                intakeMode: "guided",
                riskLevel: "medium",
                detectedFileCount: 3,
                activeFileCount: 3,
                reviewFileCount: 0,
                notes: [
                  "Rechecked with newer SimSuite rules on Mar 10, 2026.",
                  "Trusted official MCCC files were staged into the Inbox.",
                  "This batch is now ready for guided special setup.",
                ],
                guidedInstallAvailable: true,
                updatedAt: new Date().toISOString(),
                sampleFiles: [
                  "mc_cmd_center.ts4script",
                  "mc_cmd_center.package",
                  "mc_woohoo.package",
                ],
              }
            : item,
        );
        syncMockDownloadsWatcherStatus();
        return {
          actionKind: action.kind,
          focusItemId: itemId,
          createdItemId: null,
          openedUrl: null,
          snapshotId: null,
          repairedCount: 0,
          installedCount: 0,
          replacedCount: 0,
          preservedCount: 0,
          deferredReviewCount: 0,
          snapshotName: null,
          message:
            "Trusted MCCC files were downloaded into the Inbox and the batch is ready for guided setup.",
        } as T;
      }

      if (action.kind === "separate_supported_files") {
        if (!approved) {
          throw new Error("Split was blocked because approval was not confirmed.");
        }
        mockDownloadsItems = mockDownloadsItems.filter((item) => item.id !== 48);
        mockDownloadsItems = mockDownloadsItems.map((item) =>
          item.id === itemId
            ? {
                ...item,
                status: "ready",
                intakeMode: "guided",
                riskLevel: "medium",
                detectedFileCount: 3,
                activeFileCount: 3,
                reviewFileCount: 0,
                errorMessage: null,
                notes: [
                  "Rechecked with newer SimSuite rules on Mar 10, 2026.",
                  "Clean MCCC files were split out of the mixed archive.",
                  "This batch is now ready for guided special setup.",
                ],
                guidedInstallAvailable: true,
                updatedAt: new Date().toISOString(),
                sampleFiles: [
                  "mc_cmd_center.ts4script",
                  "mc_cmd_center.package",
                  "mc_woohoo.package",
                ],
              }
            : item,
        );
        mockDownloadsItems.push({
          id: 48,
          displayName: "MCCC_mixed_folder_extras.zip",
          sourcePath: `${DEFAULT_DOWNLOADS_PATH}\\MCCC_mixed_folder.zip`,
          sourceKind: "archive",
          archiveFormat: "zip",
          status: "needs_review",
          sourceSize: 608_640,
          detectedFileCount: 2,
          activeFileCount: 2,
          appliedFileCount: 0,
          reviewFileCount: 2,
          firstSeenAt: new Date().toISOString(),
          lastSeenAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          errorMessage: null,
          notes: ["These are the leftover files from the split MCCC batch."],
          intakeMode: "standard",
          riskLevel: "medium",
          matchedProfileKey: null,
          matchedProfileName: null,
          specialFamily: null,
          assessmentReasons: ["These files were separated from the MCCC set and still need their own check."],
          dependencySummary: [],
          missingDependencies: [],
          inboxDependencies: [],
          incompatibilityWarnings: [],
          postInstallNotes: [],
          evidenceSummary: ["Leftover files were kept separate so the clean MCCC set could continue safely."],
          catalogSource: null,
          existingInstallDetected: false,
          guidedInstallAvailable: false,
          sampleFiles: ["othermod.ts4script", "notes.package"],
        });
        syncMockDownloadsWatcherStatus();
        return {
          actionKind: action.kind,
          focusItemId: itemId,
          createdItemId: 48,
          openedUrl: null,
          snapshotId: null,
          repairedCount: 0,
          installedCount: 0,
          replacedCount: 0,
          preservedCount: 0,
          deferredReviewCount: 0,
          snapshotName: null,
          message:
            "The clean MCCC files were split into their own batch and the extras stayed behind for review.",
        } as T;
      }

      throw new Error("This mock review action is not implemented yet.");
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
      syncMockDownloadsWatcherStatus();
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
  getDownloadItemGuidedPlan: (itemId: number) =>
    invoke<GuidedInstallPlan | null>("get_download_item_guided_plan", { itemId }),
  getDownloadItemReviewPlan: (itemId: number) =>
    invoke<SpecialReviewPlan | null>("get_download_item_review_plan", { itemId }),
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
  applyGuidedDownloadItem: (itemId: number, approved = false) =>
    invoke<ApplyGuidedDownloadResult>("apply_guided_download_item", {
      itemId,
      approved,
    }),
  applySpecialReviewFix: (itemId: number, approved = false) =>
    invoke<ApplySpecialReviewFixResult>("apply_special_review_fix", {
      itemId,
      approved,
    }),
  applyReviewPlanAction: (
    itemId: number,
    actionKind: ReviewPlanAction["kind"],
    relatedItemId?: number | null,
    url?: string | null,
    approved = false,
  ) =>
    invoke<ApplyReviewPlanActionResult>("apply_review_plan_action", {
      itemId,
      actionKind,
      relatedItemId: relatedItemId ?? null,
      url: url ?? null,
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
