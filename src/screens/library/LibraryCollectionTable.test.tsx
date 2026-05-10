import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import type { LibraryFileRow, UserView } from "../../lib/types";
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
    parserWarnings: [],
  },
];

afterEach(() => {
  cleanup();
});

it("shows a calm row in casual mode without the full path", () => {
  render(
    <LibraryCollectionTable
      userView="beginner"
      rows={SAMPLE_ROWS}
      selectedId={1}
      selectedIds={new Set()}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onToggleSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getByText(/betterbuildbuy/i)).toBeInTheDocument();
  expect(screen.queryByText(/mods\\buildbuy/i)).not.toBeInTheDocument();
});

it.each<UserView>(["beginner", "standard", "power"])(
  "renders the list structure in %s mode",
  (userView) => {
    render(
      <LibraryCollectionTable
        userView={userView}
        rows={SAMPLE_ROWS}
        selectedId={1}
        selectedIds={new Set()}
        page={0}
        totalPages={1}
        onSelect={() => {}}
        onToggleSelect={() => {}}
        onPrevPage={() => {}}
        onNextPage={() => {}}
      />,
    );

    expect(screen.getByText(userView === "beginner" ? "File" : "Mod or file")).toBeInTheDocument();
    expect(screen.getByText(/betterbuildbuy/i)).toBeInTheDocument();
    expect(screen.getByText(/status/i)).toBeInTheDocument();
  },
);


it("can rerender from empty results to populated rows without changing hook order", () => {
  const props = {
    userView: "standard" as const,
    selectedId: null,
    selectedIds: new Set<number>(),
    page: 0,
    totalPages: 1,
    onSelect: () => {},
    onToggleSelect: () => {},
    onPrevPage: () => {},
    onNextPage: () => {},
  };

  const { rerender } = render(
    <LibraryCollectionTable
      {...props}
      rows={[]}
    />,
  );

  expect(screen.getByText(/no indexed files match/i)).toBeInTheDocument();

  expect(() =>
    rerender(
      <LibraryCollectionTable
        {...props}
        rows={SAMPLE_ROWS}
      />,
    ),
  ).not.toThrow();

  expect(screen.getAllByText(/betterbuildbuy/i).length).toBeGreaterThan(0);
});

it("uses cautious duplicate and update-source wording in row badges", () => {
  render(
    <LibraryCollectionTable
      userView="standard"
      rows={[
        {
          ...SAMPLE_ROWS[0],
          id: 2,
          filename: "PossiblePreset.package",
          hasDuplicate: true,
          watchStatus: "not_watched",
        },
      ]}
      selectedId={2}
      selectedIds={new Set()}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onToggleSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getAllByText(/possible duplicate/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/no update source/i)).toBeInTheDocument();
  expect(screen.queryByText(/confirmed duplicate/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});
