import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
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
            resourceSummary: ["Catalog x6", "Definition x6"],
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
  expect(screen.getByText(/catalog x6/i)).toBeVisible();
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
