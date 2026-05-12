import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { LibraryDetailsPanel } from "./LibraryDetailsPanel";

const emptyInsights = {
  format: null,
  resourceSummary: [],
  scriptNamespaces: [],
  embeddedNames: [],
  creatorHints: [],
  versionHints: [],
  versionSignals: [],
  familyHints: [],
};

afterEach(() => {
  cleanup();
});

it("shows inspect, health, and edit actions instead of a single generic details button in standard view", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={
        {
          id: 1,
          filename: "MCCC_MCCommandCenter.ts4script",
          creator: "Deaderpool",
          kind: "ScriptMods",
          subtype: "Core",
          confidence: 0.92,
          safetyNotes: ["dependency_missing"],
          parserWarnings: ["stale_manifest"],
          installedVersionSummary: { version: "1.4.0" },
          watchResult: { status: "current", sourceLabel: "Patreon" },
          insights: {
            ...emptyInsights,
            format: "ts4script-zip",
            resourceSummary: ["Archive entries: 124", "Top-level namespaces: 2"],
            scriptNamespaces: ["deaderpool", "mccc"],
            familyHints: ["mccc"],
          },
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/at a glance/i)).toBeVisible();
  expect(screen.getByText(/needs attention/i)).toBeVisible();
  expect(screen.getByText(/subtype/i)).toBeVisible();
  expect(screen.getByText(/script content/i)).toBeVisible();
  expect(screen.getByText(/2 script folders/i)).toBeVisible();
  expect(screen.getByRole("button", { name: /inspect file/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /warnings & updates/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /edit details/i })).toBeVisible();
});

it("shows a package resource badge in the standard snapshot", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={
        {
          id: 2,
          filename: "CozyKitchen.package",
          creator: null,
          kind: "BuildBuy",
          subtype: null,
          confidence: 0.7,
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
          watchResult: null,
          insights: {
            ...emptyInsights,
            format: "dbpf-package",
            resourceSummary: ["6 build/buy items", "2 other resources"],
          },
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/contents/i)).toBeVisible();
  expect(screen.getByText(/6 build\/buy items/i)).toBeVisible();
});

it("keeps the beginner action simple", () => {
  render(
    <LibraryDetailsPanel
      userView="beginner"
      selectedFile={
        {
          id: 3,
          filename: "CozyKitchen.package",
          creator: null,
          kind: "BuildBuy",
          subtype: null,
          confidence: 0.7,
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
          watchResult: null,
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /more details/i })).toBeVisible();
  expect(screen.queryByRole("button", { name: /warnings & updates/i })).toBeNull();
  expect(screen.queryByRole("button", { name: /edit details/i })).toBeNull();
});

