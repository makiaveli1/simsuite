import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import { VirtualizedLooseFiles } from "./VirtualizedLooseFiles";

function makeRow(id: number): LibraryFileRow {
  return {
    id,
    filename: `direct-file-${id}.package`,
    path: `C:\\Mods\\direct-file-${id}.package`,
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.86,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: "Fixture",
    bundleName: null,
    bundleType: null,
    relativeDepth: 0,
    safetyNotes: [],
    parserWarnings: [],
    watchStatus: "not_watched",
    hasDuplicate: false,
    sameFolderPeerCount: 0,
    samePackPeerCount: 0,
  };
}

it("can rerender from a small direct-file group to a virtualized group without changing hook order", () => {
  const smallRows = [makeRow(1)];
  const largeRows = Array.from({ length: 301 }, (_, index) => makeRow(index + 1));

  const { rerender } = render(
    <VirtualizedLooseFiles
      userView="standard"
      allFiles={smallRows}
      selectedFile={null}
      onSelectFile={() => {}}
    />,
  );

  expect(screen.getByTitle("direct-file-1.package")).toBeInTheDocument();

  expect(() =>
    rerender(
      <VirtualizedLooseFiles
        userView="standard"
        allFiles={largeRows}
        selectedFile={null}
        onSelectFile={() => {}}
      />,
    ),
  ).not.toThrow();

  expect(screen.getByRole("button", { name: /show all 301 files/i })).toBeInTheDocument();
});
