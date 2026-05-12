import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { DuplicatesScreen } from "./DuplicatesScreen";
import type { DuplicateOverview, DuplicatePair } from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    getDuplicateOverview: vi.fn(),
    listDuplicatePairs: vi.fn(),
  },
}));

const overview: DuplicateOverview = {
  totalPairs: 2,
  exactPairs: 1,
  filenamePairs: 1,
  versionPairs: 0,
};

const duplicatePairs: DuplicatePair[] = [
  {
    id: 1,
    duplicateType: "filename",
    detectionMethod: "Filename match",
    isDuplicate: false,
    comparisonKind: "name_match_review",
    classification: "name_match_review",
    classificationLabel: "Name match",
    confidenceLabel: "Name match",
    evidence: ["Same filename", "Creator unknown"],
    cautions: ["Compare before changing anything"],
    primaryFileId: 11,
    primaryFilename: "Other.package",
    primaryPath: "C:\\Fixtures\\Mods\\Other.package",
    primaryCreator: null,
    primaryHash: null,
    primaryModifiedAt: null,
    primarySize: 120,
    secondaryFileId: 12,
    secondaryFilename: "Other Copy.package",
    secondaryPath: "C:\\Fixtures\\Mods\\Other Copy.package",
    secondaryCreator: null,
    secondaryHash: null,
    secondaryModifiedAt: null,
    secondarySize: 120,
  },
  {
    id: 2,
    duplicateType: "exact",
    detectionMethod: "Exact hash match",
    isDuplicate: true,
    comparisonKind: "exact_file",
    classification: "duplicate",
    classificationLabel: "Duplicate",
    confidenceLabel: "Same file contents",
    evidence: ["Same file contents", "Size matches", "Creator matches"],
    cautions: ["Compare before changing anything"],
    primaryFileId: 42,
    primaryFilename: "mc_cmd_center.package",
    primaryPath: "C:\\Fixtures\\Mods\\MCCC\\mc_cmd_center.package",
    primaryCreator: "Deaderpool",
    primaryHash: "abc",
    primaryModifiedAt: null,
    primarySize: 2048,
    secondaryFileId: 43,
    secondaryFilename: "mc_cmd_center copy.package",
    secondaryPath: "C:\\Fixtures\\Mods\\MCCC Copy\\mc_cmd_center.package",
    secondaryCreator: "Deaderpool",
    secondaryHash: "abc",
    secondaryModifiedAt: null,
    secondarySize: 2048,
  },
];

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("focuses a duplicate pair when Library opens Duplicates with file context", async () => {
  vi.mocked(api.getDuplicateOverview).mockResolvedValue(overview);
  vi.mocked(api.listDuplicatePairs).mockResolvedValue(duplicatePairs);

  render(
    <UiPreferencesProvider mode="seasoned">
      <DuplicatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        userView="standard"
        initialFileIds={[42]}
      />
    </UiPreferencesProvider>,
  );

  expect(await screen.findByText("Opened from Library.")).toBeVisible();
  expect(
    screen.getByText(/Focused a Library comparison for mc_cmd_center\.package/i),
  ).toBeVisible();
  expect(screen.getAllByText("mc_cmd_center.package").length).toBeGreaterThan(0);
});
