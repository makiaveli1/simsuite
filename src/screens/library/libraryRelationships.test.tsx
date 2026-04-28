import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import {
  buildSheetRelationshipsSection,
  computeDetailLibraryRelationship,
} from "./libraryDisplay";

afterEach(() => {
  cleanup();
});

it("keeps relationship sheet copy narrow, honest, and scope-labeled", () => {
  const selected = {
    id: 10,
    filename: "CoreHelper.ts4script",
    path: "Mods\\CoreHelper\\CoreHelper.ts4script",
    kind: "ScriptMods",
    sourceLocation: "mods",
    creator: "Helper Studio",
    sameFolderPeerCount: undefined,
    samePackPeerCount: 0,
    duplicateTypes: [],
    duplicatesCount: 0,
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
    },
  ];
  const relationship = computeDetailLibraryRelationship(selected as never, peers as never);

  render(<>{buildSheetRelationshipsSection(selected as never, relationship, "standard")}</>);

  expect(screen.getByText(/^Relationship hint$/i)).toBeVisible();
  expect(screen.getByText(/same folder/i)).toBeVisible();
  expect(screen.getByText(/^Visible-only$/i)).toBeVisible();
  expect(screen.getByText(/relationship hint only/i)).toBeVisible();
  expect(screen.queryByText(/dependency/i)).toBeNull();
  expect(screen.queryByText(/safe to delete/i)).toBeNull();
  expect(screen.queryByText(/mesh/i)).toBeNull();
  expect(screen.queryByText(/recolor/i)).toBeNull();
});
