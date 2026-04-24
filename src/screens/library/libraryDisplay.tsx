import {
  creatorConfidenceTier,
  creatorLabelWithConfidence,
  creatorConfidenceSuffix,
  friendlyTypeLabel,
  unknownCreatorLabel,
  versionLabelWithConfidence,
  versionConfidenceTierLabel,
  type CreatorConfidenceTier,
} from "../../lib/uiLanguage";
import type { ReactNode } from "react";
import type {
  FileDetail,
  FileInsights,
  FileRelationship,
  LibraryFileRow,
  PreviewSource,
  ProofLevel,
  RelationshipCue,
  RelationshipType,
  UserView,
  VersionConfidence,
  WatchStatus,
} from "../../lib/types";

export type { FolderNode } from "./folderTree";

export interface LibraryViewFlags {
  showCreatorInList: boolean;
  showInspectFactsInList: boolean;
  showAdvancedFilters: boolean;
  showRootFacts: boolean;
  maxSupportingFacts: number;
  // Grid card chip density per view
  maxCasNames: number;
  maxScriptNamespaces: number;
  // Card content: ScriptMods namespace limit per mode
  cardMaxScriptNamespaces: number;
}

/** CSS variable name for the type color dot/border */
export type TypeColor =
  | "cas"
  | "script"
  | "gameplay"
  | "buildbuy"
  | "override"
  | "poses"
  | "presets"
  | "tray"
  | "unknown";

/**
 * Card model for the thumbnail/grid view.
 * Each mod type has a "primary content signal" — the thing a simmer most wants to know.
 * The card renderer selects which content block to show based on kind.
 *
 * Content signal hierarchy:
 * - CAS: embeddedNames chips (item identifiers)
 * - ScriptMods: scriptNamespaces chips + version badge
 * - BuildBuy / Overrides: resourceSummary line
 * - Poses / Presets: subtype label
 * - Tray items: bundleName / household identity
 * - Unknown: resourceSummary or fallback
 */
export interface LibraryCardModel {
  // Identity (always shown)
  id: number;
  /** Raw filename — used as fallback/anchor, use displayTitle for UI */
  title: string;
  /** Clean display title: cleaned filename, suitable for casual eyes */
  displayTitle: string;
  identityLabel: string | null;
  kind: string;
  typeLabel: string;
  typeColor: TypeColor;
  creatorLabel: string;

  // Status signals (always shown in header)
  isTray: boolean;
  isMisplaced: boolean;
  watchStatusLabel: string;
  watchStatusTone: "calm" | "attention" | "muted";
  healthLabel: string | null;
  healthTone: "attention" | "muted" | null;
  hasDuplicate: boolean;
  hasIssues: boolean;
  confidenceLevel: "high" | "medium" | "low";

  // Grouping
  isGrouped: boolean;
  groupedCount: number;
  bundleName: string | null;

  // Type-specific content signals (one is always primary)
  /** CAS: embedded item names as chips */
  casNames: string[];
  casNamesOverflow: number;
  /** ScriptMods: script namespaces as chips */
  scriptNamespaces: string[];
  scriptNamespaceOverflow: number;
  /** ScriptMods: namespaces limited for grid card (view-aware) */
  cardScriptNamespaces: string[];
  cardScriptNamespaceOverflow: number;
  scriptVersionLabel: string | null;
  /** BuildBuy / Overrides / generic: resource summary line */
  contentSummary: string | null;
  /** Poses / Presets / generic fallback: subtype */
  subtype: string | null;
  /** Tray items: household/lot/room identity name */
  trayIdentityLabel: string | null;
  /** Version signal (for ScriptMods and others) */
  versionLabel: string | null;
  /** Color swatches extracted from embeddedNames / familyHints keywords */
  colorSwatches: string[];

  // Relationship cue — derived from backend window fields (Phase 5ao)
  relationshipCue?: RelationshipCue;

  // The raw row for click handling
  row: LibraryFileRow;
  /** Base64 PNG thumbnail: cached (localthumbcache) or embedded (THUM resource) */
  thumbnailPreview: string | null;
  /** Where the thumbnail came from */
  previewSource: PreviewSource;
  /** Base64 PNG thumbnail from localthumbcache.package (highest quality), if available */
  cachedThumbnailPreview: string | null;
}

export interface LibraryRowModel {
  id: number;
  /** Raw filename — use displayTitle for UI */
  title: string;
  /** Clean display title for row rendering */
  displayTitle: string;
  /** Base64 PNG thumbnail: cached (localthumbcache) or embedded (THUM resource) */
  thumbnailPreview: string | null;
  /** Where the thumbnail came from */
  previewSource: PreviewSource;
  /** Base64 PNG from localthumbcache.package, if available */
  cachedThumbnailPreview: string | null;
  identityLabel: string | null;
  kind: string;
  typeLabel: string;
  /** CSS type-color key for the type indicator dot/border */
  typeColor: TypeColor;
  /** True if this item lives in the tray (effectively disabled) */
  isTray: boolean;
  /** True if this item is in the wrong folder for its type (e.g. tray item in Mods) */
  isMisplaced: boolean;
  /** Tray identity facts if this is a tray kind, null otherwise */
  trayIdentity: TrayIdentity | null;
  /** Duplicate flag — renders only when this file appears in a duplicate pair. */
  hasDuplicate: boolean;
  watchStatusLabel: string;
  watchStatusTone: "calm" | "attention" | "muted";
  /** Renders only when there is an issue. null = no health indicator shown. */
  healthLabel: string | null;
  healthTone: "attention" | "muted" | null;
  duplicateLabel: string | null;
  duplicateTone: "muted" | null;
  supportingFacts: string[];
  /** Confidence level derived from the raw 0–1 confidence value */
  confidenceLevel: "high" | "medium" | "low";
  /** Short confidence text for display */
  confidenceLabel: string;
  /** True if the item has any safety/parser issues */
  hasIssues: boolean;
  /** Color swatches extracted from embeddedNames / familyHints keywords */
  colorSwatches: string[];
  relationshipCue?: RelationshipCue;
}

type LibraryCareSummarySource = Pick<
  FileDetail,
  "installedVersionSummary" | "safetyNotes" | "parserWarnings" | "kind" | "sourceLocation"
>;

export function libraryViewFlags(userView: UserView): LibraryViewFlags {
  return {
    showCreatorInList: userView !== "beginner",
    showInspectFactsInList: userView === "power",
    showAdvancedFilters: true, // accessible to all users — content can be simplified for beginners if needed
    showRootFacts: userView !== "beginner",
    maxSupportingFacts: userView === "power" ? 3 : 2,
    // Grid card chip density per view
    maxCasNames: userView === "beginner" ? 2 : userView === "power" ? 4 : 3,
    maxScriptNamespaces: userView === "beginner" ? 1 : userView === "power" ? 4 : 2,
    // Card namespace limit: Casual=0 (skip namespace chips, show creator/version fallback),
    // Seasoned=2 (primary + 1), Creator=4 (full)
    cardMaxScriptNamespaces: userView === "beginner" ? 0 : userView === "power" ? 4 : 2,
  };
}

