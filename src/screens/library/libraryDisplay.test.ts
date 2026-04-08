import { describe, expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import {
  buildLibraryRowModel,
  libraryViewFlags,
  summarizeLibraryCareState,
} from "./libraryDisplay";

const SAMPLE_ROW: LibraryFileRow = {
  id: 1,
  filename: "MCCmdCenter.package",
  path: "Mods\\Scripts\\MCCmdCenter.package",
  extension: ".package",
  kind: "ScriptMods",
  subtype: "Core",
  confidence: 0.91,
  sourceLocation: "mods",
  size: 2048,
  modifiedAt: "2026-03-19T12:00:00.000Z",
  creator: "Deaderpool",
  bundleName: null,
  bundleType: null,
  relativeDepth: 2,
  safetyNotes: [],
  parserWarnings: [],
};

describe("libraryViewFlags", () => {
  it("keeps technical detail out of casual mode", () => {
    expect(libraryViewFlags("beginner").showInspectFactsInList).toBe(false);
  });
});

describe("buildLibraryRowModel", () => {
  it("caps the row at two supporting facts for seasoned mode", () => {
    const row = buildLibraryRowModel(SAMPLE_ROW, "standard");

    expect(row.supportingFacts).toHaveLength(2);
  });

  it("surfaces tray grouping and placement for tray items", () => {
    const row = buildLibraryRowModel(
      {
        ...SAMPLE_ROW,
        id: 2,
        filename: "LooseBlueprint.blueprint",
        extension: ".blueprint",
        kind: "TrayLot",
        subtype: "Lot",
        sourceLocation: "mods",
        creator: null,
        bundleName: "LooseBlueprint",
        bundleType: "lot",
      },
      "power",
    );

    expect(row.isMisplaced).toBe(true);
    expect(row.supportingFacts).toContain("LooseBlueprint");
    expect(row.supportingFacts).toContain("Misplaced tray");
  });

  it("suppresses raw path-like tray grouping values in row clues", () => {
    const row = buildLibraryRowModel(
      {
        ...SAMPLE_ROW,
        id: 3,
        filename: "OakHousehold_0x00ABCDEF.trayitem",
        extension: ".trayitem",
        kind: "TrayHousehold",
        subtype: "Household",
        sourceLocation: "tray",
        creator: "Oakby",
        bundleName: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Tray\\OakHousehold",
        bundleType: "household",
      },
      "power",
    );

    expect(row.supportingFacts.join(" ")).not.toMatch(/C:\\Users\\Player/i);
    expect(row.supportingFacts).toContain("🔖 Tray");
  });
});

describe("summarizeLibraryCareState", () => {
  it("prefers action wording a casual simmer can understand", () => {
    expect(
      summarizeLibraryCareState({
        installedVersionSummary: null,
        safetyNotes: ["Possible conflict"],
        parserWarnings: [],
        kind: "ScriptMods",
        sourceLocation: "mods",
      }),
    ).toContain("deserve attention");
  });

  it("uses tray-specific wording for tray content", () => {
    expect(
      summarizeLibraryCareState({
        installedVersionSummary: null,
        safetyNotes: [],
        parserWarnings: [],
        kind: "TrayHousehold",
        sourceLocation: "tray",
      }),
    ).toContain("lives in Tray");
  });
});
