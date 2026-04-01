import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { LibraryDetailsPanel } from "./LibraryDetailsPanel";

it("shows snapshot, care, and more instead of inline edit forms", () => {
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
          safetyNotes: [],
          parserWarnings: [],
          installedVersionSummary: null,
        } as never
      }
      onOpenMoreDetails={() => {}}
      onOpenUpdates={() => {}}
    />,
  );

  expect(screen.getByText(/snapshot/i)).toBeVisible();
  expect(screen.getByText(/care/i)).toBeVisible();
  expect(screen.getByRole("button", { name: /more details/i })).toBeVisible();
});
