import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import { LibraryThumbnailGrid } from "./LibraryThumbnailGrid";

const SAMPLE_ROWS: LibraryFileRow[] = [
  {
    id: 1,
    filename: "LongCreatorName_CozyChair.package",
    path: "Mods\\BuildBuy\\LongCreatorName_CozyChair.package",
    extension: ".package",
    kind: "BuildBuy",
    subtype: "Chair",
    confidence: 0.9,
    sourceLocation: "mods",
    size: 4096,
    modifiedAt: "2026-03-19T12:00:00.000Z",
    creator: "LongCreatorName",
    bundleName: null,
    bundleType: null,
    relativeDepth: 2,
    safetyNotes: [],
    parserWarnings: [],
    watchStatus: "not_watched",
    hasDuplicate: false,
    sameFolderPeerCount: 0,
    samePackPeerCount: 0,
  },
];

it("can rerender from empty results to populated cards without changing hook order", () => {
  const props = {
    userView: "standard" as const,
    selectedId: null,
    page: 0,
    totalPages: 1,
    onSelect: () => {},
    onPrevPage: () => {},
    onNextPage: () => {},
  };

  const { rerender } = render(
    <LibraryThumbnailGrid
      {...props}
      rows={[]}
    />,
  );

  expect(screen.getByText(/no indexed files match/i)).toBeInTheDocument();

  expect(() =>
    rerender(
      <LibraryThumbnailGrid
        {...props}
        rows={SAMPLE_ROWS}
      />,
    ),
  ).not.toThrow();

  expect(screen.getByTitle("LongCreatorName_CozyChair.package")).toBeInTheDocument();
});
