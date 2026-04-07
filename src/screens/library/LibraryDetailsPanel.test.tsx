import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { LibraryDetailsPanel } from "./LibraryDetailsPanel";

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
          filename: "WonderfulWhims.package",
          creator: "Lumpinou",
          kind: "Gameplay",
          subtype: "Romance",
          confidence: 0.92,
          safetyNotes: ["dependency_missing"],
          parserWarnings: ["stale_manifest"],
          installedVersionSummary: { version: "1.4.0" },
          watchResult: { status: "current", sourceLabel: "Patreon" },
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/snapshot/i)).toBeVisible();
  expect(screen.getByText(/care/i)).toBeVisible();
  expect(screen.getByRole("button", { name: /inspect file/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /health details/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /edit details/i })).toBeVisible();
});

it("keeps the beginner action simple", () => {
  render(
    <LibraryDetailsPanel
      userView="beginner"
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
        } as never
      }
      onOpenInspectDetails={() => {}}
      onOpenHealthDetails={() => {}}
      onOpenEditDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByRole("button", { name: /more details/i })).toBeVisible();
  expect(screen.queryByRole("button", { name: /health details/i })).toBeNull();
  expect(screen.queryByRole("button", { name: /edit details/i })).toBeNull();
});
