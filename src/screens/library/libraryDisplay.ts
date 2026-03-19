import { friendlyTypeLabel, unknownCreatorLabel } from "../../lib/uiLanguage";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";

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
  healthLabel: string;
  healthTone: "calm" | "attention" | "muted";
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
  ].filter((value): value is string => Boolean(value));

  return {
    id: row.id,
    title: row.filename,
    typeLabel: friendlyTypeLabel(row.kind),
    healthLabel: describeLibraryHealth(row),
    healthTone: libraryHealthTone(row),
    supportingFacts: supportingFacts.slice(0, flags.maxSupportingFacts),
  };
}

export function summarizeLibraryCareState(
  detail: LibraryCareSummarySource,
): string {
  if (detail.safetyNotes.length || detail.parserWarnings.length) {
    return "This file needs attention before you forget about it.";
  }

  if (detail.installedVersionSummary) {
    return "This file has update tracking ready if you want to check it.";
  }

  return "Nothing stands out right now.";
}

function describeLibraryHealth(row: Pick<LibraryFileRow, "confidence" | "safetyNotes">) {
  if (row.safetyNotes.length) {
    return "Needs attention";
  }

  if (row.confidence < 0.55) {
    return "Check details";
  }

  return "Looks okay";
}

function libraryHealthTone(
  row: Pick<LibraryFileRow, "confidence" | "safetyNotes">,
): LibraryRowModel["healthTone"] {
  if (row.safetyNotes.length) {
    return "attention";
  }

  if (row.confidence < 0.55) {
    return "muted";
  }

  return "calm";
}