function cleanTechnicalLabel(value: string): string {
  return value
    .replace(/\.(package|ts4script|trayitem|blueprint|bpi|householdbinary)$/i, "")
    // Strip _0xHEXID (DBPF Group/Instance IDs: _0x00ABCDEF)
    .replace(/_0x[0-9a-f]{6,8}$/i, "")
    // Strip plain _HEXID suffixes (raw hex without 0x prefix)
    .replace(/_[0-9a-f]{6,16}$/i, "")
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function libraryIdentityLabelForFilename(filename: string, identity: string): string | null {
  const cleanedFilename = cleanTechnicalLabel(filename).toLowerCase();
  const cleanedIdentity = cleanTechnicalLabel(identity).toLowerCase();
  return cleanedFilename === cleanedIdentity ? null : identity;
}

export function describeLibraryPrimaryLabel(
  file: Pick<LibraryFileRow, "filename" | "kind" | "subtype" | "creator" | "bundleName" | "sourceLocation" | "insights">,
): string {
  const subtype = file.subtype?.trim() || null;
  const creator = file.creator?.trim() || null;
  const embeddedNames = describeEmbeddedNames(file.insights);
  const scriptNamespaces = describeScriptNamespaces(file.insights);
  const trayGrouping = usefulTrayGroupingValue(file);
  const trayIdentity = describeTrayIdentity(file as Pick<FileDetail, "kind" | "subtype" | "sourceLocation"> & { insights?: FileInsights });
  const contentProfile = summarizePackageContentProfile(file.insights, file.kind, file.subtype ?? null);

  switch (file.kind) {
    case "CAS":
      return embeddedNames[0] ? cleanTechnicalLabel(embeddedNames[0]) : subtype ?? "CAS item";
    case "ScriptMods":
      if (creator) return `${creator} script mod`;
      if (scriptNamespaces.samples[0]) return `${scriptNamespaces.samples[0]} script mod`;
      return subtype ? `${subtype} script mod` : "Script mod";
    case "BuildBuy":
      return subtype ?? contentProfile ?? "Build/Buy item";
    case "Gameplay":
      return subtype ?? contentProfile ?? "Gameplay mod";
    case "OverridesAndDefaults":
      return subtype ?? contentProfile ?? "Override package";
    case "PosesAndAnimation":
      return subtype ?? "Pose / animation";
    case "PresetsAndSliders":
      return subtype ?? "Preset / slider";
    case "TrayHousehold":
    case "TrayLot":
    case "TrayRoom":
    case "TrayItem":
    case "Household":
    case "Lot":
    case "Room": {
      const trayLabel = trayIdentity.kind !== "standard" ? trayKindLabel(trayIdentity.kind) : "Tray item";
      return trayGrouping ?? trayLabel;
    }
    default:
      return subtype ?? contentProfile ?? cleanTechnicalLabel(file.filename) ?? "Unknown file";
  }
}

export function buildLibraryRowModel(
  row: LibraryFileRow,
  userView: UserView,
): LibraryRowModel {
  const flags = libraryViewFlags(userView);
  // ScriptMods with a detectable version signal get a 3rd slot in standard view
  // so the version clue appears without needing power view.
  if (
    userView === "standard" &&
    row.kind === "ScriptMods" &&
    row.insights?.versionSignals?.some((s) => s.confidence >= 0.55)
  ) {
    flags.maxSupportingFacts = 3;
  }
  const creatorLabel = row.creator ?? unknownCreatorLabel(userView);
  const isTray = row.sourceLocation === "tray";
  const confidence = row.confidence ?? 0;
  const confidenceLevel: LibraryRowModel["confidenceLevel"] =
    confidence >= 0.8 ? "high" : confidence >= 0.55 ? "medium" : "low";
  const hasIssues = row.safetyNotes.length > 0 || row.parserWarnings.length > 0;

  const supportingFacts = buildSupportingFacts(row, {
    flags,
    creatorLabel,
    isTray,
    userView,
  });

  const watchStatusLabel = describeWatchStatus(row.watchStatus);
  const watchStatusTone = watchStatusToneFor(row.watchStatus);

  const healthIssue = computeLibraryHealthIssue(row);

  const trayIdentity = describeTrayIdentity({
    kind: row.kind,
    subtype: row.subtype,
    sourceLocation: row.sourceLocation,
  });
  const primaryLabel = describeLibraryPrimaryLabel(row);
  // displayTitle: cleaned filename — the primary identity a simmer uses to recognise the file
  const displayTitle = cleanTechnicalLabel(row.filename);

  return {
    id: row.id,
    title: row.filename,
    displayTitle,
    identityLabel: libraryIdentityLabelForFilename(row.filename, primaryLabel),
    // Cascade: Mod Manager → game cache → embedded THUM → null
    // First-party cascade: embedded THUM → game cache → fallback
    // embedded is the foundation (no external dependency).
    // Game cache (localthumbcache) is a shared secondary when available.
    thumbnailPreview: row.insights?.thumbnailPreview ?? row.insights?.cachedThumbnailPreview ?? null,
    previewSource: row.insights?.thumbnailPreview
        ? 'embedded'
        : (row.insights?.cachedThumbnailPreview ? 'cache' : 'fallback'),
    cachedThumbnailPreview: row.insights?.cachedThumbnailPreview ?? null,
    kind: row.kind,
    typeLabel: friendlyTypeLabel(row.kind),
    typeColor: typeColorForKind(row.kind),
    isTray,
    isMisplaced: trayIdentity.isMisplaced,
    trayIdentity,
    hasDuplicate: row.hasDuplicate ?? false,
    watchStatusLabel,
    watchStatusTone,
    healthLabel: healthIssue?.label ?? null,
    healthTone: healthIssue?.tone ?? null,
    duplicateLabel: row.hasDuplicate ? "Duplicate" : null,
    duplicateTone: row.hasDuplicate ? "muted" : null,
    supportingFacts: supportingFacts.slice(0, flags.maxSupportingFacts),
    confidenceLevel,
    confidenceLabel:
      confidenceLevel === "high"
        ? "High confidence"
        : confidenceLevel === "medium"
          ? "Medium confidence"
          : "Low confidence",
    hasIssues,
    colorSwatches: extractColorSwatches(
      row.insights?.embeddedNames ?? [],
      row.insights?.familyHints ?? [],
    ),
    relationshipCue: deriveRelationshipCue(computeFileRelationship(row, [])) ?? undefined,
  };
}

/**
 * Builds a card model for the thumbnail/grid view.
 * Per-type content strategy:
 * - CAS: embeddedNames chips (item identifier names)
 * - ScriptMods: scriptNamespaces chips + version badge
 * - BuildBuy / Overrides: resourceSummary line
 * - Poses / Presets: subtype label
 * - Tray items: bundleName / household identity
 * - Unknown: resourceSummary or subtype or fallback
 */
export function buildLibraryCardModel(
  row: LibraryFileRow,
  userView: UserView,
): LibraryCardModel {
  const creatorLabel = row.creator ?? unknownCreatorLabel(userView);
  const isTray = row.sourceLocation === "tray";
  const confidence = row.confidence ?? 0;
  const confidenceLevel: LibraryCardModel["confidenceLevel"] =
    confidence >= 0.8 ? "high" : confidence >= 0.55 ? "medium" : "low";
  const hasIssues = row.safetyNotes.length > 0 || row.parserWarnings.length > 0;

  const watchStatusLabel = describeWatchStatus(row.watchStatus);
  const watchStatusTone = watchStatusToneFor(row.watchStatus);
  const healthIssue = computeLibraryHealthIssue(row);

  const trayIdentity = describeTrayIdentity({
    kind: row.kind,
    subtype: row.subtype,
    sourceLocation: row.sourceLocation,
  });

  // Grouping
  const isGrouped = Boolean(row.bundleName && (row.groupedFileCount ?? 0) > 1);
  const groupedCount = row.groupedFileCount ?? 0;
  const bundleName = row.bundleName ?? null;

  // CAS: embeddedNames — the item identifier strings (e.g. "NSW_Skinblend")
  const flags = libraryViewFlags(userView);
  const allCasNames = row.insights?.embeddedNames ?? [];
  const visibleCasNames = allCasNames.slice(0, flags.maxCasNames);
  const casNamesOverflow = Math.max(0, allCasNames.length - flags.maxCasNames);

  // ScriptMods: scriptNamespaces + version
  const allNamespaces = row.insights?.scriptNamespaces ?? [];
  const visibleNamespaces = allNamespaces.slice(0, flags.maxScriptNamespaces);
  const scriptNamespaceOverflow = Math.max(0, allNamespaces.length - flags.maxScriptNamespaces);
  // Grid card: may use a different (tighter) cap for Casual clarity
  const cardVisibleNamespaces = allNamespaces.slice(0, flags.cardMaxScriptNamespaces);
  const cardScriptNamespaceOverflow = Math.max(0, allNamespaces.length - flags.cardMaxScriptNamespaces);
  const versionSignal = row.insights?.versionSignals?.[0];
  const scriptVersionLabel = versionSignal
    ? `v${versionSignal.normalizedValue}`
    : row.insights?.versionHints?.[0] ?? null;

  // Generic content summary: resourceSummary — humanized for grid card display
  const rawContentSummary = row.insights?.resourceSummary?.[0] ?? null;
  const contentSummary = describeResourceSummary(rawContentSummary ?? '');

  // Subtype
  const subtype = row.subtype?.trim() ?? null;

  // Tray identity label (household/lot/room name)
  const trayIdentityLabel = trayIdentity.kind !== "standard"
    ? (usefulTrayGroupingValue({ bundleName: row.bundleName ?? null, insights: row.insights })
      ?? trayKindLabel(trayIdentity.kind))
    : null;

  // Generic version label
  const versionLabel = scriptVersionLabel;
  const primaryLabel = describeLibraryPrimaryLabel(row);
  // Clean display title: cleaned filename — what the file is actually called
  const displayTitle = cleanTechnicalLabel(row.filename);

  return {
    id: row.id,
    title: row.filename,
    displayTitle,
    identityLabel: libraryIdentityLabelForFilename(row.filename, primaryLabel),
    // Cascade: Mod Manager → game cache → embedded THUM → null
    // First-party cascade: embedded THUM → game cache → fallback
    // embedded is the foundation (no external dependency).
    // Game cache (localthumbcache) is a shared secondary when available.
    thumbnailPreview: row.insights?.thumbnailPreview ?? row.insights?.cachedThumbnailPreview ?? null,
    previewSource: row.insights?.thumbnailPreview
        ? 'embedded'
        : (row.insights?.cachedThumbnailPreview ? 'cache' : 'fallback'),
    cachedThumbnailPreview: row.insights?.cachedThumbnailPreview ?? null,
    kind: row.kind,
    typeLabel: friendlyTypeLabel(row.kind),
    typeColor: typeColorForKind(row.kind),
    creatorLabel,
    isTray,
    isMisplaced: trayIdentity.isMisplaced,
    watchStatusLabel,
    watchStatusTone,
    healthLabel: healthIssue?.label ?? null,
    healthTone: healthIssue?.tone ?? null,
    hasDuplicate: row.hasDuplicate ?? false,
    hasIssues,
    confidenceLevel,
    isGrouped,
    groupedCount,
    bundleName,
    casNames: visibleCasNames,
    casNamesOverflow,
    scriptNamespaces: visibleNamespaces,
    scriptNamespaceOverflow,
    cardScriptNamespaces: cardVisibleNamespaces,
    cardScriptNamespaceOverflow,
    scriptVersionLabel,
    contentSummary,
    subtype,
    trayIdentityLabel,
    versionLabel,
    colorSwatches: extractColorSwatches(allCasNames, row.insights?.familyHints ?? []),
    row,
    // Relationship cue — derived from backend window fields (Phase 5ao)
    relationshipCue: deriveRelationshipCue(computeFileRelationship(row, [])) ?? undefined,
  };
}

// ─── Color Swatch Extraction ────────────────────────────────────────────────

const COLOR_KEYWORDS: [RegExp, string][] = [
  [/\bred\b|scarlet|crimson|vermillion|ruby|sangre/i, "#DC2626"],
  [/\borange\b|amber|rust|tangerine|peach/i, "#EA580C"],
  [/\byellow\b|gold|lemon|mustard|cream/i, "#CA8A04"],
  [/\bgreen\b|emerald|sage|forest|olive|mint|teal/i, "#16A34A"],
  [/\bblue\b|navy|cobalt|royal|aqua|sky/i, "#2563EB"],
  [/\bindigo\b|violet|lavender|purple/i, "#7C3AED"],
  [/\bpink\b|rose|blush|magenta|fuchsia|raspberry/i, "#DB2777"],
  [/\bbrown\b|chocolate|coffee|tan|caramel|walnut/i, "#92400E"],
  [/\bblack\b|charcoal|obsidian|jet|onyx/i, "#18181B"],
  [/\bwhite\b|ivory|pearl|snow|lattice/i, "#F4F4F5"],
  [/\bgray\b|grey|slate|ash|smoke/i, "#71717A"],
  [/\bsilver\b|chrome|platinum/i, "#A8A29E"],
  [/\bglass\b|transparent|crystal/i, "#93C5FD"],
  [/\bmarble\b/i, "#E5E7EB"],
  [/\bwood\b|oak|pine|maple|walnut|wooden/i, "#A16207"],
  [/\bmetal\b|metallic|brushed|steel/i, "#6B7280"],
  [/\bleather\b/i, "#78350F"],
  [/\bfloral\b|flower|rose petals|blossom/i, "#F472B6"],
];

/**
 * Extract up to 6 hex color swatches from CAS item names and family hints.
 * Returns a curated list — no duplicates, in priority order.
 */
export function extractColorSwatches(embeddedNames: string[], familyHints: string[]): string[] {
  const seen = new Set<string>();
  const swatches: string[] = [];
  const sources = [...embeddedNames, ...familyHints];

  for (const source of sources) {
    for (const [keyword, hex] of COLOR_KEYWORDS) {
      if (keyword.test(source) && !seen.has(hex)) {
        seen.add(hex);
        swatches.push(hex);
        if (swatches.length >= 6) return swatches;
      }
    }
  }
  return swatches;
}

/** Maps SimSuite kind (PascalCase) to a CSS type-color key */
export function typeColorForKind(kind: string): TypeColor {
  switch (kind) {
    case "CAS":
      return "cas";
    case "ScriptMods":
      return "script";
    case "Gameplay":
      return "gameplay";
    case "BuildBuy":
      return "buildbuy";
    case "OverridesAndDefaults":
      return "override";
    case "PosesAndAnimation":
      return "poses";
    case "PresetsAndSliders":
      return "presets";
    case "TrayHousehold":
    case "TrayLot":
    case "TrayRoom":
    case "TrayItem":
    case "Household":
    case "Lot":
    case "Room":
      return "tray";
    default:
      return "unknown";
  }
}

export function summarizeLibraryCareState(
  detail: LibraryCareSummarySource,
): string {
  const isTrayItem = ["TrayHousehold", "TrayLot", "TrayRoom", "TrayItem", "Household", "Lot", "Room"].includes(detail.kind);

  if (isTrayItem && detail.sourceLocation === "mods") {
    return "This looks like tray content outside the Tray folder, so it deserves a quick review.";
  }
  if (isTrayItem && detail.sourceLocation === "tray") {
    return "This file lives in Tray and behaves like library content, not an active mod.";
  }
  if (detail.safetyNotes.length) {
    return "This file has safety notes that deserve attention.";
  }
  if (detail.parserWarnings.length) {
    return "This file has parser warnings worth reviewing.";
  }
  if (detail.installedVersionSummary) {
    return "This file has update tracking ready if you want to check it.";
  }
  return "Nothing stands out right now.";
}

export function formatLibraryFileFormat(file: Pick<FileDetail, "insights" | "extension" | "path">): string {
  if (isLibraryScriptArchive(file)) {
    return "Script archive (.ts4script)";
  }
  if (file.insights?.format === "dbpf-package" || file.extension === ".package") {
    return "Package file (.package)";
  }
  return file.insights?.format ?? file.extension ?? file.path.split(".").pop()?.toUpperCase() ?? "Unknown";
}

export function isLibraryScriptArchive(
  file: Pick<FileDetail, "insights" | "extension">,
): boolean {
  return file.insights?.format === "ts4script-zip" || file.extension === ".ts4script";
}

export function summarizeLibraryScriptContent(
  file: Pick<FileDetail, "insights" | "extension">,
): string | null {
  if (!isLibraryScriptArchive(file)) {
    return null;
  }

  const namespaceCount = file.insights?.scriptNamespaces?.length ?? 0;
  if (namespaceCount > 0) {
    return `${namespaceCount} script ${namespaceCount === 1 ? "folder" : "folders"}`;
  }

  return file.insights?.resourceSummary?.find((item) => /archive entries:/i.test(item)) ?? "Script archive";
}

export function summarizeLibraryResourceBadge(
  file: Pick<FileDetail, "kind" | "subtype" | "insights" | "extension">,
): string | null {
  if (isLibraryScriptArchive(file)) {
    return null;
  }

  return summarizePackageContentProfile(file.insights, file.kind, file.subtype);
}

export function formatLibraryFamilyHintLabel(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return value;
  }

  if (!/[\s_-]/.test(trimmed) && /^[a-z0-9]+$/i.test(trimmed) && trimmed.length <= 5) {
    return trimmed.toUpperCase();
  }

  const aliases = new Map<string, string>([
    ["mccc", "MCCC"],
    ["s4cl", "S4CL"],
    ["tmex", "TMEX"],
    ["xml", "XML"],
  ]);

  return trimmed
    .split(/[\s_-]+/)
    .filter(Boolean)
    .map((part) => {
      const lower = part.toLowerCase();
      if (aliases.has(lower)) {
        return aliases.get(lower) ?? part;
      }
      if (/^[0-9]+$/.test(part)) {
        return part;
      }
      return `${part.charAt(0).toUpperCase()}${part.slice(1)}`;
    })
    .join(" ");
}

