import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, LibraryFileRow, UserView, WatchStatus } from "../../lib/types";

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

export interface LibraryRowModel {
  id: number;
  title: string;
  typeLabel: string;
  /** CSS type-color key for the type indicator dot/border */
  typeColor: TypeColor;
  /** True if this item lives in the tray (effectively disabled) */
  isTray: boolean;
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
  "installedVersionSummary" | "safetyNotes" | "parserWarnings"
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

  return {
    id: row.id,
    title: row.filename,
    typeLabel: friendlyTypeLabel(row.kind),
    typeColor: typeColorForKind(row.kind),
    isTray,
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

/** Maps SimSuite kind (PascalCase) to a CSS type-color key */
function typeColorForKind(kind: string): TypeColor {
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
      return "tray";
    default:
      return "unknown";
  }
}

export function summarizeLibraryCareState(
  detail: LibraryCareSummarySource,
): string {
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
    // CAS items: show subtype (e.g. "hair", "tops") + creator
    case "CAS":
      if (row.subtype?.trim()) facts.push(row.subtype.trim());
      facts.push(creatorLabel);
      break;

    // Script mods: show creator + confidence level
    case "ScriptMods":
      facts.push(creatorLabel);
      if (row.confidence != null) {
        const confLevel =
          row.confidence >= 0.8
            ? "High confidence"
            : row.confidence >= 0.55
              ? "Medium confidence"
              : "Low confidence";
        facts.push(confLevel);
      }
      break;

    // Gameplay mods: show creator
    case "Gameplay":
      facts.push(creatorLabel);
      if (flags.showInspectFactsInList && row.subtype?.trim()) {
        facts.push(row.subtype.trim());
      }
      break;

    // Build/Buy: show creator + source location
    case "BuildBuy":
      facts.push(creatorLabel);
      if (flags.showRootFacts) facts.push(isTray ? "🔖 In tray" : "Mods");
      break;

    // Overrides & Defaults: show creator + confidence
    case "OverridesAndDefaults":
      facts.push(creatorLabel);
      if (row.confidence != null) {
        const confLevel =
          row.confidence >= 0.8
            ? "High confidence"
            : row.confidence >= 0.55
              ? "Medium confidence"
              : "Low confidence";
        facts.push(confLevel);
      }
      break;

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
      facts.push("🔖 Tray");
      facts.push(creatorLabel);
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
