import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import { FolderContentPane } from "./FolderContentPane";
import type { FolderNode } from "./folderTree";

const emptyTree: FolderNode = {
  name: "Mods",
  fullPath: "Mods",
  depth: 0,
  children: [],
  directFileCount: 0,
  totalFileCount: 0,
  childFolderCount: 0,
  files: [],
};

const directFile: LibraryFileRow = {
  id: 1,
  filename: "DirectPreset.package",
  path: "Mods\\Presets\\DirectPreset.package",
  extension: ".package",
  kind: "PresetsAndSliders",
  subtype: "Body Presets",
  confidence: 0.86,
  sourceLocation: "mods",
  size: 2048,
  modifiedAt: "2026-03-19T12:00:00.000Z",
  creator: "Preset Maker",
  bundleName: null,
  bundleType: null,
  relativeDepth: 2,
  safetyNotes: [],
  parserWarnings: [],
};

it("uses Direct files wording and row thumbnail fallbacks for folder file rows", () => {
  const { container } = render(
    <FolderContentPane
      userView="standard"
      folderPath="Mods/Presets"
      subfolders={[]}
      files={[]}
      rootFiles={[directFile]}
      tree={emptyTree}
      onNavigate={() => {}}
      onSelectFile={() => {}}
      selectedFile={null}
    />,
  );

  expect(screen.getByText(/direct files in Mods\/Presets/i)).toBeInTheDocument();
  expect(screen.getByText(/not inside a subfolder/i)).toBeInTheDocument();
  expect(screen.getByText(/directpreset/i)).toBeInTheDocument();
  expect(container.querySelector(".library-row-thumb-fallback")).toBeInTheDocument();
  expect(screen.queryByText(/loose files/i)).toBeNull();
});
