import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { buildSheetRelationshipsSection } from "./libraryDisplay";

afterEach(() => {
  cleanup();
});

it("keeps relationship sheet copy honest about clues versus dependency proof", () => {
  const selected = {
    id: 10,
    filename: "CoreHelper.ts4script",
    path: "Mods\\CoreHelper\\CoreHelper.ts4script",
    kind: "ScriptMods",
    sourceLocation: "mods",
    creator: "Helper Studio",
    sameFolderPeerCount: 2,
    samePackPeerCount: 0,
    duplicateTypes: ["exact"],
    duplicatesCount: 1,
    groupedFileCount: null,
    bundleName: null,
  };
  const peers = [
    selected,
    {
      ...selected,
      id: 11,
      filename: "CoreHelper.package",
      path: "Mods\\CoreHelper\\CoreHelper.package",
      duplicateTypes: [],
      duplicatesCount: 0,
    },
  ];

  render(<>{buildSheetRelationshipsSection(selected as never, peers as never, "standard")}</>);

  expect(screen.getByText(/known file facts/i)).toBeVisible();
  expect(screen.getByText(/this confirms shared placement, not a dependency/i)).toBeVisible();
  expect(screen.getByText(/check before removing/i)).toBeVisible();
  expect(screen.queryByText(/confirmed relationships/i)).toBeNull();
  expect(screen.queryByText(/safe-delete/i)).toBeNull();
  expect(screen.queryByText(/break saves/i)).toBeNull();
  expect(screen.queryByText(/depend on it/i)).toBeNull();
});
