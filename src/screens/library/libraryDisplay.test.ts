import { describe, expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import {
  buildLibraryRowModel,
  describeResourceSummary,
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

  it("shows namespace first, then version clue when namespace is missing", () => {
    // With namespace present: creator + namespace = 2 facts (version is fallback only)
    const withNamespace = {
      ...SAMPLE_ROW,
      insights: {
        ...SAMPLE_ROW.insights!,
        scriptNamespaces: ["deaderpool.mccc"],
        versionSignals: [{ rawValue: "2025.9.0", normalizedValue: "2025.9.0", sourceKind: "payload", sourcePath: "version.txt", matchedBy: "readable archive payload", confidence: 0.99 }],
      },
    };
    const rowWithNs = buildLibraryRowModel(withNamespace, "standard");
    expect(rowWithNs.supportingFacts).toHaveLength(2);
    expect(rowWithNs.supportingFacts[0]).toBe("Deaderpool"); // creator
    expect(rowWithNs.supportingFacts[1]).toBe("deaderpool.mccc"); // namespace (not version)

    // Without namespace: version clue surfaces as the 2nd fact
    const withoutNamespace = {
      ...SAMPLE_ROW,
      insights: {
        ...SAMPLE_ROW.insights!,
        scriptNamespaces: [],
        versionSignals: [{ rawValue: "2025.9.0", normalizedValue: "2025.9.0", sourceKind: "payload", sourcePath: "version.txt", matchedBy: "readable archive payload", confidence: 0.99 }],
      },
    };
    const rowWithoutNs = buildLibraryRowModel(withoutNamespace, "standard");
    expect(rowWithoutNs.supportingFacts).toHaveLength(2);
    expect(rowWithoutNs.supportingFacts[0]).toBe("Deaderpool"); // creator
    expect(rowWithoutNs.supportingFacts[1]).toMatch(/^v/); // version clue (no namespace)
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

describe("describeResourceSummary", () => {
  it("passes through already-humanized Rust-backend format", () => {
    expect(describeResourceSummary("6 build/buy items")).toBe("6 build/buy items");
    expect(describeResourceSummary("2 CAS parts")).toBe("2 CAS parts");
    expect(describeResourceSummary("48 script entries")).toBe("48 script entries");
  });

  it("humanizes raw DBPF Foo x N format", () => {
    expect(describeResourceSummary("Catalog x6")).toBe("6 catalog items");
    expect(describeResourceSummary("CASPart x2")).toBe("2 caspart items");
    expect(describeResourceSummary("HotSpotControl x1")).toBe("1 hotspotcontrol item");
    expect(describeResourceSummary("StringTable x18")).toBe("18 stringtable items");
  });

  it("suppresses NameMap and other internal noise", () => {
    expect(describeResourceSummary("NameMap x1")).toBe(null);
    expect(describeResourceSummary("Compressed x4")).toBe(null);
    expect(describeResourceSummary("S4mpdData x2")).toBe(null);
  });

  it("humanizes colon format", () => {
    expect(describeResourceSummary("Image: 1")).toBe("1 image");
    expect(describeResourceSummary("audio: 2")).toBe("2 audio entries");
    expect(describeResourceSummary("StringTable: 18")).toBe("18 text entries");
    expect(describeResourceSummary("image: 3")).toBe("3 images");
  });

  it("suppresses hex-like noise", () => {
    expect(describeResourceSummary("e1070b30")).toBe(null);
    expect(describeResourceSummary("0x00000000")).toBe(null);
    expect(describeResourceSummary("00000000deadbeef")).toBe(null);
  });

  it("passes through archive-style labels", () => {
    expect(describeResourceSummary("Archive entries: 48")).toBe("Archive entries: 48");
    expect(describeResourceSummary("Top-level namespaces: 2")).toBe("Top-level namespaces: 2");
  });

  it("suppresses unknown short values", () => {
    expect(describeResourceSummary("abc")).toBe(null);
    expect(describeResourceSummary("x")).toBe(null);
  });

  it("returns null for empty/whitespace", () => {
    expect(describeResourceSummary("")).toBe(null);
    expect(describeResourceSummary("   ")).toBe(null);
  });
});
