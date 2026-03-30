import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, LibraryFileRow, UserView, WatchStatus } from "../../lib/types";

export interface LibraryViewFlags {
  showCreatorInList: boolean;
  showInspectFactsInList: boolean;
  showAdvancedFilters: boolean;
  showRootFacts: boolean;
  maxSupportingFacts: number;
}

export interface LibraryRowModel {
  id: number;
  title: string;
  typeLabel: string;
  /** Renders only when there is an issue. null = no health indicator shown. */
  healthLabel: string | null;
  healthTone: "attention" | "muted" | null;
  /** Duplicate flag — renders only when this file appears in a duplicate pair. */
  duplicateLabel: string | null;
  duplicateTone: "muted" | null;
  watchStatusLabel: string;
  watchStatusTone: "calm" | "attention" | "muted";
  supportingFacts: string[];
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
  const supportingFacts = [
    row.subtype?.trim() || friendlyTypeLabel(row.kind),
    flags.showCreatorInList ? creatorLabel : null,
    flags.showRootFacts
      ? row.sourceLocation === "tray"
        ? "Tray"
        : "Mods"
      : null,
    flags.showInspectFactsInList ? `Depth ${row.relativeDepth}` : null,
    // Show installed version for power users on tracked items.
    flags.showCreatorInList && row.installedVersion
      ? `v${row.installedVersion}`
      : null,
  ].filter((value): value is string => Boolean(value));

  const watchStatusLabel = describeWatchStatus(row.watchStatus);
  const watchStatusTone = watchStatusToneFor(row.watchStatus);
  const healthIssue = computeLibraryHealthIssue(row);

  return {
    id: row.id,
    title: row.filename,
    typeLabel: friendlyTypeLabel(row.kind),
    healthLabel: healthIssue?.label ?? null,
    healthTone: healthIssue?.tone ?? null,
    duplicateLabel: row.hasDuplicate ? "Duplicate" : null,
    duplicateTone: row.hasDuplicate ? "muted" : null,
    watchStatusLabel,
    watchStatusTone,
    supportingFacts: supportingFacts.slice(0, flags.maxSupportingFacts),
  };
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

/**
 * Returns the most significant issue for a library item, or null if the item is fine.
 * Priority: safety note > disabled/tray > parser warning.
 * Low confidence alone is NOT an issue — it just means identification is uncertain.
 */
function computeLibraryHealthIssue(
  row: Pick<LibraryFileRow, "safetyNotes" | "parserWarnings" | "sourceLocation">,
): { label: string; tone: "attention" | "muted" } | null {
  // Safety notes are genuine problems that deserve manual review.
  if (row.safetyNotes.length > 0) {
    return { label: "Needs review", tone: "attention" };
  }
  // Disabled — in tray means not loaded by the game.
  if (row.sourceLocation === "tray") {
    return { label: "Disabled", tone: "muted" };
  }
  // Parser warnings are notable but usually not game-breaking.
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
): LibraryRowModel["watchStatusTone"] {
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
