import { describe, expect, it, afterEach } from "vitest";
import type { LibraryFileRow } from "../../lib/types";
import { buildFolderTree, clearCachedTree, getFolderContents } from "./folderTree";

function fileRow(id: number, path: string): LibraryFileRow {
  return {
    id,
    filename: path.split(/[\\/]/).at(-1) ?? `file-${id}.package`,
    path,
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.9,
    sourceLocation: "mods",
    size: 128,
    modifiedAt: null,
    creator: null,
    bundleName: null,
    bundleType: null,
    relativeDepth: 0,
    safetyNotes: [],
    parserWarnings: [],
  };
}

describe("folder tree contents", () => {
  afterEach(() => {
    clearCachedTree();
  });

  it("finds files in nested folders below the first child level", () => {
    const files = [
      fileRow(1, "C:/Mods/Creator/direct.package"),
      fileRow(2, "C:/Mods/Creator/Nested/deep.package"),
    ];
    const tree = buildFolderTree(files);

    const contents = getFolderContents("Mods/Creator/Nested", files, tree);

    expect(contents.subfolders).toEqual([]);
    expect(contents.files.map((file) => file.id)).toEqual([2]);
    expect(contents.rootFiles).toEqual([]);
  });
});
