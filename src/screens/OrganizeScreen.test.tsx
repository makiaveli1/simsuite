import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { OrganizeScreen } from "./OrganizeScreen";
import type { StagingPlan } from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    generateSortingPreviewPlan: vi.fn(),
    getStagingAreas: vi.fn(),
    getStagingPreviewPlan: vi.fn(),
    previewOrganization: vi.fn(),
    applyPreviewOrganization: vi.fn(),
    listSnapshots: vi.fn(),
    restoreSnapshot: vi.fn(),
    listRulePresets: vi.fn(),
  },
}));

const previewPlan: StagingPlan = {
  id: "organize-preview-plan-test",
  createdAt: "2026-05-13T00:00:00.000Z",
  source: "organize",
  status: "preview_only",
  title: "Suggested organization preview",
  summary:
    "2 Library files have preview-only organization suggestions. No files changed.",
  itemCount: 2,
  wouldTouchFiles: false,
  caveats: [
    "No files changed. This generated plan is preview-only.",
    "Suggested destinations are review aids, not file changes.",
  ],
  items: [
    {
      id: "sorting-file-101",
      fileId: 101,
      fileName:
        "VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      currentPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      suggestedDestinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      actionKind: "suggest_move",
      evidenceLevel: "evidence_backed",
      reason:
        "Package metadata suggests CAS content, so SimSuite can preview a CAS destination with caveats.",
      caveats: [
        "Review before changes. This plan does not move files.",
        "Filename-only clues are not used as strong move proof.",
      ],
      sourceSignals: ["kind:CAS", "source:mods", "configured_root:mods"],
      blockedReasons: [],
      bucket: "cas",
      confidenceLabel: "evidence-backed",
      currentRoot: "mods",
      wouldTouchFiles: false,
    },
    {
      id: "sorting-file-102",
      fileId: 102,
      fileName: "UnknownThing.package",
      currentPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\UnknownThing.package",
      suggestedDestinationPath: null,
      actionKind: "leave_in_place",
      evidenceLevel: "review_only",
      reason:
        "SimSuite has limited information here, so this file should stay in place for review.",
      caveats: ["No files changed. This plan is preview-only."],
      sourceSignals: ["kind:Unknown", "source:mods"],
      blockedReasons: ["weak_or_unknown_metadata"],
      bucket: "unknown_leave_in_place",
      confidenceLabel: "review-only",
      currentRoot: "mods",
      wouldTouchFiles: false,
    },
  ],
};

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

const manyStagedSummary = {
  areas: Array.from({ length: 7 }, (_, index) => ({
    itemId: `2026030900141${index}`,
    subdirectories: [
      {
        path: `C:\\Simsuite\\downloads_inbox\\2026030900141${index}\\batch-${index}`,
        name: `batch-${index}`,
        fileCount: 1,
        totalBytes: 1024,
        createdAt: null,
      },
    ],
  })),
  totalBytes: 7 * 1024,
  totalFileCount: 7,
};

const pendingPlan: StagingPlan = {
  id: "pending-plan-test",
  createdAt: "2026-05-14T00:00:00.000Z",
  source: "staging",
  status: "preview_only",
  title: "Staging preview plan",
  summary:
    "1 staged folder can be reviewed as a preview-only plan. No files changed.",
  itemCount: 1,
  wouldTouchFiles: false,
  caveats: [
    "Current Staging data is folder-level; per-file organization suggestions are future work.",
    "No files changed. This command is read-only.",
  ],
  items: [
    {
      id: "pending-42-1",
      fileId: null,
      fileName: "clean",
      currentPath: "C:\\Simsuite\\downloads_inbox\\42\\clean",
      suggestedDestinationPath: null,
      actionKind: "suggest_review",
      evidenceLevel: "review_only",
      reason:
        "SimSuite can see this staged folder, but v1 does not include per-file organization suggestions yet.",
      caveats: ["Folder-level plan data only; no per-file move is suggested."],
      sourceSignals: ["staging_folder_detected"],
      blockedReasons: ["per_file_staging_data_not_available"],
      bucket: "needs_review",
      confidenceLabel: "review-only",
      currentRoot: "inbox",
      wouldTouchFiles: false,
    },
  ],
};

function renderOrganize() {
  return render(
    <OrganizeScreen
      refreshVersion={0}
      onNavigate={() => {}}
      onDataChanged={() => {}}
      userView="standard"
    />,
  );
}

function enabledButtonLabels() {
  return screen
    .getAllByRole("button")
    .filter((button) => !(button as HTMLButtonElement).disabled)
    .map((button) => button.textContent ?? "");
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

it("renders Organize as a preview-only planning workspace", () => {
  renderOrganize();

  expect(
    screen.getByRole("heading", {
      name: /Review suggested organization plans/i,
    }),
  ).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.getByRole("tab", { name: /Create plan/i })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(screen.getByRole("tab", { name: /Pending plans/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Generate preview/i })).toBeEnabled();
  expect(screen.getByRole("button", { name: /Open Library/i })).toBeEnabled();
  expect(screen.getByText(/No generated plan yet/i)).toBeInTheDocument();

  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete/i,
      ),
    ]),
  );
});

