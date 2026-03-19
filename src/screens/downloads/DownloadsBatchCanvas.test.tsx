import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { DownloadsBatchCanvas } from "./DownloadsBatchCanvas";

it("shows the waiting-on-you decision story for that lane", () => {
  render(
    <DownloadsBatchCanvas
      lane="waiting_on_you"
      userView="standard"
      selectionTitle="Adeepindigo_Healthcare_Redux_Addon.zip"
      summary="This batch needs one more choice from you before it can move."
      safeCount={0}
      reviewCount={1}
      unchangedCount={0}
      previewItems={[]}
    />,
  );

  expect(screen.getByText(/needs one more choice/i)).toBeVisible();
});
