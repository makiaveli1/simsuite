import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
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

it("shows staged content as preview-only and does not expose file-changing actions", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue(stagedSummary);

  render(<StagingScreen onNavigate={() => {}} userView="standard" />);

  expect(await screen.findByRole("heading", { name: /staging/i })).toBeInTheDocument();
  expect(screen.getByText(/Staging is preview-only right now/i)).toBeInTheDocument();
  expect(screen.getByText(/No files will be changed from this screen yet/i)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /User confirmation required/i })).toBeDisabled();
  expect(screen.getByRole("button", { name: /Preview only/i })).toBeDisabled();

  expect(screen.queryByRole("button", { name: /Commit/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Reject/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

  await waitFor(() => {
    expect(api.getStagingAreas).toHaveBeenCalledTimes(1);
  });
  expect(api.cleanupStagingAreas).not.toHaveBeenCalled();
  expect(api.commitStagingArea).not.toHaveBeenCalled();
  expect(api.commitAllStagingAreas).not.toHaveBeenCalled();
});

