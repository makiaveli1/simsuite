import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { OrganizeScreen } from "./OrganizeScreen";
import type {
  ApplyPlanListItem,
  ApplyPlanValidationPreview,
  PersistedApplyPlan,
  StagingPlan,
} from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    generateSortingPreviewPlan: vi.fn(),
    buildApplyPlanFromStagingPlan: vi.fn(),
    saveApplyPlanPreview: vi.fn(),
    listSavedApplyPlans: vi.fn(),
    getApplyPlan: vi.fn(),
    previewApplyPlanValidation: vi.fn(),
    deleteDraftApplyPlan: vi.fn(),
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

const savedPlanSummary: ApplyPlanListItem = {
  id: 701,
  sourceStagingPlanId: "organize-preview-plan-test",
  sourcePlanKind: "sorting_preview",
  title: "Suggested organization preview",
  summary:
    "2 Library files have preview-only organization suggestions. No files changed.",
  status: "draft",
  wouldTouchFiles: false,
  confirmationRequired: true,
  backupRequired: true,
  restoreAvailable: false,
  totalItems: 2,
  applyableItems: 0,
  blockedItems: 1,
  reviewOnlyItems: 1,
  createdAt: "2026-05-15T10:00:00.000Z",
  updatedAt: "2026-05-15T10:00:00.000Z",
};

const savedPlanDetails: PersistedApplyPlan = {
  ...savedPlanSummary,
  caveats: [
    "No files changed. This saved plan is a draft preview record.",
    "Future validation is required before any file-changing workflow exists.",
  ],
  sourceScope: {
    kind: "library_folder",
    sourceLocation: "mods",
    folderPath: "CAS/Hair",
    recursive: true,
    limit: 60,
  },
  scanSessionId: null,
  items: [
    {
      id: 9001,
      applyPlanId: 701,
      sourceItemId: "sorting-file-101",
      fileId: 101,
      fileName:
        "VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      currentPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      currentRoot: "mods",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      destinationRoot: "mods",
      actionKind: "suggest_move",
      evidenceLevel: "evidence_backed",
      bucket: "cas",
      confidenceLabel: "evidence-backed",
      itemStatus: "draft_candidate",
      blocked: false,
      reviewOnly: false,
      validationStatus: null,
      conflictStatus: null,
      pathPrivacyLevel: "local_full_path_required",
      createdAt: "2026-05-15T10:00:00.000Z",
      updatedAt: "2026-05-15T10:00:00.000Z",
      signals: [
        {
          id: 9101,
          applyPlanItemId: 9001,
          signalKind: "source_signal",
          signalLabel: "kind:CAS",
          signalValue: "kind:CAS",
          evidenceLevel: "evidence_backed",
          sourceSystem: "staging_plan",
          createdAt: "2026-05-15T10:00:00.000Z",
        },
      ],
      blockers: [],
    },
    {
      id: 9002,
      applyPlanId: 701,
      sourceItemId: "sorting-file-102",
      fileId: 102,
      fileName: "UnknownThing.package",
      currentPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\UnknownThing.package",
      currentRoot: "mods",
      destinationPath: null,
      destinationRoot: null,
      actionKind: "leave_in_place",
      evidenceLevel: "review_only",
      bucket: "unknown_leave_in_place",
      confidenceLabel: "review-only",
      itemStatus: "blocked",
      blocked: true,
      reviewOnly: true,
      validationStatus: null,
      conflictStatus: null,
      pathPrivacyLevel: "local_full_path_required",
      createdAt: "2026-05-15T10:00:00.000Z",
      updatedAt: "2026-05-15T10:00:00.000Z",
      signals: [
        {
          id: 9102,
          applyPlanItemId: 9002,
          signalKind: "source_signal",
          signalLabel: "kind:Unknown",
          signalValue: "kind:Unknown",
          evidenceLevel: "review_only",
          sourceSystem: "staging_plan",
          createdAt: "2026-05-15T10:00:00.000Z",
        },
      ],
      blockers: [
        {
          id: 9202,
          applyPlanItemId: 9002,
          blockerKind: "blocked_reason",
          reasonCode: "weak_or_unknown_metadata",
          message: "weak_or_unknown_metadata",
          sourceSystem: "staging_plan",
          createdAt: "2026-05-15T10:00:00.000Z",
        },
      ],
    },
  ],
};

