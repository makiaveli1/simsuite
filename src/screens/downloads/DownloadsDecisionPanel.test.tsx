import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DownloadsDecisionPanel } from "./DownloadsDecisionPanel";

function enabledButtonNames() {
  return screen
    .getAllByRole("button")
    .filter((button) => !(button as HTMLButtonElement).disabled)
    .map((button) => button.textContent ?? "");
}

it("keeps Inbox decisions preview-only when file actions are blocked", () => {
  render(
    <DownloadsDecisionPanel
      userView="standard"
      title="SpringRefreshPack.zip"
      laneLabel="Ready for review"
      badges={[]}
      signals={[]}
      summary="No files changed."
      nextStepTitle="Review this inbox batch"
      nextStepDescription="Use Inbox to understand what arrived before creating a preview plan."
      primaryActionLabel="Apply"
      onPrimaryAction={() => {}}
      secondaryActionLabel="Reject"
      onSecondaryAction={() => {}}
      onOpenProof={() => {}}
      proofSummary="Open the files and proof when you need the full story."
      fileActionsBlocked
    />,
  );

  expect(screen.getByText(/Not ready to apply yet/i)).toBeVisible();
  expect(screen.getByText(/Inbox is review-only/i)).toBeVisible();
  expect(enabledButtonNames()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|reject|move files|clean up|quarantine|delete|commit|fix/i,
      ),
    ]),
  );
});
