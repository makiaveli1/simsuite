import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { StagingScreen } from "./StagingScreen";

vi.mock("../lib/api", () => ({
  api: {
    getStagingAreas: vi.fn(),
    getStagingPreviewPlan: vi.fn(),
    cleanupStagingAreas: vi.fn(),
    commitStagingArea: vi.fn(),
    commitAllStagingAreas: vi.fn(),
  },
}));

const stagedSummary = {
  areas: [
    {
      itemId: "20260309001415",
      subdirectories: [
        {
          path: "C:\\Simsuite\\downloads_inbox\\20260309001415\\clean",
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

const previewPlan = {
  id: "staging-preview-plan-v1",
  createdAt: "2026-05-13T00:00:00.000Z",
  source: "staging" as const,
  status: "preview_only" as const,
  title: "Staging preview plan",
  summary:
    "1 staged folder can be reviewed as a preview-only plan. No files changed.",
  itemCount: 1,
  wouldTouchFiles: false as const,
  caveats: [
    "Current Staging data is folder-level; per-file organization suggestions are future work.",
    "No files changed. This command is read-only.",
  ],
  items: [
    {
      id: "staging-42-1",
      fileId: null,
      fileName: "clean",
      currentPath: "C:\\Simsuite\\downloads_inbox\\42\\clean",
      suggestedDestinationPath: null,
      actionKind: "suggest_review" as const,
      evidenceLevel: "review_only" as const,
      reason:
        "SimSuite can see this staged folder, but v1 does not include per-file organization suggestions yet.",
      caveats: ["Folder-level plan data only; no per-file move is suggested."],
      sourceSignals: ["staging_folder_detected"],
      blockedReasons: ["per_file_staging_data_not_available"],
      bucket: "needs_review" as const,
      confidenceLabel: "review-only" as const,
      currentRoot: "inbox" as const,
      wouldTouchFiles: false as const,
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("shows pending plan content as preview-only and does not expose file-changing actions", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue(stagedSummary);
  vi.mocked(api.getStagingPreviewPlan).mockResolvedValue(previewPlan);
  const onNavigate = vi.fn();

  render(<StagingScreen onNavigate={onNavigate} userView="standard" />);

  expect(await screen.findByRole("heading", { name: /Plan Preview/i })).toBeInTheDocument();
  expect(screen.getByText(/main workflow now lives in Organize/i)).toBeInTheDocument();
  expect(screen.getByText(/No files will be changed from this route/i)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Open Organize/i })).toBeEnabled();
  expect(screen.getByRole("region", { name: /Preview plan/i })).toBeInTheDocument();
  expect(screen.getAllByText(/Plan Preview/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Pending data is folder-level/i)).toBeInTheDocument();
  expect(screen.getByText(/Pending batch 1/i)).toBeInTheDocument();
  expect(screen.getByText(/not a saved organization plan yet/i)).toBeInTheDocument();
  expect(screen.queryByText(/20260309001415/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/^clean$/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Staging preview plan/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Current Staging data/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/staged folder/i)).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: /User confirmation required/i })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: /Show technical details/i }));
  expect(screen.getByText(/Internal folder ID/i)).toBeInTheDocument();
  expect(screen.getByText(/20260309001415/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Open Organize/i }));
  expect(onNavigate).toHaveBeenCalledWith("organize");

  expect(screen.queryByRole("button", { name: /Commit/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Reject/i })).not.toBeInTheDocument();
  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

  await waitFor(() => {
    expect(api.getStagingAreas).toHaveBeenCalledTimes(1);
    expect(api.getStagingPreviewPlan).toHaveBeenCalledTimes(1);
  });
  expect(api.cleanupStagingAreas).not.toHaveBeenCalled();
  expect(api.commitStagingArea).not.toHaveBeenCalled();
  expect(api.commitAllStagingAreas).not.toHaveBeenCalled();
});

it("shows a blocked preview plan without exposing file-changing actions", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue({
    areas: [],
    totalBytes: 0,
    totalFileCount: 0,
  });
  vi.mocked(api.getStagingPreviewPlan).mockResolvedValue({
    ...previewPlan,
    status: "blocked",
    summary: "No pending plan content is available for a preview plan.",
    itemCount: 0,
    items: [],
  });

  render(<StagingScreen onNavigate={() => {}} userView="standard" />);

  expect(await screen.findByText(/No saved organization plans yet/i)).toBeInTheDocument();
  expect(screen.getByText(/Not ready to apply yet/i)).toBeInTheDocument();
  expect(screen.getByText(/No saved organization plans yet/i)).toBeInTheDocument();
  expect(screen.getByText(/Generated plans are not saved yet/i)).toBeInTheDocument();

  expect(api.cleanupStagingAreas).not.toHaveBeenCalled();
  expect(api.commitStagingArea).not.toHaveBeenCalled();
  expect(api.commitAllStagingAreas).not.toHaveBeenCalled();
});