const validationPreview: ApplyPlanValidationPreview = {
  planId: 701,
  status: "blocked",
  canProceedToConfirmation: false,
  checkedAt: "2026-05-15T11:00:00.000Z",
  summary: {
    totalItems: 2,
    blockedItems: 1,
    reviewOnlyItems: 1,
    conflictItems: 1,
    staleItems: 0,
    missingSourceItems: 0,
    destinationConflictItems: 1,
    backupBlockedItems: 2,
  },
  caveats: [
    "No files changed. This validation preview is read-only.",
    "Backup/restore is required before any future confirmation.",
  ],
  items: [
    {
      itemId: 9001,
      fileId: 101,
      fileName:
        "VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name_That_Should_Remain_Visible.package",
      validationStatus: "destination_exists",
      conflictStatus: "destination_exists",
      blocked: true,
      reviewOnly: false,
      canApplyLater: false,
      reasons: ["A file already exists at the suggested destination."],
      requiredNextSteps: ["Choose a different destination in a future review step."],
    },
    {
      itemId: 9002,
      fileId: 102,
      fileName: "UnknownThing.package",
      validationStatus: "review_only_blocked",
      conflictStatus: "not_checked",
      blocked: true,
      reviewOnly: true,
      canApplyLater: false,
      reasons: ["Saved draft item is review-only."],
      requiredNextSteps: ["Review this item manually before future validation."],
    },
  ],
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
  expect(screen.getByRole("tab", { name: /Saved plans/i })).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: /Pending batches/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /Generate preview/i })).toBeEnabled();
  expect(screen.getByRole("button", { name: /Saved plans/i })).toBeEnabled();
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

  fireEvent.click(screen.getByRole("tab", { name: /Pending batches/i }));

  await screen.findByText(/Imported and downloaded batches are summarized here/i);
  expect(screen.getByRole("tab", { name: /Pending batches/i })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(screen.getByText(/Saved organization drafts live in Saved plans/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Pending batch 1/i)).toBeInTheDocument();
  expect(screen.getByText(/not a saved organization plan yet/i)).toBeInTheDocument();
  expect(screen.getByText(/Pending data is folder-level/i)).toBeInTheDocument();
  expect(screen.getByText(/Open Inbox/i)).toBeInTheDocument();
  expect(screen.getByText(/Inbox owns their detailed review/i)).toBeInTheDocument();
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

  fireEvent.click(screen.getByRole("tab", { name: /Pending batches/i }));

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
  expect(screen.getByRole("button", { name: /Save preview plan/i })).toBeEnabled();
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

it("saves a generated preview plan through the backend-owned builder", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue(previewPlan);
  vi.mocked(api.buildApplyPlanFromStagingPlan).mockResolvedValue({
    planId: savedPlanSummary.id,
    plan: savedPlanSummary,
  });
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  renderOrganize();

  fireEvent.change(screen.getByLabelText(/Folder path/i), {
    target: { value: "CAS/Hair" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  expect(await screen.findByText(/Suggested organization preview/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Save preview plan/i }));

  await waitFor(() => {
    expect(api.buildApplyPlanFromStagingPlan).toHaveBeenCalledWith({
      previewRequest: {
        scope: {
          kind: "library_folder",
          sourceLocation: "mods",
          folderPath: "CAS/Hair",
          recursive: true,
          limit: 60,
        },
      },
      sourcePlanKind: "sorting_preview",
    });
  });

  expect(api.saveApplyPlanPreview).not.toHaveBeenCalled();
  expect(await screen.findByText(/Saved as draft preview plan/i)).toBeInTheDocument();
  expect(screen.getByRole("tab", { name: /Saved plans/i })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect((await screen.findAllByText(/Draft preview plans/i)).length).toBeGreaterThan(0);
  expect(await screen.findByText(/Plan details/i)).toBeInTheDocument();
  expect(screen.getByText(/kind:CAS/i)).toBeInTheDocument();
  expect(screen.getByText(/weak_or_unknown_metadata/i)).toBeInTheDocument();
});

it("lists saved draft plans without dumping paths, opens details, and cancels drafts safely", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.deleteDraftApplyPlan).mockResolvedValue({
    planId: savedPlanSummary.id,
    cancelled: true,
    status: "cancelled",
  });
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));

  expect(await screen.findByText(/Draft preview plans/i)).toBeInTheDocument();
  expect(screen.getByText(/Suggested organization preview/i)).toBeInTheDocument();
  expect(screen.queryByText(/C:\\Users\\Player/i)).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /Review details/i }));

  expect(await screen.findByText(/Plan details/i)).toBeInTheDocument();
  expect(screen.getByText(/Source scope/i)).toBeInTheDocument();
  expect(screen.getByText(/kind:CAS/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Blockers/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/weak_or_unknown_metadata/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Technical details/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);

  fireEvent.click(screen.getByRole("button", { name: /^Cancel draft$/i }));
  expect(
    screen.getByText(/This only cancels the saved draft record/i),
  ).toBeInTheDocument();
  expect(screen.getByText(/It does not touch files/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Confirm cancel draft/i }));

  await waitFor(() => {
    expect(api.deleteDraftApplyPlan).toHaveBeenCalledWith(savedPlanSummary.id);
  });
  expect(await screen.findByText(/Draft cancelled/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply/i,
      ),
    ]),
  );
});

