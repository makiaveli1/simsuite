import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { StagingScreen } from "./StagingScreen";

vi.mock("../lib/api", () => ({
  api: {
    getStagingAreas: vi.fn(),
    cleanupStagingAreas: vi.fn(),
    commitStagingArea: vi.fn(),
    commitAllStagingAreas: vi.fn(),
  },
}));

const stagedSummary = {
  areas: [
    {
      itemId: "42",
      subdirectories: [
        {
          path: "C:\\Simsuite\\downloads_inbox\\42\\clean",
          name: "clean",
          fileCount: 2,
          totalBytes: 2048,
          createdAt: null,
        },
      ],
    },
  ],
  totalBytes: 2048,
  totalFileCount: 2,
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("uses an in-app confirmation dialog before rejecting staged files", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue(stagedSummary);
  vi.mocked(api.cleanupStagingAreas).mockResolvedValue({
    deletedCount: 1,
    freedBytes: 2048,
    errors: [],
  });
  const confirmSpy = vi.spyOn(globalThis, "confirm").mockReturnValue(true);

  render(<StagingScreen onNavigate={() => {}} userView="standard" />);

  fireEvent.click(await screen.findByRole("button", { name: /^reject$/i }));

  expect(confirmSpy).not.toHaveBeenCalled();
  expect(await screen.findByRole("dialog", { name: /reject staged files/i })).toBeInTheDocument();
  expect(api.cleanupStagingAreas).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: /remove staged files/i }));

  await waitFor(() => {
    expect(api.cleanupStagingAreas).toHaveBeenCalledWith([
      "C:\\Simsuite\\downloads_inbox\\42\\clean",
    ]);
  });
});
