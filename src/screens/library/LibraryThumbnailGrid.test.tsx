import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
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

afterEach(() => {
  cleanup();
});

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

  expect(screen.getAllByTitle("LongCreatorName_CozyChair.package").length).toBeGreaterThan(0);
});

it("shows card identity at rest while keeping missing preview fallback honest", () => {
  render(
    <LibraryThumbnailGrid
      userView="standard"
      rows={SAMPLE_ROWS}
      selectedId={null}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getAllByText(/longcreatorname cozychair/i).length).toBeGreaterThan(0);
  expect(screen.getAllByTitle(/no preview available/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/no preview/i)).toBeInTheDocument();
});