it("shows validation preview details through the backend-owned preview API", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.previewApplyPlanValidation).mockResolvedValue(validationPreview);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));

  expect(await screen.findByText(/Draft preview plans/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Review details/i }));

  expect(await screen.findByText(/Plan details/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Validation preview/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Needs validation/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Check saved plan/i }));

  await waitFor(() => {
    expect(api.previewApplyPlanValidation).toHaveBeenCalledWith({
      planId: savedPlanSummary.id,
    });
  });

  expect((await screen.findAllByText(/Future confirmation blocked/i)).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Destination exists/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Review-only/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/No files changed. This validation preview is read-only./i)).toBeInTheDocument();
  expect(screen.getByText(/Backup\/restore is required/i)).toBeInTheDocument();
  expect(screen.getByText(/A file already exists at the suggested destination/i)).toBeInTheDocument();
  expect(screen.getByText(/Review this item manually before future validation/i)).toBeInTheDocument();
  expect(screen.getByText(/Backup required/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/Ready to apply/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Can apply/i)).not.toBeInTheDocument();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("explains valid preview-only validation without implying Apply readiness", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.previewApplyPlanValidation).mockResolvedValue({
    ...validationPreview,
    status: "valid_preview_only",
    summary: {
      ...validationPreview.summary,
      blockedItems: 0,
      reviewOnlyItems: 0,
      conflictItems: 0,
      destinationConflictItems: 0,
      backupBlockedItems: 0,
    },
    items: [
      {
        ...validationPreview.items[0],
        validationStatus: "valid_preview_only",
        conflictStatus: "none",
        blocked: false,
        reviewOnly: false,
        reasons: ["No current validation blocker was found in this preview."],
      },
    ],
  });
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Check saved plan/i }));

  expect(
    await screen.findByText(/No current blocker found, but still preview-only/i),
  ).toBeInTheDocument();
  expect(screen.getByText(/Apply is still not available/i)).toBeInTheDocument();
  expect(screen.queryByText(/Ready to apply/i)).not.toBeInTheDocument();
  expect(screen.queryByText(/Safe to move/i)).not.toBeInTheDocument();
});

it("shows validation preview errors safely", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.previewApplyPlanValidation).mockRejectedValue(
    new Error("validation unavailable"),
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Check saved plan/i }));

  expect(await screen.findByText(/Could not load validation preview/i)).toBeInTheDocument();
  expect(screen.getByText(/validation unavailable/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
});

it("shows saved-plan empty and error states as preview-only", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValueOnce([]);
  const { unmount } = renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));

  expect(await screen.findByText(/No saved preview plans yet/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  unmount();

  vi.mocked(api.listSavedApplyPlans).mockRejectedValueOnce(new Error("list failed"));
  renderOrganize();
  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));

  expect(await screen.findByText(/list failed/i)).toBeInTheDocument();
  expect(screen.getByText(/No saved preview plans yet/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
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
