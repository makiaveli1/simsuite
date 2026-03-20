import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { DockSectionDefinition } from "../../components/DockSectionStack";
import { UiPreferencesProvider } from "../../components/UiPreferencesContext";
import type { FileDetail } from "../../lib/types";
import { LibraryDetailSheet } from "./LibraryDetailSheet";

afterEach(() => {
  cleanup();
});

const SAMPLE_FILE: FileDetail = {
  id: 1,
  filename: "(PinkishWrld)Rose Body Preset V2.package",
  path: "Mods\\Presets\\Rose Body Preset V2.package",
  extension: ".package",
  kind: "PresetsAndSliders",
  subtype: "Body Presets",
  confidence: 0.72,
  sourceLocation: "mods",
  size: 2048,
  modifiedAt: "2026-03-20T00:00:00.000Z",
  creator: "PinkishWrld",
  bundleName: null,
  bundleType: null,
  relativeDepth: 2,
  safetyNotes: [],
  hash: null,
  createdAt: null,
  parserWarnings: [],
  insights: {
    format: "Package file",
    resourceSummary: [],
    scriptNamespaces: [],
    embeddedNames: [],
    creatorHints: [],
    versionHints: [],
    versionSignals: [],
    familyHints: [],
  },
  installedVersionSummary: null,
  watchResult: null,
  creatorLearning: {
    lockedByUser: false,
    preferredPath: null,
    learnedAliases: [],
  },
  categoryOverride: {
    savedByUser: false,
    kind: null,
    subtype: null,
  },
};

const SAMPLE_SECTIONS: DockSectionDefinition[] = [
  {
    id: "updates",
    label: "Updates",
    hint: "Version tracking now lives in Updates.",
    children: <p>Check the Updates tab to manage tracking.</p>,
  },
];

describe("LibraryDetailSheet", () => {
  it("uses a compact summary and does not show panel reset controls", () => {
    render(
      <UiPreferencesProvider mode="seasoned">
        <LibraryDetailSheet
          open
          mode="health"
          selectedFile={SAMPLE_FILE}
          sections={SAMPLE_SECTIONS}
          userView="standard"
          onClose={() => {}}
        />
      </UiPreferencesProvider>,
    );

    expect(screen.getByText(/\(PinkishWrld\)Rose Body Preset V2\.package/i)).toBeInTheDocument();
    expect(screen.getByText(/^PinkishWrld$/)).toBeInTheDocument();
    expect(screen.getByText(/72%/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /reset/i })).not.toBeInTheDocument();
    expect(screen.queryByTitle(/move updates up/i)).not.toBeInTheDocument();
  });
});
