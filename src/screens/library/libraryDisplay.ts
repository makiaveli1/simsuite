import { friendlyTypeLabel } from "../../lib/uiLanguage";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";

export interface LibraryViewFlags {
  showAdvancedFilters: boolean;
}

export interface LibraryRowModel {
  id: number;
  title: string;
  typeLabel: string;
  typeTone:
    | "gameplay"
    | "cas"
    | "buildbuy"
    | "scriptmods"
    | "overrides"
    | "poses"
    | "presets"
    | "tray"
    | "unknown";
  healthLabel: string;
  healthTone: "calm" | "attention" | "muted";
}

type LibraryCareSummarySource = Pick<
  FileDetail,
  "installedVersionSummary" | "safetyNotes" | "parserWarnings"
>;

export function libraryViewFlags(userView: UserView): LibraryViewFlags {
  return {
    showAdvancedFilters: userView === "power",
  };
}

export function buildLibraryRowModel(
  row: LibraryFileRow,
  _userView: UserView,
): LibraryRowModel {
  return {
    id: row.id,
    title: row.filename,
    typeLabel: friendlyTypeLabel(row.kind),
    typeTone: libraryTypeTone(row.kind),
    healthLabel: describeLibraryHealth(row),
    healthTone: libraryHealthTone(row),
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

function libraryTypeTone(kind: string): LibraryRowModel["typeTone"] {
  switch (kind) {
    case "Gameplay":
      return "gameplay";
    case "CAS":
      return "cas";
    case "BuildBuy":
      return "buildbuy";
    case "ScriptMods":
      return "scriptmods";
    case "OverridesAndDefaults":
      return "overrides";
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
