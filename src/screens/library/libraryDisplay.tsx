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
  LibraryFileRow,
  UserView,
  VersionConfidence,
  WatchStatus,
} from "../../lib/types";

export interface LibraryViewFlags {
  showCreatorInList: boolean;
  showInspectFactsInList: boolean;
  showAdvancedFilters: boolean;
  showRootFacts: boolean;
  maxSupportingFacts: number;
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
 * Shows embedded item names as a "content preview" — no image extraction needed.
 */
export interface LibraryCardModel {
  id: number;
  title: string;
  kind: string;
  typeLabel: string;
  typeColor: TypeColor;
  creatorLabel: string;
  isTray: boolean;
  isMisplaced: boolean;
  watchStatusLabel: string;
  watchStatusTone: "calm" | "attention" | "muted";
  healthLabel: string | null;
  healthTone: "attention" | "muted" | null;
  hasDuplicate: boolean;
  hasIssues: boolean;
  confidenceLevel: "high" | "medium" | "low";
  /** Embedded item names — shown as the content preview text */
  embeddedNames: string[];
  /** Resource summary — e.g. "6 build/buy items" */
  resourceSummary: string | null;
  /** How many items total in embeddedNames (for "+N more" overflow) */
  totalEmbeddedNames: number;
  /** Version signal if present */
  versionLabel: string | null;
  /** The raw row for click handling */
  row: LibraryFileRow;
}

export interface LibraryRowModel {
  id: number;
  title: string;
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
}

type LibraryCareSummarySource = Pick<
  FileDetail,
  "installedVersionSummary" | "safetyNotes" | "parserWarnings" | "kind" | "sourceLocation"
>;

export function libraryViewFlags(userView: UserView): LibraryViewFlags {
  return {
    showCreatorInList: userView !== "beginner",
    showInspectFactsInList: userView === "power",
    showAdvancedFilters: userView === "power",
    showRootFacts: userView !== "beginner",
    maxSupportingFacts: userView === "power" ? 3 : 2,
  };
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
  });

  const watchStatusLabel = describeWatchStatus(row.watchStatus);
  const watchStatusTone = watchStatusToneFor(row.watchStatus);

  const healthIssue = computeLibraryHealthIssue(row);

  const trayIdentity = describeTrayIdentity({
    kind: row.kind,
    subtype: row.subtype,
    sourceLocation: row.sourceLocation,
  });

  return {
    id: row.id,
    title: row.filename,
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
  };
}

/**
 * Builds a card model for the thumbnail/grid view.
 * Uses embeddedNames as a text-based content preview — no image extraction needed.
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

  // Show up to 4 embedded names as the content preview
  const allNames = row.insights?.embeddedNames ?? [];
  const visibleNames = allNames.slice(0, 4);
  const overflowCount = allNames.length - visibleNames.length;

  // Resource summary from insights
  const resourceSummary = row.insights?.resourceSummary?.[0] ?? null;

  // Version signal for script mods
  const versionSignal = row.insights?.versionSignals?.[0];
  const versionLabel = versionSignal
    ? `v${versionSignal.normalizedValue}`
    : row.insights?.versionHints?.[0] ?? null;

  return {
    id: row.id,
    title: row.filename,
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
    embeddedNames: visibleNames,
    resourceSummary,
    totalEmbeddedNames: allNames.length,
    versionLabel,
    row,
  };
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
 * Returns embedded names filtered to likely human-readable values for display.
 * Filters out raw STBL UUID-style entries and other non-meaningful strings.
 */
export function describeEmbeddedNames(
  insights: FileInsights | undefined,
): string[] {
  if (!insights?.embeddedNames?.length) return [];
  // Filter: skip entries that look like raw UUIDs or numeric IDs
  return insights.embeddedNames.filter((name) => {
    const trimmed = name.trim();
    if (!trimmed) return false;
    // Skip pure hex/UUID-looking strings
    if (/^[0-9a-f]{8}[-]?[0-9a-f]{4}[-]?[0-9a-f]{4}[-]?[0-9a-f]{4}[-]?[0-9a-f]{12}$/i.test(trimmed)) return false;
    // Skip very short numeric strings
    if (/^\d+$/.test(trimmed) && trimmed.length < 4) return false;
    // Skip strings that are mostly internal markers
    if (trimmed.startsWith("#") && trimmed.length < 10) return false;
    return true;
  }).slice(0, 8); // cap at 8 for display
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

/** Single-line script scope label for list rows and inspectors. */
export function summarizeScriptScopeForUi(
  insights: FileInsights | undefined,
): string | null {
  const ns = describeScriptNamespaces(insights);
  if (!ns.count) return null;
  if (ns.count === 1) return ns.samples[0] ?? null;
  return `${ns.samples[0] ?? "?"}+${ns.count - 1}`;
}

/** Single-line resource profile label for package/collection rows. */
export function summarizeResourceProfileForUi(
  insights: FileInsights | undefined,
): string | null {
  const summary = insights?.resourceSummary ?? [];
  if (!summary.length) return null;
  return summary[0] ?? null;
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

  if (kind === "BuildBuy") {
    return primary ?? "Build/Buy content";
  }
  if (kind === "OverridesAndDefaults") {
    return primary ?? "Override package";
  }
  if (kind === "CAS") {
    return subtype?.trim() ?? primary ?? "CAS content";
  }
  if (kind === "PresetsAndSliders") {
    return subtype?.trim() ?? primary ?? "Preset or slider";
  }
  if (kind === "Gameplay") {
    return subtype?.trim() ?? primary ?? "Gameplay package";
  }

  return primary ?? subtype?.trim() ?? null;
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
  }: {
    flags: LibraryViewFlags;
    creatorLabel: string;
    isTray: boolean;
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
        const scope = summarizeScriptScopeForUi(row.insights);
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
                ? "High confidence"
                : row.confidence >= 0.55
                  ? "Inferred version"
                  : "Weak version signal",
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
