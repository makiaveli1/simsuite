import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DownloadsDecisionPanel } from "./DownloadsDecisionPanel";

it("shows one primary action and keeps proof behind a secondary action", () => {
  render(
    <DownloadsDecisionPanel
      userView="standard"
      title="SpringRefreshPack.zip"
      laneLabel="Ready now"
      badges={[]}
      signals={[]}
      summary="No safe hand-off is ready yet."
      nextStepTitle="Keep this batch in staging"
      nextStepDescription="Nothing safe can move until the rest of the proof is checked."
      primaryActionLabel="Apply"
      onPrimaryAction={() => {}}
      secondaryActionLabel="Reject"
      onSecondaryAction={() => {}}
      onOpenProof={() => {}}
      proofSummary="Open the files and proof when you need the full story."
    />,
  );

  expect(screen.getByRole("button", { name: /apply/i })).toBeVisible();
  expect(screen.getByRole("button", { name: /reject/i })).toBeVisible();
});