it("surfaces tray identity and storage context in the inspector", () => {
  render(
    <LibraryDetailsPanel
      userView="power"
      selectedFile={
        {
          id: 4,
          filename: "OakHousehold_0x00ABCDEF.trayitem",
          path: "Tray\\OakHousehold_0x00ABCDEF.trayitem",
          extension: ".trayitem",
          creator: "Oakby",
          kind: "TrayHousehold",
          subtype: "Household",
          bundleName: "OakHousehold",
          bundleType: "household",
          groupedFileCount: 4,
          sourceLocation: "tray",
          confidence: 0.92,
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
          watchResult: null,
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/tray type/i)).toBeVisible();
  expect(screen.getAllByText(/stored/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/stored in tray/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/grouped as/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/oakhousehold/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/4 grouped files/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/stored in tray as library content/i)).toBeVisible();
});

it("hides raw path-like tray grouping values from inspector summary", () => {
  render(
    <LibraryDetailsPanel
      userView="power"
      selectedFile={
        {
          id: 5,
          filename: "OakHousehold_0x00ABCDEF.trayitem",
          path: "Tray\\OakHousehold_0x00ABCDEF.trayitem",
          extension: ".trayitem",
          creator: "Oakby",
          kind: "TrayHousehold",
          subtype: "Household",
          bundleName: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Tray\\OakHousehold",
          bundleType: "household",
          sourceLocation: "tray",
          confidence: 0.92,
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
          watchResult: null,
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.queryByText(/grouped as/i)).toBeNull();
  expect(screen.queryByText(/C:\\Users\\Player/i)).toBeNull();
});

it("shows a narrow relationship explanation in the inspector", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={
        {
          id: 6,
          filename: "CoreHelper.ts4script",
          path: "Mods\\CoreHelper\\CoreHelper.ts4script",
          creator: "Helper Studio",
          kind: "ScriptMods",
          subtype: "Core",
          confidence: 0.92,
          sourceLocation: "mods",
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
          watchResult: null,
          insights: emptyInsights,
        } as never
      }
      relationship={{
        type: "same_folder",
        proofLevel: "fact",
        label: "Same folder",
        peerCount: 1,
        countScope: "visible_only",
        evidenceSource: "filtered_folder",
      }}
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/related/i)).toBeVisible();
  expect(screen.getAllByText(/same folder/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/same mods folder/i)).toBeVisible();
  expect(screen.getByText(/confirmed/i)).toBeVisible();
  expect(screen.getByText(/visible-only/i)).toBeVisible();
  expect(screen.queryByText(/dependency/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});

it("shows action preflight wording without claiming dependency proof", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={
        {
          id: 6,
          filename: "CoreHelper.ts4script",
          path: "Mods\\CoreHelper\\CoreHelper.ts4script",
          creator: "Helper Studio",
          kind: "ScriptMods",
          subtype: "Core",
          confidence: 0.92,
          safetyNotes: [],
          parserWarnings: [],
          duplicateTypes: ["exact"],
          duplicatesCount: 1,
          installedVersionSummary: null,
          watchResult: null,
          problemSignals: [
            {
              signalType: "duplicate_candidate",
              severity: "caution",
              proofLevel: "detected",
              shortLabel: "Possible duplicate",
              explanation: "SimSuite found another file that matches duplicate comparison rules, so compare before changing either copy.",
              source: "duplicates",
              evidence: ["Duplicate comparison rule matched"],
              destination: "duplicates",
              showInLibrary: false,
              showInInspector: true,
              showInMoreDetails: true,
              showInNeedsReview: false,
            },
            {
              signalType: "script_mod_caution",
              severity: "info",
              proofLevel: "confirmed",
              shortLabel: "Script mod caution",
              explanation: "This is a script mod, so check the mod notes before disabling, moving, or deleting it.",
              source: "file_kind",
              evidence: ["kind = ScriptMods"],
              destination: null,
              showInLibrary: false,
              showInInspector: true,
              showInMoreDetails: true,
              showInNeedsReview: false,
            },
          ],
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenNeedsReview={() => {}}
      onOpenDuplicates={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getAllByText(/before changing/i).length).toBeGreaterThan(0);
  expect(screen.getByRole("button", { name: /review cautions/i })).toBeVisible();
  expect(screen.queryByText(/break saves/i)).toBeNull();
  expect(screen.queryByText(/mods that depend on it/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});

it("explains no-source and duplicate cues in the inspector without unsafe claims", () => {
  render(
    <LibraryDetailsPanel
      userView="standard"
      selectedFile={
        {
          id: 8,
          filename: "PossiblePreset.package",
          path: "Mods\\Presets\\PossiblePreset.package",
          creator: "Preset Maker",
          kind: "PresetsAndSliders",
          subtype: "Body preset",
          confidence: 0.88,
          safetyNotes: [],
          parserWarnings: [],
          duplicateTypes: ["filename"],
          duplicatesCount: 1,
          installedVersionSummary: null,
          watchResult: { status: "not_watched", sourceLabel: null },
          problemSignals: [],
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenDuplicates={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/what this means/i)).toBeVisible();
  expect(screen.getByText(/does not know where to check/i)).toBeVisible();
  expect(screen.getByText(/compare duplicate candidates/i)).toBeVisible();
  expect(screen.getAllByRole("button", { name: /open in updates/i })).toHaveLength(1);
  expect(screen.getAllByRole("button", { name: /compare in duplicates/i })).toHaveLength(1);
  expect(screen.queryByText(/confirmed duplicate/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});

it("opens the selected file folder with the real disk path in power view", () => {
  const onOpenFolder = vi.fn();

  render(
    <LibraryDetailsPanel
      userView="power"
      selectedFile={
        {
          id: 9,
          filename: "RuntimeProof.package",
          path: "C:\\Fixtures\\Mods\\RuntimeProof\\RuntimeProof.package",
          creator: "Fixture Creator",
          kind: "Gameplay",
          subtype: null,
          confidence: 0.91,
          safetyNotes: [],
          parserWarnings: [],
          duplicateTypes: [],
          duplicatesCount: 0,
          installedVersionSummary: null,
          watchResult: null,
          problemSignals: [],
          insights: emptyInsights,
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
      onOpenFolder={onOpenFolder}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: /open folder/i }));

  expect(onOpenFolder).toHaveBeenCalledWith(
    "C:\\Fixtures\\Mods\\RuntimeProof\\RuntimeProof.package",
  );
  expect(onOpenFolder).not.toHaveBeenCalledWith(expect.stringMatching(/^Mods[\\/]/));
});