export function describeLibraryFamilyContext(
  file: Pick<FileDetail, "kind" | "subtype" | "insights">,
): string | null {
  const primaryFamily = file.insights?.familyHints?.find((item) => item.trim());
  if (!primaryFamily) {
    return null;
  }

  const familyLabel = formatLibraryFamilyHintLabel(primaryFamily);
  const subtype = file.subtype?.trim();
  const hasRoleLikeSubtype = subtype
    ? ["core", "module", "library", "utility", "utilities", "framework"].includes(
        subtype.toLowerCase(),
      )
    : false;

  if (hasRoleLikeSubtype && subtype) {
    return `${subtype} role in the ${familyLabel} family`;
  }

  if (file.kind === "ScriptMods") {
    return `${familyLabel} script family`;
  }
  if (file.kind === "Gameplay") {
    return `${familyLabel} gameplay family`;
  }
  if (file.kind === "CAS") {
    return `Linked to ${familyLabel} set`;
  }
  return `${familyLabel} family`;
}

/**
 * Returns a disclosure-aware creator label for use in the inspector snapshot.
 * Handles tier 1–4 creator certainty without showing raw confidence jargon.
 */
export function describeCreatorForInspector(
  file: Pick<FileDetail, "creator" | "creatorLearning" | "insights">,
): { label: string; suffix: string | null; tier: CreatorConfidenceTier } {
  const tier = creatorConfidenceTier(
    file.creator,
    file.creatorLearning ?? null,
    file.insights?.creatorHints ?? [],
  );
  const label = creatorLabelWithConfidence(
    file.creator,
    file.creatorLearning ?? null,
    file.insights?.creatorHints ?? [],
  );
  const suffix = creatorConfidenceSuffix(
    file.creator,
    file.creatorLearning ?? null,
    file.insights?.creatorHints ?? [],
  );
  return { label, suffix, tier };
}

/**
 * Returns a version label with appropriate confidence disclosure.
 * Versions with unknown confidence are returned as null — caller should not render.
 */
export function describeVersionForInspector(
  version: string | null,
  confidence: VersionConfidence | null,
): { label: string | null; tierLabel: string } {
  return {
    label: versionLabelWithConfidence(version, confidence),
    tierLabel: versionConfidenceTierLabel(confidence),
  };
}

/**
 * Returns the primary family hint with a label prefix indicating it is
 * a derived clue, not raw extracted content.
 */
export function describeFamilyHintForSheet(
  file: Pick<FileDetail, "kind" | "subtype" | "insights">,
): { hint: string | null; role: string } {
  const primaryFamily = file.insights?.familyHints?.find((item) => item.trim());
  if (!primaryFamily) {
    return { hint: null, role: "family" };
  }
  return { hint: primaryFamily, role: "family" };
}

/**
 * Splits a concatenated CAS name into meaningful segments.
 * e.g. "NSW_SkinblendNSW_Eyelids" → ["NSW_Skinblend", "NSW_Eyelids"]
 * e.g. "Hairblend_Eyeliner_01_Overlay" → ["Hairblend_Eyeliner", "01_Overlay"]
 * Detects repeated namespace prefixes and CamelCase boundaries.
 */
function tokenizeCasName(name: string): string[] {
  const s = name.trim();
  if (!s) return [];

  // Strategy 1: repeated namespace prefix (e.g. "NSW_SkinblendNSW_Eyelids")
  const firstUnderscore = s.indexOf("_");
  if (firstUnderscore > 1 && firstUnderscore < 25) {
    const prefix = s.substring(0, firstUnderscore);
    // Look for the same prefix appearing again later in the string
    const remainder = s.substring(firstUnderscore + 1);
    const secondOccurrence = remainder.indexOf(prefix + "_");
    if (secondOccurrence > 2) {
      // e.g. "NSW_SkinblendNSW_Eyelids": prefix=NSW, remainder="SkinblendNSW_Eyelids",
      // secondOccurrence=9 (finds "NSW_" at position 9 in "SkinblendNSW_Eyelids")
      const part1 = s.substring(0, firstUnderscore + 1 + secondOccurrence);
      const part2 = remainder.substring(secondOccurrence + prefix.length + 1);
      if (part1.length > 3 && part2.length > 2) {
        return [part1.replace(/_+$/, ""), part2.replace(/_+$/, "")];
      }
    }
  }

  // Strategy 2: long concatenated names — detect CamelCase boundary mid-name
  // e.g. "FooBarFooBarThing" -> split at the repeated "FooBar"
  const camelMatch = s.match(/^([A-Z][a-z]+)((?:[A-Z][a-z]+)+)(.*)$/);
  if (camelMatch) {
    const [, firstWord, rest] = camelMatch;
    if (rest.length > 5) {
      // Check if "firstWord" appears again in "rest" — that's the split point
      const repeatIndex = rest.indexOf(firstWord);
      if (repeatIndex > 2) {
        const part1 = firstWord + rest.substring(0, repeatIndex);
        const part2 = rest.substring(repeatIndex + firstWord.length);
        if (part1.length >= 4 && part2.length >= 4) return [part1, part2];
      }
    }
  }

  // Strategy 3: underscore groups — only split 4+ parts into material | variant
  // e.g. "Hairblend_Eyeliner_01_Overlay" (4 parts) → ["Hairblend_Eyeliner", "01_Overlay"]
  // 1-3 parts are already clean and should not be split (e.g. "Miiko_Brow_03")
  const parts = s.split("_");
  if (parts.length >= 4) {
    return [parts.slice(0, 2).join("_"), parts.slice(2).join("_")].filter(t => t.length > 2);
  }

  return [s];
}

/**
 * Returns embedded names filtered to likely human-readable values for display.
 * Also tokenizes concatenated CAS names (e.g. "NSW_SkinblendNSW_Eyelids" → ["NSW_Skinblend", "NSW_Eyelids"]).
 * Filters out raw STBL UUID-style entries and other non-meaningful strings.
 */
export function describeEmbeddedNames(
  insights: FileInsights | undefined,
): string[] {
  if (!insights?.embeddedNames?.length) return [];

  const result: string[] = [];

  for (const name of insights.embeddedNames) {
    const trimmed = name.trim();
    if (!trimmed) continue;
    // Skip pure hex/UUID-looking strings
    if (/^[0-9a-f]{8}[-]?[0-9a-f]{4}[-]?[0-9a-f]{4}[-]?[0-9a-f]{4}[-]?[0-9a-f]{12}$/i.test(trimmed)) continue;
    // Skip plain 8-char hex IDs (no dashes, no 0x prefix)
    if (/^[0-9a-f]{8}$/i.test(trimmed)) continue;
    // Skip 0x-prefixed hex IDs (6-8 hex chars after 0x)
    if (/^0x[0-9a-f]{6,8}$/i.test(trimmed)) continue;
    // Skip very short numeric strings
    if (/^\d+$/.test(trimmed) && trimmed.length < 4) continue;
    // Skip strings that are mostly internal markers
    if (trimmed.startsWith("#") && trimmed.length < 10) continue;

    // Tokenize concatenated names (e.g. "NSW_SkinblendNSW_Eyelids" → two chips)
    const tokens = tokenizeCasName(trimmed);
    for (const token of tokens) {
      const cleaned = cleanTechnicalLabel(token);
      if (cleaned.length > 1) result.push(cleaned);
    }
  }

  return [...new Set(result)].slice(0, 8); // dedupe + cap
}

/**
 * Describes script namespaces for display — shows count and first few examples.
 */
export function describeScriptNamespaces(
  insights: FileInsights | undefined,
): { count: number; samples: string[] } {
  const namespaces = insights?.scriptNamespaces ?? [];
  return {
    count: namespaces.length,
    samples: namespaces.slice(0, 5),
  };
}

/** Single-line script scope label for list rows and inspectors.
 * viewMode tunes the sample count: Casual caps at 1, Seasoned at 2, Creator at 5.
 */
export function summarizeScriptScopeForUi(
  insights: FileInsights | undefined,
  viewMode: "beginner" | "standard" | "power" = "standard",
): string | null {
  const maxSamples = viewMode === "beginner" ? 1 : viewMode === "power" ? 5 : 2;
  const namespaces = insights?.scriptNamespaces ?? [];
  if (!namespaces.length) return null;
  const samples = namespaces.slice(0, maxSamples);
  if (namespaces.length === 1) return samples[0] ?? null;
  return `${samples[0] ?? "?"}+${namespaces.length - 1}`;
}

/**
 * Transforms a raw resource-summary string into a simmer-friendly label.
 * Handles both the Rust-backend humanized format ("N build/buy items")
 * and any raw DBPF strings that slip through from mock data or unknown types.
 * Returns null when the entry is too noisy or internal to be useful.
 */