it("shows pending plans inside Organize without exposing file-changing actions", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue(stagedSummary);
  vi.mocked(api.getStagingPreviewPlan).mockResolvedValue(pendingPlan);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Pending plans/i }));

  await screen.findByText(/Generated organization plans are not saved yet/i);
  expect(screen.getByRole("tab", { name: /Pending plans/i })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(screen.getByText(/Generated organization plans are not saved yet/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Pending batch 1/i)).toBeInTheDocument();
  expect(screen.getByText(/not a saved organization plan yet/i)).toBeInTheDocument();
  expect(screen.getByText(/Pending data is folder-level/i)).toBeInTheDocument();
  expect(screen.getByText(/Open Inbox/i)).toBeInTheDocument();
  expect(screen.getAllByText("2").length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Files found/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/20260309001415/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/^clean$/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Staging preview plan/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Current Staging data/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/staged folder/i)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Show technical details/i }));
  expect(screen.getByText(/Internal folder ID/i)).toBeInTheDocument();
  expect(screen.getByText(/20260309001415/i)).toBeInTheDocument();
  expect(screen.getByText(/^clean$/i)).toBeInTheDocument();

  await waitFor(() => {
    expect(api.getStagingAreas).toHaveBeenCalledTimes(1);
    expect(api.getStagingPreviewPlan).toHaveBeenCalledTimes(1);
  });
  expect(api.previewOrganization).not.toHaveBeenCalled();
  expect(api.applyPreviewOrganization).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete/i,
      ),
    ]),
  );
});

it("caps pending batch rows before showing technical details", async () => {
  vi.mocked(api.getStagingAreas).mockResolvedValue(manyStagedSummary);
  vi.mocked(api.getStagingPreviewPlan).mockResolvedValue({
    ...pendingPlan,
    itemCount: 7,
    items: [],
  });
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Pending plans/i }));

  expect(await screen.findByText(/Pending batch 1/i)).toBeInTheDocument();
  expect(screen.getByText(/Pending batch 5/i)).toBeInTheDocument();
  expect(screen.queryByText(/Pending batch 6/i)).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Show 2 more batches/i })).toBeInTheDocument();
  expect(screen.queryByText(/20260309001410/i)).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /Show 2 more batches/i }));
  expect(screen.getByText(/Pending batch 7/i)).toBeInTheDocument();
});

it("generates and renders grouped preview plan details", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue(previewPlan);
  renderOrganize();

  fireEvent.change(screen.getByLabelText(/Folder path/i), {
    target: { value: "CAS/Hair" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  await waitFor(() => {
    expect(api.generateSortingPreviewPlan).toHaveBeenCalledWith({
      scope: {
        kind: "library_folder",
        sourceLocation: "mods",
        folderPath: "CAS/Hair",
        recursive: true,
        limit: 60,
      },
    });
  });

  expect(await screen.findByText(/Suggested organization preview/i)).toBeInTheDocument();
  expect(screen.getByRole("region", { name: /CAS suggestions/i })).toBeInTheDocument();
  expect(
    screen.getByRole("region", { name: /Unknown \/ Leave in place suggestions/i }),
  ).toBeInTheDocument();
  expect(screen.getAllByText(/Suggested destination/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Why SimSuite suggested this/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Package metadata suggests CAS content/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Source signals/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/kind:CAS/i)).toBeInTheDocument();
  expect(screen.getByText(/Blocked reasons/i)).toBeInTheDocument();
  expect(screen.getByText(/weak_or_unknown_metadata/i)).toBeInTheDocument();
  expect(screen.getAllByText(/VeryLongCreatorName/i).length).toBeGreaterThan(0);

  expect(api.previewOrganization).not.toHaveBeenCalled();
  expect(api.applyPreviewOrganization).not.toHaveBeenCalled();
  expect(api.listSnapshots).not.toHaveBeenCalled();
  expect(api.restoreSnapshot).not.toHaveBeenCalled();
});

it("shows a blocked state when the generator has no plan items", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue({
    ...previewPlan,
    status: "blocked",
    summary: "No supported files were found for this bounded preview.",
    itemCount: 0,
    items: [],
  });
  renderOrganize();

  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  expect((await screen.findAllByText(/Blocked/i)).length).toBeGreaterThan(0);
  expect(screen.getByText(/No supported files were found/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);

  expect(api.applyPreviewOrganization).not.toHaveBeenCalled();
});

it("shows an error state when preview generation fails", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockRejectedValue(
    new Error("fixture failure"),
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  expect(
    await screen.findByText(/Could not generate preview plan/i),
  ).toBeInTheDocument();
  expect(screen.getByText(/fixture failure/i)).toBeInTheDocument();

  expect(api.applyPreviewOrganization).not.toHaveBeenCalled();
});
