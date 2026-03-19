import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import { LibraryCollectionTable } from "./LibraryCollectionTable";

const SAMPLE_ROWS: LibraryFileRow[] = [
  {
    id: 1,
    filename: "BetterBuildBuy.package",
    path: "Mods\\BuildBuy\\BetterBuildBuy.package",
    extension: ".package",
    kind: "Gameplay",
    subtype: "Build",
    confidence: 0.92,
    sourceLocation: "mods",
    size: 2048,
    modifiedAt: "2026-03-19T12:00:00.000Z",
    creator: "TwistedMexi",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: [],
  },
];

it("shows a calm row in casual mode without the full path", () => {
  render(
    <LibraryCollectionTable
      userView="beginner"
      rows={SAMPLE_ROWS}
      selectedId={1}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getByText(/betterbuildbuy\.package/i)).toBeInTheDocument();
  expect(screen.queryByText(/mods\\buildbuy/i)).not.toBeInTheDocument();
});
