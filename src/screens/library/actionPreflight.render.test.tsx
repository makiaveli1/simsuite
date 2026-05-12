import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { ActionPreflightDetail, buildFileActionPreflight } from "./actionPreflight";

afterEach(() => {
  cleanup();
});

it("renders preflight evidence with cautious labels instead of over-strong confirmation copy", () => {
  const preflight = buildFileActionPreflight(
    {
      id: 18,
      filename: "MCCC_MCCommandCenter.ts4script",
      path: "Mods\\MCCC\\MCCC_MCCommandCenter.ts4script",
      kind: "ScriptMods",
      sourceLocation: "mods",
      confidence: 0.78,
      safetyNotes: [],
      parserWarnings: [],
      duplicatesCount: 1,
      duplicateTypes: ["exact"],
      problemSignals: [
        {
          signalType: "review_suggested",
          severity: "warning",
          proofLevel: "confirmed",
          shortLabel: "Review suggested",
          explanation: "SimSuite found warning signals that are worth a closer look.",
          source: "review_queue",
          evidence: ["Category could not be confirmed"],
          destination: "review",
          showInLibrary: true,
          showInInspector: true,
          showInMoreDetails: true,
          showInNeedsReview: true,
        },
        {
          signalType: "no_update_source",
          severity: "info",
          proofLevel: "confirmed",
          shortLabel: "No update source",
          explanation: "SimSuite is not tracking an update source for this file yet.",
          source: "watch_status",
          evidence: ["watch_status = not_watched"],
          destination: "updates",
          showInLibrary: true,
          showInInspector: true,
          showInMoreDetails: true,
          showInNeedsReview: false,
        },
      ],
    } as never,
    {
      type: "same_folder",
      proofLevel: "fact",
      label: "Same folder",
      peerCount: 2,
      countScope: "visible_only",
      evidenceSource: "filtered_folder",
    },
    "change",
  );

  render(<ActionPreflightDetail preflight={preflight} />);

  expect(screen.getAllByText(/detected/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/^confirmed$/i)).toBeNull();
  expect(screen.queryByText(/safe to delete|missing mesh|dependency|definitely outdated/i)).toBeNull();
});
