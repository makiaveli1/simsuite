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
};

describe("libraryViewFlags", () => {
  it("only opens the extra filter row by default for creator view", () => {
    expect(libraryViewFlags("beginner").showAdvancedFilters).toBe(false);
    expect(libraryViewFlags("standard").showAdvancedFilters).toBe(false);
    expect(libraryViewFlags("power").showAdvancedFilters).toBe(true);
  });
});

describe("buildLibraryRowModel", () => {
  it("returns a simple list model with a calm type tone", () => {
    const row = buildLibraryRowModel(SAMPLE_ROW, "standard");

    expect(row.typeLabel).toBe("Script Mods");
    expect(row.typeTone).toBe("scriptmods");
    expect("supportingFacts" in row).toBe(false);
  });
});

describe("summarizeLibraryCareState", () => {
  it("prefers action wording a casual simmer can understand", () => {
    expect(
      summarizeLibraryCareState({
        installedVersionSummary: null,
        safetyNotes: ["Possible conflict"],
        parserWarnings: [],
      }),
    ).toContain("needs attention");
  });
});