export function describeResourceSummary(raw: string): string | null {
  const s = raw.trim();
  if (!s) return null;

  // Already humanized by the Rust backend — pass through cleanly
  if (/^\d+ /.test(s)) return s;

  // Pattern: "Foo x N" or "Foo xN" (raw DBPF mock format) -> "N Foo items"
  const countXMatch = s.match(/^([A-Z][a-zA-Z0-9]+)\s*x\s*(\d+)$/);
  if (countXMatch) {
    const [, label, count] = countXMatch;
    // Internal/noise types - suppress entirely
    if (/^(NameMap|Compressed|NameMapBlob|Uncomp|S4mpd|S4mpdData)/i.test(label)) return null;
    const normalised = label.toLowerCase();
    return count + " " + normalised + " item" + (Number(count) !== 1 ? "s" : "");
  }

  // Pattern: "Foo: N" (some raw formats use colon) -> "N Foo"
  const colonMatch = s.match(/^([A-Za-z]+)\s*:\s*(\d+)$/);
  if (colonMatch) {
    const [, label, count] = colonMatch;
    const normalised = label.toLowerCase();
    if (/^(image|img|picture|photo)$/.test(normalised)) return count + " image" + (Number(count) !== 1 ? "s" : "");
    if (/^(audio|sound|music|sfx)$/.test(normalised)) return count + " audio " + (Number(count) !== 1 ? "entries" : "entry");
    if (/^(video|anim)$/.test(normalised)) return count + " animation" + (Number(count) !== 1 ? "s" : "");
    if (/^(string|stringtable|text|strtable)$/.test(normalised)) return count + " text " + (Number(count) !== 1 ? "entries" : "entry");
    if (/^(name|namemap)$/.test(normalised)) return null; // internal noise
    return count + " " + label;
  }

  // Hex-like internal values (8-char hex strings, Group/Instance IDs) - suppress
  if (/^[0-9a-f]{8}([0-9a-f]{8})?$/i.test(s)) return null;
  if (/^(0x)?[0-9a-f]{7,16}$/i.test(s)) return null;

  // "Archive entries: N" - pass through as-is (already readable)
  if (/^archive entries/i.test(s)) return s;
  // "Top-level namespaces: N" - pass through
  if (/^top-?level namespaces/i.test(s)) return s;

  // Unknown format - if it looks too short or too technical, suppress
  if (s.length <= 4) return null;

  return s;
}

/**
 * Returns the first useful, humanized resource-summary entry, or null.
 * Filters out noisy internal values that don't help a simmer understand the file.
 */
export function summarizeResourceProfileForUi(
  insights: FileInsights | undefined,
): string | null {
  const summary = insights?.resourceSummary ?? [];
  if (!summary.length) return null;
  for (const raw of summary) {
    const humanized = describeResourceSummary(raw);
    if (humanized) return humanized;
  }
  return null;
}

/** Best version clue for quick UI surfaces. Only returns a value above the chosen confidence floor. */
export function summarizeVersionSignalForUi(
  insights: FileInsights | undefined,
  minConfidence = 0.8,
): string | null {
  const best = insights?.versionSignals?.find((signal) => signal.confidence >= minConfidence);
  if (!best) return null;
  const value = (best.normalizedValue || best.rawValue || "").trim();
  if (!value) return null;
  return /^[0-9]/.test(value) ? `v${value}` : value;
}

function uniquePreviewTokens(values: Array<string | null | undefined>): string[] {
  const seen = new Set<string>();

  return values.reduce<string[]>((tokens, value) => {
    const trimmed = value?.trim();
    if (!trimmed) {
      return tokens;
    }

    const key = trimmed.toLowerCase();
    if (seen.has(key)) {
      return tokens;
    }

    seen.add(key);
    tokens.push(trimmed);
    return tokens;
  }, []);
}

function formatPreviewVersionToken(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  if (!trimmed) {
    return null;
  }

  return /^[0-9]/.test(trimmed) ? `v${trimmed}` : trimmed;
}

export function buildInspectorPreviewStrip(
  model: FileDetail,
  userView: UserView,
): {
  leftTokens: string[];
  rightToken: string | null;
  summaryLabel: string;
} {
  const insights = model.insights;
  const resourceSummary = uniquePreviewTokens(insights?.resourceSummary ?? []);
  const scriptNamespaces = uniquePreviewTokens(insights?.scriptNamespaces ?? []);
  const embeddedNames = uniquePreviewTokens(describeEmbeddedNames(insights));
  const familyHints = uniquePreviewTokens(
    (insights?.familyHints ?? []).map((hint) => formatLibraryFamilyHintLabel(hint)),
  );
  const subtype = model.subtype?.trim() || null;
  const primaryResource = describeResourceSummary(resourceSummary[0] ?? '');
  const bestVersion =
    summarizeVersionSignalForUi(insights, 0.55) ??
    formatPreviewVersionToken(insights?.versionHints?.[0] ?? null);
  const strongVersion = summarizeVersionSignalForUi(insights, 0.8);

  let leftCandidates: string[] = [];
  let rightToken: string | null = null;

  switch (model.kind) {
    case "CAS":
      leftCandidates = embeddedNames.slice(0, 3);
      rightToken = primaryResource ?? subtype;
      break;

    case "ScriptMods":
      leftCandidates = scriptNamespaces.slice(0, 2);
      rightToken = bestVersion ?? primaryResource;
      break;

    case "BuildBuy":
      leftCandidates = resourceSummary.slice(0, 1).map((r) => describeResourceSummary(r)).filter((r): r is string => Boolean(r));
      rightToken = familyHints[0] ?? null;
      break;

    case "Overrides":
    case "OverridesAndDefaults":
      leftCandidates = resourceSummary.slice(0, 1).map((r) => describeResourceSummary(r)).filter((r): r is string => Boolean(r));
      rightToken = bestVersion;
      break;

    case "PosesAndAnimation":
      leftCandidates = uniquePreviewTokens([subtype, primaryResource]).slice(0, 2);
      rightToken = familyHints[0] ?? null;
      break;

    case "PresetsAndSliders":
      leftCandidates = uniquePreviewTokens([subtype, primaryResource]).slice(0, 2);
      rightToken = familyHints[0] ?? null;
      break;

    case "TrayHousehold":
    case "TrayLot":
    case "TrayRoom":
    case "TrayItem":
    case "Household":
    case "Lot":
    case "Room":
      leftCandidates = (familyHints.length ? familyHints : embeddedNames).slice(0, 2);
      rightToken = primaryResource;
      break;

    default:
      leftCandidates = resourceSummary.slice(0, 1).map((r) => describeResourceSummary(r)).filter((r): r is string => Boolean(r));
      rightToken = strongVersion;
      break;
  }

  const fallbackLeftTokens = uniquePreviewTokens([
    ...leftCandidates,
    ...(leftCandidates.length ? [] : [rightToken]),
    subtype,
    primaryResource,
    familyHints[0] ?? null,
  ]);
  const leftLimit = userView === "power" ? 4 : 3;
  const leftTokens = fallbackLeftTokens.slice(0, leftLimit);
  const dedupedRightToken =
    rightToken && !leftTokens.some((token) => token.toLowerCase() === rightToken.toLowerCase())
      ? rightToken
      : null;
  const summaryTokens = leftTokens.length
    ? leftTokens.slice(0, 3)
    : uniquePreviewTokens([dedupedRightToken, subtype, primaryResource]).slice(0, 3);

  return {
    leftTokens,
    rightToken: dedupedRightToken,
    summaryLabel: summaryTokens.length ? `Contains: ${summaryTokens.join(", ")}` : "",
  };
}

// ─── Tray identity ─────────────────────────────────────────────────────────────
// Used to surface tray-specific file identity in rows, inspector, and sheet.
// ──────────────────────────────────────────────────────────────────────────────

export type TrayKind = "household" | "lot" | "room" | "item" | "standard";

export interface TrayIdentity {
  kind: TrayKind;
  label: string;
  /** Optional suffix — usually the family hint for tray items */
  suffix: string | null;
  /** Where this file lives */
  location: string;
  /** Whether this tray item is outside its proper folder */
  isMisplaced: boolean;
  evidenceKind: EvidenceKind;
}

/**
 * Describes the tray identity of a file.
 * Returns structured identity facts for tray kinds, or a standard-mod descriptor.
 * Never makes up metadata — only surfaces what the file actually carries.
 */
export function describeTrayIdentity(
  file: Pick<FileDetail, "kind" | "subtype" | "sourceLocation"> & { insights?: FileInsights },
): TrayIdentity {
  const isTray = file.sourceLocation === "tray";
  switch (file.kind) {
    case "TrayHousehold":
    case "Household":
      return {
        kind: "household",
        label: "Household",
        suffix: file.insights?.familyHints?.[0] ?? null,
        location: file.sourceLocation,
        isMisplaced: !isTray,
        evidenceKind: file.insights?.familyHints?.length ? "derived" : "inferred",
      };
    case "TrayLot":
    case "Lot":
      return {
        kind: "lot",
        label: "Lot",
        suffix: file.insights?.familyHints?.[0] ?? null,
        location: file.sourceLocation,
        isMisplaced: !isTray,
        evidenceKind: file.insights?.familyHints?.length ? "derived" : "inferred",
      };
    case "TrayRoom":
    case "Room":
      return {
        kind: "room",
        label: "Room",
        suffix: file.insights?.familyHints?.[0] ?? null,
        location: file.sourceLocation,
        isMisplaced: !isTray,
        evidenceKind: file.insights?.familyHints?.length ? "derived" : "inferred",
      };
    case "TrayItem":
      return {
        kind: "item",
        label: "Item",
        suffix: file.insights?.familyHints?.[0] ?? null,
        location: file.sourceLocation,
        isMisplaced: !isTray,
        evidenceKind: file.insights?.familyHints?.length ? "derived" : "inferred",
      };
    default:
      return {
        kind: "standard",
        label: "",
        suffix: null,
        location: file.sourceLocation,
        isMisplaced: false,
        evidenceKind: "inferred",
      };
  }
}

/** Returns a human-readable location label */
export function trayLocationLabel(location: string): string {
  return location === "tray" ? "Stored in Tray" : "Stored in Mods";
}

export function groupedFilesLabel(count?: number | null): string | null {
  if (!count || count <= 1) {
    return null;
  }
  return `${count} grouped files`;
}

/**
 * Generic folder names to exclude from the folder-heuristic signal.
 * These are too common to convey useful grouping information.
 */
const GENERIC_FOLDER_NAMES = new Set([
  "",
  ".",
  "tmp",
  "temp",
  "download",
  "downloads",
  "desktop",
  "documents",
  "mods",
  " tray",
]);

/**
 * Compute the primary relationship for a file given the current filtered items list.
 * Returns the single highest-confidence relationship signal, or null.
 *
 * Priority (highest first):
 *   duplicate → same_pack → tray_group → same_folder → folder_heuristic → none
 *
 * NOTE: This function is O(n) in the items array. Callers must ensure the items
 * list is stable (useMemo with items.length or JSON.stringify(items) as key).
 * Do NOT call on every render without memoization — for large collections this
 * will block the main thread.
 */
