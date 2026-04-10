import { describe, expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import {
  buildLibraryRowModel,
  libraryViewFlags,
  summarizeLibraryCareState,
  summarizePackageContentProfile,
  summarizeVersionSignalForUi,
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
  insights: {
    format: "ts4script-zip",
    resourceSummary: ["Script mod"],
    scriptNamespaces: ["deaderpool.mccc"],
    embeddedNames: ["version"],
    creatorHints: ["deaderpool"],
    versionHints: ["2025.9.0"],
    versionSignals: [
      {
        rawValue: "2025.9.0",
        normalizedValue: "2025.9.0",
        sourceKind: "payload",
        sourcePath: "version.txt",
        matchedBy: "readable archive payload",
        confidence: 0.99,
      },
    ],
    familyHints: ["MCCC"],
  },
};

describe("libraryViewFlags", () => {
  it("keeps technical detail out of casual mode", () => {
    expect(libraryViewFlags("beginner").showInspectFactsInList).toBe(false);
  });
});

describe("buildLibraryRowModel", () => {
  it("caps the row at two supporting facts for seasoned mode", () => {
    // SAMPLE_ROW is ScriptMods with no version signal → stays at 2 facts
    const noVersionRow = { ...SAMPLE_ROW, insights: { ...SAMPLE_ROW.insights!, versionSignals: [] } };
    const row = buildLibraryRowModel(noVersionRow, "standard");

    expect(row.supportingFacts).toHaveLength(2);
  });

  it("shows version signal as a third fact in standard ScriptMods view", () => {
    // ScriptMods with a confident version signal gets a 3rd slot in standard view
    const row = buildLibraryRowModel(SAMPLE_ROW, "standard");

    expect(row.supportingFacts).toHaveLength(3);
    expect(row.supportingFacts[0]).toBe("Deaderpool"); // creator
    expect(row.supportingFacts[1]).toBe("deaderpool.mccc"); // namespace scope (1 namespace)
    expect(row.supportingFacts[2]).toMatch(/^v/); // version clue (e.g. v2025.9.0)
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
        groupedFileCount: 3,
      },
      "power",
    );

    expect(row.isMisplaced).toBe(true);
    expect(row.supportingFacts).toContain("LooseBlueprint");
    expect(row.supportingFacts).toContain("3 grouped files");
    expect(row.supportingFacts).toContain("Stored in Mods");
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
        groupedFileCount: 4,
      },
      "power",
    );

    expect(row.supportingFacts.join(" ")).not.toMatch(/C:\\Users\\Player/i);
    expect(row.supportingFacts).toContain("Stored in Tray");
  });

  it("shows content profile for BuildBuy rows in standard view", () => {
    const buildBuyRow = buildLibraryRowModel(
      {
        ...SAMPLE_ROW,
        id: 4,
        filename: "ModernChair.package",
        extension: ".package",
        kind: "BuildBuy",
        subtype: "Seating",
        confidence: 0.88,
        insights: {
          format: "dbpf-package",
          resourceSummary: ["4 build/buy items"],
          scriptNamespaces: [],
          embeddedNames: [],
          creatorHints: [],
          versionHints: [],
          versionSignals: [],
          familyHints: [],
        },
      },
      "standard",
    );

    // Content profile appears before creator in BuildBuy rows
    expect(buildBuyRow.supportingFacts).toContain("4 build/buy items");
    expect(buildBuyRow.supportingFacts).toContain("Deaderpool");
    // subtype is skipped — content profile is more useful than generic "Seating"
    expect(buildBuyRow.supportingFacts.join(" ")).not.toMatch(/Seating/);
  });
});

describe("package and version summaries", () => {
  it("keeps build-buy labels simmer-friendly", () => {
    expect(
      summarizePackageContentProfile(
        {
          format: "dbpf-package",
          resourceSummary: ["6 build/buy items", "2 other resources"],
          scriptNamespaces: [],
          embeddedNames: [],
          creatorHints: [],
          versionHints: [],
          versionSignals: [],
          familyHints: [],
        },
        "BuildBuy",
        null,
      ),
    ).toBe("6 build/buy items");
  });

  it("only surfaces strong-enough version clues for quick UI", () => {
    expect(
      summarizeVersionSignalForUi(
        {
          format: "ts4script-zip",
          resourceSummary: [],
          scriptNamespaces: [],
          embeddedNames: [],
          creatorHints: [],
          versionHints: ["1.41"],
          versionSignals: [
            {
              rawValue: "1.41",
              normalizedValue: "1.41",
              sourceKind: "payload",
              sourcePath: "manifest.yml",
              matchedBy: "readable archive payload",
              confidence: 0.88,
            },
          ],
          familyHints: [],
        },
        0.8,
      ),
    ).toBe("v1.41");
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
