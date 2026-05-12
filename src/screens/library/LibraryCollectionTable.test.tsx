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

it("uses deterministic duplicate and update-source wording in row badges", () => {
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

  expect(screen.getAllByText(/^duplicate$/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/no update source/i)).toBeInTheDocument();
  expect(screen.queryByText(/confirmed duplicate/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});

it("shows row thumbnails without source labels covering the preview", () => {
  const { container } = render(
    <LibraryCollectionTable
      userView="standard"
      rows={[
        {
          ...SAMPLE_ROWS[0],
          id: 4,
          filename: "Thumbnailed.package",
          insights: {
            ...emptyInsights,
            cachedThumbnailPreview: "iVBORw0KGgo=",
          },
        },
      ]}
      selectedId={4}
      selectedIds={new Set()}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onToggleSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(container.querySelector(".library-row-thumb-img")).toBeInTheDocument();
  expect(container.querySelector(".library-row-thumb-real")).toBeInTheDocument();
  expect(container.querySelector(".library-row-thumb-source-dot--cache")).toBeInTheDocument();
  expect(screen.queryByText(/^CH$/)).toBeNull();
  expect(screen.queryByText(/^EM$/)).toBeNull();
});

it("shows an intentional row thumbnail fallback for types without previews", () => {
  const { container } = render(
    <LibraryCollectionTable
      userView="standard"
      rows={[
        {
          ...SAMPLE_ROWS[0],
          id: 5,
          filename: "ScriptWithoutPreview.package",
          kind: "ScriptMods",
          insights: emptyInsights,
        },
      ]}
      selectedId={5}
      selectedIds={new Set()}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onToggleSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(container.querySelector(".library-row-thumb-fallback")).toBeInTheDocument();
  expect(container.querySelector(".library-row-thumb-frame--script")).toBeInTheDocument();
  expect(container.querySelector(".library-row-thumb-fallback-icon")).toBeInTheDocument();
});

it("keeps overflow row cues out of the row instead of half-rendering every badge", () => {
  render(
    <LibraryCollectionTable
      userView="standard"
      rows={[
        {
          ...SAMPLE_ROWS[0],
          id: 3,
          filename: "CrowdedSignals.package",
          hasDuplicate: true,
          watchStatus: "not_watched",
          parserWarnings: ["Could not inspect fully"],
        },
      ]}
      selectedId={3}
      selectedIds={new Set()}
      page={0}
      totalPages={1}
      onSelect={() => {}}
      onToggleSelect={() => {}}
      onPrevPage={() => {}}
      onNextPage={() => {}}
    />,
  );

  expect(screen.getByText(/no update source/i)).toBeInTheDocument();
  expect(screen.getByText(/warning/i)).toBeInTheDocument();
  expect(screen.queryByText(/\+1 more/i)).toBeNull();
  expect(screen.queryByText(/possible duplicate/i)).toBeNull();
  expect(screen.queryByText(/confirmed duplicate/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
});