export function computeFileRelationship(
  file: Pick<LibraryFileRow, "id" | "path" | "bundleName" | "hasDuplicate" | "groupedFileCount" | "creator" | "sourceLocation" | "sameFolderPeerCount" | "samePackPeerCount">,
  items: Pick<LibraryFileRow, "id" | "path" | "bundleName" | "hasDuplicate" | "groupedFileCount" | "creator" | "sourceLocation" | "sameFolderPeerCount" | "samePackPeerCount">[],
): FileRelationship | null {
  if (file.hasDuplicate) {
    const duplicateCount = items.filter((f) => f.id !== file.id && f.hasDuplicate).length;
    return {
      type: "duplicate",
      proofLevel: "fact",
      label: `${Math.max(duplicateCount + 1, 2)} duplicate${duplicateCount + 1 !== 1 ? "s" : ""}`,
      peerCount: duplicateCount,
    };
  }

  if (file.bundleName) {
    const peerCount = file.samePackPeerCount ?? null;
    if (peerCount !== null && peerCount > 0) {
      return {
        type: "same_pack",
        proofLevel: "claim",
        label: `${peerCount} more in same pack`,
        peerCount,
      };
    }

    if (items.length > 0) {
      const bundlePeers = items.filter((f) => f.id !== file.id && f.bundleName === file.bundleName);
      if (bundlePeers.length > 0) {
        return {
          type: "same_pack",
          proofLevel: "claim",
          label: `${bundlePeers.length} more in same pack`,
          peerCount: bundlePeers.length,
        };
      }
    }
  }

  if (file.groupedFileCount != null && file.groupedFileCount > 1) {
    return {
      type: "tray_group",
      proofLevel: "claim",
      label: `${file.groupedFileCount} tray files`,
      peerCount: file.groupedFileCount - 1,
    };
  }

  if (file.creator?.trim() && items.length > 0) {
    const creatorPeers = items.filter(
      (f) => f.id !== file.id && f.creator?.trim() && f.creator === file.creator,
    );
    if (creatorPeers.length > 0) {
      return {
        type: "same_creator",
        proofLevel: "claim",
        label: `Same creator · ${file.creator}`,
        peerCount: creatorPeers.length,
      };
    }
  }

  const peerCount = file.sameFolderPeerCount ?? null;
  if (peerCount !== null && peerCount > 0) {
    const isMods = file.sourceLocation === "mods";
    return {
      type: isMods ? "same_folder" : "folder_heuristic",
      proofLevel: isMods ? "fact" : "heuristic",
      label: `${peerCount + 1} in this folder`,
      peerCount,
    };
  }

  const parentPath = extractParentPath(file.path);
  const parentFolder = extractParentFolder(file.path);
  if (parentPath && parentFolder && !GENERIC_FOLDER_NAMES.has(parentFolder.toLowerCase()) && items.length > 0) {
    const isMods = file.sourceLocation === "mods";
    const folderPeers = items.filter(
      (f) => f.id !== file.id && extractParentPath(f.path) === parentPath,
    );
    if (folderPeers.length > 0) {
      return {
        type: isMods ? "same_folder" : "folder_heuristic",
        proofLevel: isMods ? "fact" : "heuristic",
        label: `${folderPeers.length + 1} in ${parentFolder}`,
        peerCount: folderPeers.length,
      };
    }
  }

  return null;
}

export function extractParentPath(filePath: string): string | null {
  if (!filePath) return null;
  const normalized = filePath.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash <= 0) return null;
  return normalized.slice(0, lastSlash) || null;
}

/**
 * Extract the parent folder name from a file path.
 * Handles both forward slashes and backslashes.
 * Returns the last non-empty segment of the directory portion.
 */
export function extractParentFolder(filePath: string): string | null {
  if (!filePath) return null;
  const normalized = filePath.replace(/\\/g, "/");
  const lastSlash = normalized.lastIndexOf("/");
  if (lastSlash <= 0) return null;
  return normalized.slice(0, lastSlash).split("/").pop() ?? null;
}

export function describeTraySummary(
  file: Pick<FileDetail, "kind" | "subtype" | "sourceLocation" | "bundleName" | "groupedFileCount" | "insights">,
): string | null {
  const trayIdentity = describeTrayIdentity(file);
  if (trayIdentity.kind === "standard") {
    return null;
  }

  const kindLabel = trayKindLabel(trayIdentity.kind);
  const kindPhrase = `Classified as ${kindLabel}`;
  const groupingValue = usefulTrayGroupingValue(file);

  const groupedCountLabel = groupedFilesLabel(file.groupedFileCount);

  if (trayIdentity.isMisplaced) {
    if (groupingValue && groupedCountLabel) {
      return `${kindPhrase}, grouped as ${groupingValue}, is in a tray set with ${groupedCountLabel} and is sitting in Mods, so it needs review.`;
    }
    return groupingValue
      ? `${kindPhrase}, grouped as ${groupingValue}, is sitting in Mods and needs review.`
      : `${kindPhrase}. This tray content is sitting in Mods and needs review.`;
  }

  if (groupingValue && groupedCountLabel) {
    return `${kindPhrase}, grouped as ${groupingValue}, is in a tray set with ${groupedCountLabel} and is stored in Tray as library content.`;
  }

  return groupingValue
    ? `${kindPhrase}, grouped as ${groupingValue}, is stored in Tray as library content.`
    : `${kindPhrase}. It is stored in Tray as library content, not an active mod.`;
}

/** Returns a human-readable label for what the item is, e.g. "Household" or "Lot" */
export function trayKindLabel(kind: TrayKind): string {
  switch (kind) {
    case "household":
      return "Household";
    case "lot":
      return "Lot";
    case "room":
      return "Room";
    case "item":
      return "Tray Item";
    default:
      return "";
  }
}

function isPathLikeValue(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return false;
  return (
    /^[a-zA-Z]:[\\/]/.test(trimmed) ||
    trimmed.startsWith("/mnt/") ||
    trimmed.startsWith("/home/") ||
    trimmed.startsWith("\\\\") ||
    trimmed.includes("\\") ||
    trimmed.includes("/")
  );
}

export function usefulTrayGroupingValue(
  file: { bundleName?: string | null; insights?: FileInsights },
): string | null {
  const bundleName = file.bundleName?.trim();
  if (bundleName && !isPathLikeValue(bundleName)) {
    return bundleName;
  }

  const familyHint = (file.insights?.familyHints ?? [])
    .map((item) => item.trim())
    .find((item) => item && !isPathLikeValue(item));

  return familyHint ?? null;
}

