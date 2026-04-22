import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { LibraryDetailSheet } from "./LibraryDetailSheet";

vi.mock("../../components/DockSectionStack", () => ({
  DockSectionStack: () => <div data-testid="dock-section-stack" />,
}));

afterEach(() => {
  cleanup();
});

it("surfaces family role context and script clues in inspect mode", () => {
  render(
    <LibraryDetailSheet
      open
      mode="inspect"
      selectedFile={
        {
          id: 4,
          filename: "MCCC_MCCommandCenter.ts4script",
          path: "C:\\Games\\The Sims 4\\Mods\\MCCC_MCCommandCenter.ts4script",
          kind: "ScriptMods",
          subtype: "Core",
          confidence: 0.96,
          creator: "Deaderpool",
          insights: {
            format: "ts4script-zip",
            resourceSummary: ["Archive entries: 124", "Top-level namespaces: 2"],
            scriptNamespaces: ["deaderpool", "mccc"],
            embeddedNames: [],
            creatorHints: ["Deaderpool"],
            versionHints: ["2026.2.0"],
            versionSignals: [],
            familyHints: ["mccc"],
          },
        } as never
      }
      sections={[]}
      userView="standard"
      onClose={() => {}}
    />,
  );

  expect(screen.getByText(/^mccc$/i)).toBeInTheDocument();
  expect(screen.getByText(/^core$/i)).toBeInTheDocument();
  expect(screen.getByText(/version 2026.2.0/i)).toBeInTheDocument();
  expect(screen.getByText(/deaderpool\+1/i)).toBeInTheDocument();
});
