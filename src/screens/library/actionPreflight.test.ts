import { expect, it } from "vitest";
import { buildFileActionPreflight } from "./actionPreflight";

it("builds calm preflight cautions without fake dependency claims", () => {
  const preflight = buildFileActionPreflight(
    {
      id: 12,
      filename: "CoreHelper.ts4script",
      path: "Mods\\CoreHelper\\CoreHelper.ts4script",
      kind: "ScriptMods",
      sourceLocation: "mods",
      confidence: 0.41,
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
          evidence: ["Script file is nested deeper than the safe script depth"],
          destination: "review",
          showInLibrary: true,
          showInInspector: true,
          showInMoreDetails: true,
          showInNeedsReview: true,
        },
        {
          signalType: "duplicate_candidate",
          severity: "caution",
          proofLevel: "detected",
          shortLabel: "Possible duplicate",
          explanation: "Compare before changing either copy.",
          source: "duplicates",
          evidence: ["Duplicate comparison rule matched"],
          destination: "duplicates",
          showInLibrary: false,
          showInInspector: true,
          showInMoreDetails: true,
          showInNeedsReview: false,
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
          showInLibrary: false,
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

  expect(preflight.title).toMatch(/before you change this file/i);
  expect(preflight.decision).toBe("caution");
  expect(preflight.recommendedRoute).toBe("review");
  expect(preflight.signals.map((signal) => signal.label)).toEqual(
    expect.arrayContaining([
      "Review suggested",
      "Possible duplicate",
      "No update source",
      "Same folder",
    ]),
  );
  expect(preflight.signals.some((signal) => /placement clue, not proof of a required relationship/i.test(signal.explanation))).toBe(true);
  expect(preflight.signals.some((signal) => /safe to delete|mesh|recolor|required by|used by|depend on each other/i.test(signal.explanation))).toBe(false);
});

it("stays quiet when a file has no preflight signals", () => {
  const preflight = buildFileActionPreflight(
    {
      id: 5,
      filename: "CozyKitchen.package",
      path: "Mods\\BuildBuy\\CozyKitchen.package",
      kind: "BuildBuy",
      sourceLocation: "mods",
      confidence: 0.92,
      safetyNotes: [],
      parserWarnings: [],
      duplicatesCount: 0,
      duplicateTypes: [],
      problemSignals: [],
    } as never,
    null,
    "change",
  );

  expect(preflight.signals).toHaveLength(0);
  expect(preflight.decision).toBe("allow");
});