export function buildSheetTraySection(
  file: Pick<
    FileDetail,
    "kind" | "subtype" | "sourceLocation" | "insights" | "bundleName" | "bundleType" | "groupedFileCount" | "safetyNotes"
  >,
  userView: "beginner" | "standard" | "power",
): ReactNode {
  const trayIdentity = describeTrayIdentity(file);

  if (trayIdentity.kind === "standard") {
    return null;
  }

  const groupingValue = usefulTrayGroupingValue(file);
  const relatedHints = [
    ...(groupingValue ? [groupingValue] : []),
    ...(file.insights?.familyHints ?? [])
      .map((item) => item.trim())
      .filter((item) => item && item !== groupingValue && !isPathLikeValue(item)),
  ].slice(0, 6);

  return (
    <div className="detail-list">
      <div className="detail-row">
        <span>
          Tray type
          <span className="detail-row-evidence-badge">Inferred</span>
        </span>
        <strong>{trayKindLabel(trayIdentity.kind)}</strong>
      </div>
      <div className="detail-row">
        <span>
          Stored
          <span className="detail-row-evidence-badge">Inferred</span>
        </span>
        <strong>{trayLocationLabel(trayIdentity.location)}</strong>
      </div>
      {groupingValue ? (
        <div className="detail-row">
          <span>
            Grouped as
            <span className="detail-row-evidence-badge">Derived</span>
          </span>
          <strong>{groupingValue}</strong>
        </div>
      ) : null}
      {file.bundleType?.trim() ? (
        <div className="detail-row">
          <span>
            Tray group
            <span className="detail-row-evidence-badge">Derived</span>
          </span>
          <strong>{file.bundleType}</strong>
        </div>
      ) : null}
      {groupedFilesLabel(file.groupedFileCount) ? (
        <div className="detail-row">
          <span>
            Tray set
            <span className="detail-row-evidence-badge">Derived</span>
          </span>
          <strong>{groupedFilesLabel(file.groupedFileCount)}</strong>
        </div>
      ) : null}
      <div className="detail-row detail-row--block">
        <span>Tray summary</span>
        <strong>
          {describeTraySummary(file) ??
            (trayIdentity.isMisplaced
              ? "Tray content detected outside Tray. Review before moving or trusting it."
              : "Library object only. It stays in Tray and does not behave like an active mod.")}
        </strong>
      </div>
      {relatedHints.length > 0 && userView !== "beginner" ? (
        <div className="detail-row detail-row--block">
          <span>
            Grouping hint
            <span className="detail-row-evidence-badge">
              {trayIdentity.evidenceKind === "derived" ? "Derived" : "Inferred"}
            </span>
          </span>
          <div className="tag-list">
            {relatedHints.map((item) => (
              <span key={item} className="ghost-chip">
                {item}
              </span>
            ))}
          </div>
        </div>
      ) : null}
      {file.safetyNotes.length > 0 ? (
        <div className="detail-row detail-row--block">
          <span>Tray notes</span>
          <div className="tag-list">
            {file.safetyNotes.map((note) => (
              <span key={note} className="warning-tag">
                {note}
              </span>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}

// ─── Evidence-kind labeling ────────────────────────────────────────────────────
// Used to label sheet section content with its evidence origin.
// ──────────────────────────────────────────────────────────────────────────────
export type EvidenceKind = "extracted" | "derived" | "inferred";

/** Returns an evidence-kind badge label */
export function evidenceKindLabel(kind: EvidenceKind): string {
  switch (kind) {
    case "extracted":
      return "Extracted";
    case "derived":
      return "Derived";
    case "inferred":
      return "Inferred";
  }
}

/** Maps a field name to its evidence kind for the sheet sections */
export function fieldEvidenceKind(
  field: "embeddedNames" | "scriptNamespaces" | "resourceSummary" | "familyHints" | "creatorHints" | "versionSignals" | "subtype" | "kind" | "creator",
): EvidenceKind {
  switch (field) {
    case "embeddedNames":
    case "scriptNamespaces":
    case "resourceSummary":
      return "extracted";
    case "familyHints":
    case "creatorHints":
      return "derived";
    case "versionSignals":
    case "subtype":
    case "kind":
    case "creator":
      return "inferred";
  }
}

// ─── Sheet section content builders ───────────────────────────────────────────
// These helpers build the content for each sheet section.
// Each returns a ReactNode block suitable for a DockSectionDefinition.children.
// ──────────────────────────────────────────────────────────────────────────────

/** Section: Attribution & Tracking — creator evidence + family grouping */
export function buildSheetAttributionSection(
  file: FileDetail,
  userView: "beginner" | "standard" | "power",
): ReactNode {
  const creatorTier = creatorConfidenceTier(
    file.creator,
    file.creatorLearning ?? null,
    file.insights?.creatorHints ?? [],
  );
  const creatorLabel = creatorLabelWithConfidence(
    file.creator,
    file.creatorLearning ?? null,
    file.insights?.creatorHints ?? [],
  );

  // Evidence list: what signals drove the creator conclusion
  const evidenceItems: Array<{ label: string; kind: EvidenceKind }> = [];

  if (file.insights?.creatorHints?.length) {
    file.insights.creatorHints.forEach((hint) => {
      evidenceItems.push({ label: hint, kind: "derived" });
    });
  }
  if (file.creatorLearning?.learnedAliases?.length) {
    file.creatorLearning.learnedAliases.forEach((alias) => {
      evidenceItems.push({ label: alias, kind: "derived" });
    });
  }
  // Path folder as weakest signal
  if (file.path.includes("/Mods/") || file.path.includes("/Downloads/")) {
    const folder = file.path.split("/").filter(Boolean).slice(-2).join("/");
    if (folder) {
      evidenceItems.push({ label: `Folder: ${folder}`, kind: "inferred" });
    }
  }

  // Family grouping
  const familyItems = (file.insights?.familyHints ?? [])
    .filter((f) => f.trim())
    .slice(0, 6);

  return (
    <div className="detail-list">
      {/* Creator attribution */}
      <div className="detail-row">
        <span>Creator</span>
        <strong>
          {creatorLabel}
          <span
            className="detail-row-suffix"
            title={`Creator ${creatorConfidenceTier(
              file.creator,
              file.creatorLearning ?? null,
              file.insights?.creatorHints ?? [],
            ) === "known" ? "saved" : creatorTier === "strong_hint" ? "from file" : "from folder"}`}
          >
            {" "}
            (
            {creatorTier === "known"
              ? "saved"
              : creatorTier === "strong_hint"
                ? "from file"
                : "from folder"}
            )
          </span>
        </strong>
      </div>

      {/* Family grouping */}
      {familyItems.length > 0 ? (
        <div className="detail-row detail-row--block">
          <span>Family / Set</span>
          <div className="tag-list">
            {familyItems.map((f) => (
              <span key={f} className="ghost-chip">
                {f}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {/* Creator evidence signals */}
      {evidenceItems.length > 0 && userView !== "beginner" ? (
        <div className="detail-row detail-row--block">
          <span>Evidence</span>
          <div className="tag-list">
            {evidenceItems.slice(0, 8).map((item) => (
              <span key={item.label} className={`ghost-chip evidence-${item.kind}`}>
                {item.label}
              </span>
            ))}
            {evidenceItems.length > 8 ? (
              <span className="ghost-chip-inline-button">+{evidenceItems.length - 8} more</span>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/**
 * Returns a short, simmer-facing content label for package files.
 * Uses extracted summary strings when they are already readable, otherwise falls back
 * to kind/subtype wording that stays honest and low-jargon.
 */
export function summarizePackageContentProfile(
  insights: FileInsights | undefined,
  kind: string | null,
  subtype: string | null,
): string | null {
  const resource = insights?.resourceSummary ?? [];
  const primary = resource.find(
    (item) => item.trim() && !/other resource/i.test(item) && !/compressed resource/i.test(item),
  );

  // Humanize before returning so raw DBPF strings don't leak into UI
  const humanizedPrimary = primary ? describeResourceSummary(primary) : null;

  if (kind === "BuildBuy") {
    return humanizedPrimary ?? "Build/Buy content";
  }
  if (kind === "OverridesAndDefaults") {
    return humanizedPrimary ?? "Override package";
  }
  if (kind === "CAS") {
    return subtype?.trim() ?? humanizedPrimary ?? "CAS content";
  }
  if (kind === "PresetsAndSliders") {
    return subtype?.trim() ?? humanizedPrimary ?? "Preset or slider";
  }
  if (kind === "Gameplay") {
    return subtype?.trim() ?? humanizedPrimary ?? "Gameplay package";
  }

  return humanizedPrimary ?? subtype?.trim() ?? null;
}

/** Section: What's Inside — extracted file contents, ordered by type-specific usefulness */
export function buildSheetContentsSection(
  file: FileDetail,
  userView: "beginner" | "standard" | "power",
): ReactNode {
  const embedded = describeEmbeddedNames(file.insights);
  const namespaces = describeScriptNamespaces(file.insights);
  const resource = file.insights?.resourceSummary ?? [];
  const isScriptMod = file.kind === "ScriptMods";
  // CAS: detected by kind or resource type signatures in the file's extracted content
  const isCas =
    file.kind === "CAS" ||
    (file.insights?.resourceSummary ?? []).some((r) =>
      ["CASPart", "Skintone", "Hair", "Tops", "Bottoms", "Dress", "Shoes", "Accessory", "Outfit"].some(
        (t) => r.includes(t),
      ),
    );

  // Nothing to show?
  if (!embedded.length && !namespaces.count && !resource.length) {
    return (
      <p className="text-muted">No extracted content details available for this file.</p>
    );
  }

  return (
    <div className="detail-list">
      {/*
       * Content order is type-dependent:
       * - ScriptMods: Namespaces first (primary signal), skip redundant "Contains"
       * - CAS: Included names first (actual item names), then resource profile
       * - Other packages: resource profile first, then names
       */}

      {/* For ScriptMods: Namespaces — the primary content signal */}
      {namespaces.count > 0 ? (
        <div className="detail-row detail-row--block">
          <span>
            Namespaces
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <div className="tag-list">
            {namespaces.samples.map((ns) => (
              <span key={ns} className="ghost-chip">
                {ns}
              </span>
            ))}
            {namespaces.count > 5 ? (
              <span className="ghost-chip-inline-button">+{namespaces.count - 5} more</span>
            ) : null}
          </div>
        </div>
      ) : null}

      {/*
       * "Contains" — shown only for non-ScriptMods, non-CAS packages.
       * CAS shows it after "Included names" (see below).
       * Hidden entirely for ScriptMods (namespace section is more useful).
       */}
      {!isScriptMod && resource.length > 0 && !isCas ? (
        <div className="detail-row detail-row--block">
          <span>
            Contains
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <div className="tag-list">
            {resource.slice(0, 5).map((r) => (
              <span key={r} className="ghost-chip">
                {r}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {/*
       * "Included names" — order depends on file type:
       * - CAS: shown FIRST (item names like "NSW_Skinblend" are the most useful signal)
       * - ScriptMods: shown last (module stems are secondary to namespaces)
       * - Other packages: shown after content profile
       * Hidden in beginner view for ScriptMods and generic packages.
       */}
      {embedded.length > 0 && userView !== "beginner" && isCas ? (
        <div className="detail-row detail-row--block">
          <span>
            Included names
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <div className="tag-list">
            {embedded.slice(0, 8).map((name) => (
              <span key={name} className="ghost-chip">
                {name}
              </span>
            ))}
            {embedded.length > 8 ? (
              <span className="ghost-chip-inline-button">+{embedded.length - 8} more</span>
            ) : null}
          </div>
        </div>
      ) : null}

      {/*
       * "Contains" — shown for CAS after "Included names", since raw counts
       * are secondary to the item names themselves.
       */}
      {!isScriptMod && resource.length > 0 && isCas ? (
        <div className="detail-row detail-row--block">
          <span>
            Contains
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <div className="tag-list">
            {resource.slice(0, 5).map((r) => (
              <span key={r} className="ghost-chip">
                {r}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {/*
       * "Included names" — for non-CAS types, shown after the content profile.
       * Hidden in beginner view.
       */}
      {embedded.length > 0 && userView !== "beginner" && !isCas ? (
        <div className="detail-row detail-row--block">
          <span>
            Included names
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <div className="tag-list">
            {embedded.slice(0, 8).map((name) => (
              <span key={name} className="ghost-chip">
                {name}
              </span>
            ))}
            {embedded.length > 8 ? (
              <span className="ghost-chip-inline-button">+{embedded.length - 8} more</span>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** Section: Compatibility & Health — version signals and warnings */
export function buildSheetCompatibilitySection(
  file: FileDetail,
  mode: "inspect" | "health",
  userView: "beginner" | "standard" | "power",
): ReactNode {
  const versionInfo = file.installedVersionSummary;
  const watch = file.watchResult;
  const hasWarnings =
    file.safetyNotes.length > 0 || file.parserWarnings.length > 0;

  return (
    <div className="detail-list">
      {/* Version tracking */}
      {watch ? (
        <div className="detail-row">
          <span>Watch status</span>
          <strong
            className={`library-health-pill is-${watch.status === "current" ? "calm" : watch.status === "exact_update_available" ? "attention" : "muted"}`}
          >
            {watch.status === "current"
              ? "Up to date"
              : watch.status === "exact_update_available"
                ? "Update available"
                : watch.status === "possible_update"
                  ? "May have update"
                  : watch.status === "unknown"
                    ? "Check updates"
                    : "Not tracked"}
          </strong>
        </div>
      ) : null}

      {/* Installed version with confidence */}
      {versionInfo?.version ? (
        <div className="detail-row">
          <span>Installed</span>
          <strong>
            {versionInfo.version}
            {versionInfo.confidence !== "exact" && versionInfo.confidence !== "strong" ? (
              <span
                className="detail-row-suffix"
                title={`Version ${versionInfo.confidence}`}
              >
                {" "}
                ({versionInfo.confidence === "medium" ? "Possible" : versionInfo.confidence === "weak" ? "Speculative" : "Unconfirmed"})
              </span>
            ) : null}
          </strong>
        </div>
      ) : null}

      {/* Version signals */}
      {file.insights?.versionSignals?.length && userView !== "beginner" ? (
        <div className="detail-row detail-row--block">
          <span>
            Version evidence
            <span className="detail-row-evidence-badge">Inferred</span>
          </span>
          <div className="tag-list">
            {file.insights.versionSignals.slice(0, 6).map((sig) => (
              <span key={sig.rawValue} className="ghost-chip">
                {sig.normalizedValue || sig.rawValue}
              </span>
            ))}
            {file.insights.versionSignals.length > 6 ? (
              <span className="ghost-chip-inline-button">
                +{file.insights.versionSignals.length - 6} more
              </span>
            ) : null}
          </div>
        </div>
      ) : null}

      {/* Warnings — always show in health mode */}
      {(hasWarnings || mode === "health") && userView !== "beginner" ? (
        <>
          {file.safetyNotes.length > 0 ? (
            <div className="detail-row detail-row--block">
              <span>Safety notes</span>
              <div className="tag-list">
                {file.safetyNotes.map((note) => (
                  <span key={note} className="warning-tag">
                    {note}
                  </span>
                ))}
              </div>
            </div>
          ) : null}
          {file.parserWarnings.length > 0 ? (
            <div className="detail-row detail-row--block">
              <span>Parser warnings</span>
              <div className="tag-list">
                {file.parserWarnings.slice(0, 5).map((warn) => (
                  <span key={warn} className="ghost-chip">
                    {warn}
                  </span>
                ))}
                {file.parserWarnings.length > 5 ? (
                  <span className="ghost-chip-inline-button">
                    +{file.parserWarnings.length - 5} more
                  </span>
                ) : null}
              </div>
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  );
}

/** Section: File & Diagnostics — path, size, structure */
export function buildSheetDiagnosticsSection(
  file: FileDetail,
  userView: "beginner" | "standard" | "power",
): ReactNode {
  if (userView === "beginner") {
    return null; // not shown in Casual
  }

  const sizeLabel =
    file.size > 1024 * 1024
      ? `${(file.size / (1024 * 1024)).toFixed(1)} MB`
      : file.size > 1024
        ? `${(file.size / 1024).toFixed(0)} KB`
        : `${file.size} B`;

  const modifiedLabel = file.modifiedAt
    ? new Date(file.modifiedAt).toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      })
    : "Unknown";

  return (
    <div className="detail-list">
      <div className="detail-row">
        <span>Location</span>
        <strong className="mono-text" style={{ fontSize: "0.75rem" }}>
          {file.path}
        </strong>
      </div>
      <div className="detail-row">
        <span>Size</span>
        <strong>{sizeLabel}</strong>
      </div>
      <div className="detail-row">
        <span>Last modified</span>
        <strong>{modifiedLabel}</strong>
      </div>
      <div className="detail-row">
        <span>Format</span>
        <strong>{formatLibraryFileFormat(file)}</strong>
      </div>
      {file.hash ? (
        <div className="detail-row detail-row--block">
          <span>
            Fingerprint
            <span className="detail-row-evidence-badge">Extracted</span>
          </span>
          <strong className="mono-text" style={{ fontSize: "0.7rem", wordBreak: "break-all" }}>
            {file.hash.slice(0, 16)}...
          </strong>
        </div>
      ) : null}
      {file.sourceLocation === "tray" ? (
        <div className="detail-row">
          <span>Status</span>
          <span className="library-health-pill is-muted">Disabled (in tray)</span>
        </div>
      ) : null}
    </div>
  );
}

/** Builds type-specific supporting facts for a library row */
function buildSupportingFacts(
  row: LibraryFileRow,
  {
    flags,
    creatorLabel,
    isTray,
    userView,
  }: {
    flags: LibraryViewFlags;
    creatorLabel: string;
    isTray: boolean;
    userView: UserView;
  },
): string[] {
  const facts: string[] = [];
  const kind = row.kind;

  if (!flags.showCreatorInList) return facts;

  switch (kind) {
    // CAS: subtype is usually the quickest clue, with content profile as fallback.
    case "CAS": {
      const profile = summarizePackageContentProfile(row.insights, row.kind, row.subtype);
      if (row.subtype?.trim()) {
        facts.push(row.subtype.trim());
      } else if (profile) {
        facts.push(profile);
      }
      facts.push(creatorLabel);
      break;
    }

    // Script mods: creator + namespace + best version clue.
    // When namespace is missing, prefer showing the version clue (version found
    // in file content) rather than a generic confidence label.
    case "ScriptMods":
      facts.push(creatorLabel);
      {
        const scope = summarizeScriptScopeForUi(row.insights, userView);
        if (scope) {
          facts.push(scope);
        } else {
          // No namespace found — show version clue if one exists, otherwise
          // fall back to a confidence label that describes what we know.
          const versionClue = summarizeVersionSignalForUi(row.insights, 0.55);
          if (versionClue) {
            facts.push(versionClue);
          } else if (row.confidence != null) {
            facts.push(
              row.confidence >= 0.8
                ? "Strong file clue"
                : row.confidence >= 0.55
                  ? "Possible version clue"
                  : "Very weak version clue",
            );
          }
        }
      }
      break;

    // Gameplay: subtype first, package profile second when available.
    case "Gameplay": {
      const profile = summarizePackageContentProfile(row.insights, row.kind, row.subtype);
      facts.push(creatorLabel);
      if (row.subtype?.trim()) {
        facts.push(row.subtype.trim());
      } else if (profile) {
        facts.push(profile);
      }
      break;
    }

    // Build/Buy: content profile matters more than folder location.
    case "BuildBuy": {
      const profile = summarizePackageContentProfile(row.insights, row.kind, row.subtype);
      if (profile) facts.push(profile);
      facts.push(creatorLabel);
      if (isTray) facts.push("🔖 In tray");
      break;
    }

    // Overrides: call the package what it is, then show who it came from.
    case "OverridesAndDefaults": {
      const profile = summarizePackageContentProfile(row.insights, row.kind, row.subtype);
      facts.push(profile ?? "Override package");
      facts.push(creatorLabel);
      break;
    }

    // Poses and Animation: show creator + subtype (pose pack, animation set, etc.)
    case "PosesAndAnimation":
      facts.push(creatorLabel);
      if (row.subtype?.trim()) facts.push(row.subtype.trim());
      break;

    // Presets and Sliders: show creator + body area (from subtype if available)
    case "PresetsAndSliders":
      facts.push(creatorLabel);
      if (row.subtype?.trim()) facts.push(row.subtype.trim());
      break;

    // Tray items: show tray badge + creator
    case "TrayHousehold":
    case "TrayLot":
    case "TrayRoom":
    case "TrayItem":
    case "Household":
    case "Lot":
    case "Room":
      const groupingValue = usefulTrayGroupingValue({
        bundleName: row.bundleName ?? null,
        insights: row.insights,
      });
      if (groupingValue) facts.push(groupingValue);
      const groupedCount = groupedFilesLabel(row.groupedFileCount);
      if (groupedCount && flags.showInspectFactsInList) facts.push(groupedCount);
      facts.push(isTray ? "Stored in Tray" : "Stored in Mods");
      // Creator is always the most important signal for tray items.
      if (row.creator?.trim()) facts.push(creatorLabel);
      break;

    // Unknown: show source + creator
    default:
      if (flags.showRootFacts) {
        facts.push(isTray ? "🔖 In tray" : "Mods");
      }
      if (flags.showCreatorInList) facts.push(creatorLabel);
      break;
  }

  return facts.filter((f): f is string => Boolean(f));
}

/**
 * Returns the most significant issue for a library item, or null if the item is fine.
 * Priority: safety note > tray > parser warning.
 */
function computeLibraryHealthIssue(
  row: Pick<LibraryFileRow, "safetyNotes" | "parserWarnings" | "sourceLocation">,
): { label: string; tone: "attention" | "muted" } | null {
  if (row.safetyNotes.length > 0) {
    return { label: "Needs review", tone: "attention" };
  }
  if (row.sourceLocation === "tray") {
    return { label: "Disabled", tone: "muted" };
  }
  if (row.parserWarnings.length > 0) {
    return { label: "Warning", tone: "muted" };
  }
  return null;
}

function describeWatchStatus(watchStatus?: WatchStatus): string {
  switch (watchStatus) {
    case "current":
      return "Up to date";
    case "exact_update_available":
      return "Update available";
    case "possible_update":
      return "May have update";
    case "unknown":
      return "Check updates";
    case "not_watched":
    default:
      return "Not tracked";
  }
}

function watchStatusToneFor(
  watchStatus?: WatchStatus,
): "calm" | "attention" | "muted" {
  switch (watchStatus) {
    case "current":
      return "calm";
    case "exact_update_available":
      return "attention";
    case "possible_update":
      return "muted";
    case "unknown":
    case "not_watched":
    default:
      return "muted";
  }
}


// ─── Folder Summary & Tree Clue Types (Phase 5ao) ────────────────────────

/**
 * Folder summary computation.
 * Used by FolderSummaryPanel to display intelligence when a folder is selected.
 */

/** Folder summary computation mode. */
export type FolderSummaryMode = "casual" | "power";

type SheetRelationshipTier = "confirmed" | "likely" | "possible";

type SheetRelationshipCard = {
  key: string;
  tier: SheetRelationshipTier;
  label: string;
  reason: string;
  actionLabel?: string;
  onAction?: () => void;
};

export function buildSheetRelationshipsSection(
  file: FileDetail,
  allFiles: LibraryFileRow[],
  _userView: UserView,
): ReactNode {
  const cards: SheetRelationshipCard[] = [];
  const isScriptMod = file.kind.includes("Script") || /\.ts4script$/i.test(file.filename);
  const sameCreatorPeers = file.creator?.trim()
    ? allFiles.filter((candidate) => candidate.id !== file.id && candidate.creator === file.creator)
    : [];
  const parentPath = extractParentPath(file.path);
  const parentFolder = extractParentFolder(file.path);
  const folderPeers = parentPath
    ? allFiles.filter((candidate) => candidate.id !== file.id && extractParentPath(candidate.path) === parentPath)
    : [];
  const exactDuplicate = file.duplicateTypes.includes("exact");
  const versionDuplicate = file.duplicateTypes.includes("version");
  const trayBundle = file.sourceLocation === "tray" && (file.groupedFileCount ?? 0) > 1;

  if (exactDuplicate) {
    cards.push({
      key: "exact-duplicate",
      tier: "confirmed",
      label: "Exact duplicates detected",
      reason:
        file.duplicatesCount > 0
          ? `Duplicate detector found ${file.duplicatesCount} matching pair${file.duplicatesCount !== 1 ? "s" : ""} for this file.`
          : "Duplicate detector marked this file as an exact duplicate.",
    });
  }

  if (file.sourceLocation === "mods" && (file.sameFolderPeerCount ?? 0) > 0) {
    cards.push({
      key: "same-folder-mods",
      tier: "confirmed",
      label: "Same folder set",
      reason: `${(file.sameFolderPeerCount ?? 0) + 1} files live in ${parentFolder ?? "this folder"}. This confirms shared placement, not a dependency.`,
    });
  }

  if (trayBundle) {
    cards.push({
      key: "tray-bundle",
      tier: "confirmed",
      label: "Tray bundle",
      reason: `${file.groupedFileCount} tray files were grouped together from the same household / lot bundle.`,
    });
  }

  if (file.bundleName && (file.samePackPeerCount ?? 0) > 0) {
    cards.push({
      key: "same-pack",
      tier: "likely",
      label: "Same pack membership",
      reason: `${(file.samePackPeerCount ?? 0) + 1} files share the ${file.bundleName} pack grouping.`,
    });
  }

  if (sameCreatorPeers.length > 0) {
    cards.push({
      key: "same-creator",
      tier: "likely",
      label: "Same creator",
      reason: `${sameCreatorPeers.length} other file${sameCreatorPeers.length !== 1 ? "s" : ""} in this view are attributed to ${file.creator}.`,
    });
  }

  if (versionDuplicate) {
    cards.push({
      key: "version-related",
      tier: "likely",
      label: "Version-related duplicate",
      reason: "Duplicate detection flagged a version-related match. This may mean another copy or revision of the same mod is present.",
    });
  }

  if (file.sourceLocation !== "mods" && folderPeers.length > 0) {
    cards.push({
      key: "same-folder-possible",
      tier: "possible",
      label: "Shared folder placement",
      reason: `${folderPeers.length + 1} files sit in ${parentFolder ?? "the same folder"}. Outside Mods, this is only a possible relationship signal.`,
    });
  }

  const safeDeleteWarnings: string[] = [];
  if (exactDuplicate) {
    safeDeleteWarnings.push("This file has a duplicate clue. Compare the matching files before removing either copy.");
  }
  if (isScriptMod) {
    safeDeleteWarnings.push("This looks like a script mod. Some mods can rely on script files, so check the mod notes before disabling it.");
  }

  if (cards.length === 0 && safeDeleteWarnings.length === 0) {
    return null;
  }

  const renderTier = (tier: SheetRelationshipTier, title: string) => {
    const tierCards = cards.filter((card) => card.tier === tier);
    if (tierCards.length === 0) return null;
    return (
      <div className="detail-block">
        <div className="section-label">{title}</div>
        <div className="detail-list">
          {tierCards.map((card) => (
            <div key={card.key} className="detail-row detail-row--block">
              <span>{card.label}</span>
              <div>
                <strong>{card.reason}</strong>
                {card.actionLabel && card.onAction ? (
                  <div style={{ marginTop: "0.45rem" }}>
                    <button type="button" className="secondary-action" onClick={card.onAction}>
                      {card.actionLabel}
                    </button>
                  </div>
                ) : null}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  };

  return (
    <>
      {renderTier("confirmed", "Known file facts")}
      {renderTier("likely", "Likely related clues")}
      {cards.some((card) => card.tier === "possible") ? (
        <div className="detail-block">
          <div className="section-label">Possible placement clues</div>
          <p className="text-muted" style={{ marginTop: 0, marginBottom: "0.5rem" }}>
            Same-folder connections are based on file location. Many files in the same folder are unrelated, so this is a possible clue only.
          </p>
          <div className="detail-list">
            {cards.filter((card) => card.tier === "possible").map((card) => (
              <div key={card.key} className="detail-row detail-row--block">
                <span>{card.label}</span>
                <strong>{card.reason}</strong>
              </div>
            ))}
          </div>
        </div>
      ) : null}
      {safeDeleteWarnings.length > 0 ? (
        <div className="detail-block library-safedelete-warning">
          <div className="section-label">Check before removing</div>
          <div className="detail-list">
            {safeDeleteWarnings.map((warning) => (
              <div key={warning} className="detail-row detail-row--block">
                <span>Check first</span>
                <strong>{warning}</strong>
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </>
  );
}

/** Maps ProofLevel to user-facing confidence label. */
export function proofLevelToLabel(level: ProofLevel): "Confirmed" | "Likely" | "Possible" {
  switch (level) {
    case "fact":      return "Confirmed";
    case "claim":     return "Likely";
    case "heuristic": return "Possible";
  }
}

/** Counts for a single folder. */
export interface FolderCountSummary {
  totalFiles: number;
  directFiles: number;
  nestedFiles: number;
  subfolderCount: number;
  warningCount: number;
  duplicateCount: number;
  /** Files stored directly at depth-0 in the folder (not in subfolders). */
  rootFilesCount: number;
}

/** A single distribution entry (kind, source, or creator). */
export interface FolderDistributionItem {
  key: string;
  label: string;
  count: number;
  percentage: number;
}

/** A relationship cluster within a folder. */
export interface FolderRelationshipCluster {
  /** Stable id for rendering keys. */
  id: string;
  type: "duplicate" | "same_pack" | "same_folder";
  proofLevel: ProofLevel;
  confidenceLabel: "Confirmed" | "Likely" | "Possible";
  /** How many files in this folder belong to this cluster. */
  affectedFileCount: number;
  /** Peak peer count for same_folder / same_pack clusters. */
  peakPeerCount: number;
  title: string;
  description: string;
}

/** Full folder summary payload. */
export interface FolderSummaryData {
  folderId: string;
  folderName: string;
  folderPath: string;
  counts: FolderCountSummary;
  kindDistribution: FolderDistributionItem[];
  sourceDistribution: FolderDistributionItem[];
  creatorDistribution: FolderDistributionItem[];
  dominantKind?: string;
  dominantKindShare?: number;
  relationshipClusters: FolderRelationshipCluster[];
  notes: string[];
}

/** Per-tree-node clue data. */
export interface FolderTreeClue {
  dominantKind?: string;
  dominantKindShare?: number;
  warningCount: number;
  duplicateCount: number;
  issueState: "none" | "warning" | "duplicate" | "mixed";
}

/**
 * Derives the primary RelationshipCue from a FileRelationship.
 * Returns null when no significant relationship exists.
 */
export function deriveRelationshipCue(rel: FileRelationship | null): RelationshipCue | null {
  if (!rel || rel.type === "none") return null;
  const label = proofLevelToLabel(rel.proofLevel);
  const count = rel.peerCount ?? 0;
  switch (rel.type) {
    case "duplicate":
      return {
        type: rel.type,
        proofLevel: rel.proofLevel,
        confidenceLabel: label,
        shortLabel: `${label} · Duplicate`,
        compactLabel: "Dup",
        description: rel.label,
        relatedCount: count,
      };
    case "same_pack":
      return {
        type: rel.type,
        proofLevel: rel.proofLevel,
        confidenceLabel: label,
        shortLabel: `${label} · Pack with ${count + 1} files`,
        compactLabel: count > 0 ? `Pack ${count + 1}` : "Pack",
        description: rel.label,
        relatedCount: count,
      };
    case "same_folder":
      // Phase 5ak Sentinel challenge: same_folder IS a fact for mods source.
      // Folder structure = semantic packaging for mods. Only a heuristic for downloads/tray.
      return {
        type: rel.type,
        proofLevel: rel.proofLevel === "fact" ? "fact" : rel.proofLevel,
        confidenceLabel: rel.proofLevel === "fact" ? "Confirmed" : label,
        shortLabel: `${rel.proofLevel === "fact" ? "Confirmed" : label} · Folder set (${count + 1} peers)`,
        compactLabel: count > 0 ? `Set ${count + 1}` : "Set",
        description: rel.label,
        relatedCount: count,
      };
    case "folder_heuristic":
      return {
        type: rel.type,
        proofLevel: rel.proofLevel,
        confidenceLabel: label,
        shortLabel: `${label} · Folder set (${count + 1} peers)`,
        compactLabel: count > 0 ? `Set ${count + 1}` : "Set",
        description: rel.label,
        relatedCount: count,
      };
    case "tray_group":
      return {
        type: rel.type,
        proofLevel: rel.proofLevel,
        confidenceLabel: label,
        shortLabel: `${label} · Tray group`,
        compactLabel: "Tray",
        description: rel.label,
        relatedCount: count,
      };
    case "same_creator":
      return {
        type: rel.type,
        proofLevel: rel.proofLevel,
        confidenceLabel: label,
        shortLabel: `${label} · Same creator`,
        compactLabel: "Creator",
        description: rel.label,
        relatedCount: count,
      };
    default:
      return null;
  }
}

/**
 * Compute a FolderSummaryData from the files in a folder.
 * O(n) in folderFiles — caller must wrap in useMemo.
 *
 * @param folderFiles  All files in the folder (from folderContents.files).
 * @param folderNode   The FolderNode for this folder.
 */
export function computeFolderSummary(
  folderFiles: LibraryFileRow[],
  folderNode: { name: string; fullPath: string; childFolderCount: number; files?: number[] },
  /** Number of depth-0 files (stored directly in this folder, not in subfolders). */
  rootFilesCount: number = 0,
): FolderSummaryData {
  const totalFiles = folderFiles.length + rootFilesCount;
  // directFiles: files stored at this folder node (node.files IDs) + rootFilesCount for root folders.
  // For root folders (where node.files is undefined), rootFilesCount IS the direct file count.
  const nodeDirectCount = folderNode.files ? Math.min(folderNode.files.length, folderFiles.length) : 0;
  const directFiles = nodeDirectCount + rootFilesCount;
  const nestedFiles = Math.max(0, totalFiles - directFiles);

  // ── Kind distribution ──────────────────────────────────────────────────
  const kindMap = new Map<string, number>();
  for (const f of folderFiles) {
    kindMap.set(f.kind, (kindMap.get(f.kind) ?? 0) + 1);
  }
  const kindDistribution: FolderDistributionItem[] = Array.from(kindMap.entries())
    .map(([key, count]) => ({
      key,
      label: friendlyTypeLabel(key as LibraryFileRow["kind"]),
      count,
      percentage: totalFiles > 0 ? (count / totalFiles) * 100 : 0,
    }))
    .sort((a, b) => b.count - a.count);

  const dominantKind = kindDistribution[0];
  const dominantKindShare = dominantKind ? dominantKind.percentage : undefined;

  // ── Source distribution ────────────────────────────────────────────────
  const sourceMap = new Map<string, number>();
  for (const f of folderFiles) {
    sourceMap.set(f.sourceLocation, (sourceMap.get(f.sourceLocation) ?? 0) + 1);
  }
  const sourceDistribution: FolderDistributionItem[] = Array.from(sourceMap.entries())
    .map(([key, count]) => ({
      key,
      label: key === "mods" ? "Mods" : key === "tray" ? "Tray" : key,
      count,
      percentage: totalFiles > 0 ? (count / totalFiles) * 100 : 0,
    }))
    .sort((a, b) => b.count - a.count);

  // ── Creator distribution (top 8) ───────────────────────────────────────
  const creatorMap = new Map<string, number>();
  let missingCreator = 0;
  for (const f of folderFiles) {
    if (f.creator && f.creator.trim()) {
      creatorMap.set(f.creator, (creatorMap.get(f.creator) ?? 0) + 1);
    } else {
      missingCreator++;
    }
  }
  const creatorDistribution: FolderDistributionItem[] = Array.from(creatorMap.entries())
    .map(([key, count]) => ({ key, label: key, count, percentage: totalFiles > 0 ? (count / totalFiles) * 100 : 0 }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 8);
  const notes: string[] = [];
  if (missingCreator > totalFiles * 0.5 && totalFiles > 5) {
    notes.push("Creator info missing on most files");
  }

  // ── Counts ─────────────────────────────────────────────────────────────
  // Note: safetyNotes / parserWarnings are #[serde(skip)] on LibraryFileRow —
  // only available in FileDetail, not in tree/list rows. warningCount is 0 here.
  const duplicateCount = folderFiles.filter((f) => f.hasDuplicate).length;
  const warningCount = 0;

  // ── Relationship clusters ──────────────────────────────────────────────
  // Aggregate from per-row sameFolderPeerCount / samePackPeerCount / hasDuplicate.
  // These are backend window-query fields — O(1) per row, no extra computation needed.
  const duplicateFiles = folderFiles.filter((f) => f.hasDuplicate);
  const packClusterFiles = folderFiles.filter((f) => {
    const cnt = (f as LibraryFileRow & { samePackPeerCount?: number }).samePackPeerCount;
    return cnt != null && cnt > 0;
  });
  const folderPeerFiles = folderFiles.filter((f) => {
    const cnt = (f as LibraryFileRow & { sameFolderPeerCount?: number }).sameFolderPeerCount;
    return cnt != null && cnt > 0;
  });

  const relationshipClusters: FolderRelationshipCluster[] = [];

  if (duplicateFiles.length > 0) {
    relationshipClusters.push({
      id: "dup",
      type: "duplicate",
      proofLevel: "fact",
      confidenceLabel: "Confirmed",
      affectedFileCount: duplicateFiles.length,
      peakPeerCount: 0,
      title: "Duplicate files",
      description: `${duplicateFiles.length} file${duplicateFiles.length !== 1 ? "s" : ""} marked as exact duplicates`,
    });
  }

  if (packClusterFiles.length > 0) {
    const peakPack = Math.max(...packClusterFiles.map((f) => (f as LibraryFileRow & { samePackPeerCount?: number }).samePackPeerCount ?? 0));
    relationshipClusters.push({
      id: "pack",
      type: "same_pack",
      proofLevel: "claim",
      confidenceLabel: "Likely",
      affectedFileCount: packClusterFiles.length,
      peakPeerCount: peakPack,
      title: "Pack groupings",
      description: `${packClusterFiles.length} files share pack membership — likely from the same mod pack`,
    });
  }

  if (folderPeerFiles.length > 0) {
    const peakFolder = Math.max(...folderPeerFiles.map((f) => (f as LibraryFileRow & { sameFolderPeerCount?: number }).sameFolderPeerCount ?? 0));
    relationshipClusters.push({
      id: "folder",
      type: "same_folder",
      proofLevel: "fact",
      confidenceLabel: "Confirmed",
      affectedFileCount: folderPeerFiles.length,
      peakPeerCount: peakFolder,
      title: "Folder sets",
      description: `${folderPeerFiles.length} files share the same folder — often downloaded together`,
    });
  }

  return {
    folderId: folderNode.fullPath,
    folderName: folderNode.name,
    folderPath: folderNode.fullPath,
    counts: {
      totalFiles,
      directFiles,
      nestedFiles,
      subfolderCount: folderNode.childFolderCount,
      warningCount,
      duplicateCount,
      rootFilesCount,
    },
    kindDistribution,
    sourceDistribution,
    creatorDistribution,
    dominantKind: dominantKind?.key,
    dominantKindShare,
    relationshipClusters,
    notes,
  };
}

/**
 * Compute FolderTreeClue for a single tree node.
 * O(n) in allFiles — caller should memoize at the tree level.
 *
 * @param allFiles   All files in the current tree dataset (treeRows.items).
 * @param folderNode The tree node to compute the clue for.
 */
export function computeTreeClue(
  allFiles: LibraryFileRow[],
  folderNode: { fullPath: string; sourceLocation: string },
): FolderTreeClue {
  // Files in this folder's subtree
  const subtreeFiles = allFiles.filter((f) =>
    f.sourceLocation === folderNode.sourceLocation &&
    f.path.replace(/\\/g, "/").includes(folderNode.fullPath),
  );

  if (subtreeFiles.length === 0) {
    return { warningCount: 0, duplicateCount: 0, issueState: "none" };
  }

  const kindMap = new Map<string, number>();
  let duplicateCount = 0;
  let warningCount = 0;

  for (const f of subtreeFiles) {
    if (f.hasDuplicate) duplicateCount++;
    // Note: safetyNotes/parserWarnings not available on LibraryFileRow in tree context
    kindMap.set(f.kind, (kindMap.get(f.kind) ?? 0) + 1);
  }

  const total = subtreeFiles.length;
  let dominantKind: string | undefined;
  let dominantKindShare: number | undefined;
  for (const [kind, count] of kindMap.entries()) {
    const share = (count / total) * 100;
    if (share >= 60 && count >= 4) {
      dominantKind = kind;
      dominantKindShare = share;
      break;
    }
  }

  let issueState: FolderTreeClue["issueState"] = "none";
  if (warningCount > 0 && duplicateCount > 0) issueState = "mixed";
  else if (warningCount > 0) issueState = "warning";
  else if (duplicateCount > 0) issueState = "duplicate";

  return { dominantKind, dominantKindShare, warningCount, duplicateCount, issueState };
}
