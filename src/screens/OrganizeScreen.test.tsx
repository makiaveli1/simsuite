import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { OrganizeScreen } from "./OrganizeScreen";
import type {
  ApplyPlanListItem,
  ApplyPlanConfirmationTokenReceipt,
  ApplyPlanDryRunPreview,
  PersistedApplyPlanRestoreEntry,
  PersistedApplyPlanResult,
  PersistedApplyPlanRun,
  ApplyPlanValidationPreview,
  ApplyPlanOperationPreview,
  PersistedApplyPlan,
  GenerateSortingPreviewPlanResult,
  StagingPlan,
} from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    generateSortingPreviewPlan: vi.fn(),
    saveApplyPlanFromPreviewSnapshot: vi.fn(),
    buildApplyPlanFromStagingPlan: vi.fn(),
    saveApplyPlanPreview: vi.fn(),
    listSavedApplyPlans: vi.fn(),
    getApplyPlan: vi.fn(),
    previewApplyPlanValidation: vi.fn(),
    previewApplyPlanDryRun: vi.fn(),
    previewApplyPlanOperations: vi.fn(),
    issueApplyPlanConfirmationToken: vi.fn(),
    createApplyPlanRunLog: vi.fn(),
    listApplyPlanRunLogs: vi.fn(),
    getApplyPlanRunLog: vi.fn(),
    recordApplyPlanResultLog: vi.fn(),
    listApplyPlanResultLogs: vi.fn(),
    recordApplyPlanRestoreEntry: vi.fn(),
    listApplyPlanRestoreEntries: vi.fn(),
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

const previewResult: GenerateSortingPreviewPlanResult = {
  plan: previewPlan,
  previewSnapshotId: 1701,
  previewSnapshotHash:
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
  previewSnapshotHashVersion: "apply_plan_preview_snapshot_v3",
  previewSnapshotHashAlgorithm: "sha256",
  previewSnapshotCreatedAt: "2026-05-15T09:30:00.000Z",
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
  sourcePlanKind: "backend_generated_sorting_preview",
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
  planHash:
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  planHashVersion: "apply_plan_hash_v1",
  planHashAlgorithm: "sha256",
  planHashCreatedAt: "2026-05-15T10:00:00.000Z",
  previewSnapshotId: 1701,
  previewSnapshotHash:
    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
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
  folderConfig: {
    mode: "custom",
    label: "Custom Organize folder profile",
    bucketFolders: { cas: "CAS/Hair" },
    creatorFolderMode: "when_available",
    categoryFolderMode: "bucket_and_category",
    maxDepth: 3,
    exclusionPatterns: ["keep loose overrides in review"],
  },
  contextTrail: [
    {
      sourceSystem: "library",
      signalKind: "indexed_scope",
      label: "Library paths and package metadata feed destination suggestions",
      value: "mods",
      strength: "evidence",
    },
    {
      sourceSystem: "creator_category_audit",
      signalKind: "metadata_bucket_hints",
      label: "Creator and Category Audit confidence can shape preview buckets",
      value: "custom",
      strength: "routing",
    },
  ],
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
  planHash:
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  sourcePlanKind: "backend_generated_sorting_preview",
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

const dryRunPreview: ApplyPlanDryRunPreview = {
  planId: 701,
  status: "blocked",
  canProceedToApply: false,
  canProceedToConfirmation: false,
  checkedAt: "2026-05-15T11:05:00.000Z",
  summary: {
    totalItems: 5,
    candidateItems: 1,
    skippedItems: 3,
    blockedItems: 1,
    reviewOnlyItems: 1,
    conflictItems: 1,
    backupRequiredItems: 1,
  },
  caveats: [
    "No files changed. This dry-run preview is read-only.",
    "Apply is not ready yet.",
    "Future confirmation blocked.",
  ],
  items: [
    {
      itemId: 9001,
      fileId: 101,
      fileName: "CandidateHair.package",
      dryRunStatus: "candidate_after_future_safety_gates",
      actionPreview: "would_move_later",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\CandidateHair.package",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\CandidateHair.package",
      reasons: ["No current dry-run blocker was returned for this item."],
      blockers: [],
      requiredBeforeApply: [
        "Validation proof required",
        "Backup required",
        "Restore map required",
        "Result log required",
        "Explicit confirmation required",
        "Apply executor proof required",
      ],
      canApply: false,
    },
    {
      itemId: 9002,
      fileId: 102,
      fileName: "UnknownThing.package",
      dryRunStatus: "would_require_review",
      actionPreview: "would_skip",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\UnknownThing.package",
      destinationPath: null,
      reasons: ["Manual review needed before this item can be discussed further."],
      blockers: ["weak_or_unknown_metadata"],
      requiredBeforeApply: ["Manual review needed", "Validation proof required"],
      canApply: false,
    },
    {
      itemId: 9003,
      fileId: 103,
      fileName: "NeedsBackup.package",
      dryRunStatus: "would_require_backup",
      actionPreview: "no_action",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\NeedsBackup.package",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\NeedsBackup.package",
      reasons: ["Backup required before any future file-changing workflow."],
      blockers: ["backup_required"],
      requiredBeforeApply: ["Backup required", "Restore map required"],
      canApply: false,
    },
    {
      itemId: 9004,
      fileId: 104,
      fileName: "DestinationConflict.package",
      dryRunStatus: "would_require_destination_review",
      actionPreview: "would_skip",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\DestinationConflict.package",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\DestinationConflict.package",
      reasons: ["Destination needs review before this item can continue."],
      blockers: ["destination_exists"],
      requiredBeforeApply: ["Choose a different destination in a future review step"],
      canApply: false,
    },
    {
      itemId: 9005,
      fileId: null,
      fileName: "MissingSource.package",
      dryRunStatus: "would_skip",
      actionPreview: "would_skip",
      sourcePath: null,
      destinationPath: null,
      reasons: ["Saved source evidence is missing."],
      blockers: ["missing_source"],
      requiredBeforeApply: ["Validation proof required"],
      canApply: false,
    },
  ],
};

const operationPreview: ApplyPlanOperationPreview = {
  planId: 701,
  status: "preview_only",
  canProceedToApply: false,
  canProceedToConfirmation: false,
  checkedAt: "2026-05-15T11:10:00.000Z",
  operationSetHash:
    "bbbbbb0123456789bbbbbb0123456789bbbbbb0123456789bbbbbb0123456789",
  operationSetHashAlgorithm: "sha256",
  operationSetHashVersion: "apply_plan_operation_set_v1",
  summary: {
    totalItems: 5,
    candidateOperations: 2,
    blockedItems: 1,
    skippedItems: 3,
    reviewOnlyItems: 1,
    conflictItems: 1,
    backupRequiredItems: 1,
  },
  caveats: [
    "No files changed. This operation-set preview is read-only.",
    "Operation-set preview is not Apply; confirmation, backup, restore map, result log, and executor proof are still required.",
  ],
  operations: [
    {
      itemId: 9001,
      fileId: 101,
      fileName: "OperationCandidate.package",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\OperationCandidate.package",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\OperationCandidate.package",
      actionPreview: "would_move_later",
      reasons: ["Backend validation and dry-run classified this as a future candidate."],
      requiredBeforeApply: [
        "Validation proof required",
        "Backup required",
        "Restore map required",
        "Result log required",
        "Explicit confirmation required",
        "Apply executor proof required",
      ],
      canApply: false,
    },
    {
      itemId: 9003,
      fileId: 103,
      fileName: "OperationBackupProof.package",
      sourcePath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\OperationBackupProof.package",
      destinationPath:
        "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\OperationBackupProof.package",
      actionPreview: "no_action",
      reasons: ["Backup required before any future file-changing workflow."],
      requiredBeforeApply: ["Backup required", "Restore map required"],
      canApply: false,
    },
  ],
};

const blockedOperationPreview: ApplyPlanOperationPreview = {
  ...operationPreview,
  status: "blocked",
  summary: {
    ...operationPreview.summary,
    candidateOperations: 0,
  },
  operations: [],
  caveats: [
    ...operationPreview.caveats,
    "Client-supplied or legacy ApplyPlan previews are review/audit-only and cannot produce operation-set candidates.",
  ],
};

const confirmationReceipt: ApplyPlanConfirmationTokenReceipt = {
  token: "aptok_v1_test_confirmation_token",
  tokenId: 3101,
  planId: 701,
  planHash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  operationSetHash: operationPreview.operationSetHash,
  operationSetHashAlgorithm: operationPreview.operationSetHashAlgorithm,
  operationSetHashVersion: operationPreview.operationSetHashVersion,
  sourcePlanKind: "backend_generated_sorting_preview",
  allowedOperationCount: 2,
  singleUseState: "unused",
  issuedAt: "2026-05-15T11:12:00.000Z",
  expiresAt: null,
  canProceedToApply: false,
  canExecute: false,
  caveats: [
    "Backend-issued confirmation token V1 created for this operation-set hash.",
    "Apply executor is still locked. This token cannot move, copy, delete, quarantine, replace, or restore files.",
  ],
};

const recoveryRunLog: PersistedApplyPlanRun = {
  id: 3001,
  applyPlanId: 701,
  status: "draft_log",
  backupStrategy: "copy_backup_first",
  confirmationToken: null,
  confirmedAt: null,
  startedAt: null,
  finishedAt: null,
  totalItems: 2,
  skippedItems: 1,
  appliedItems: 0,
  failedItems: 1,
  restoredItems: 0,
  summary: "DB-only recovery history. No files changed.",
  createdAt: "2026-05-17T10:00:00.000Z",
  updatedAt: "2026-05-17T10:05:00.000Z",
};

const recoveryResultLog: PersistedApplyPlanResult = {
  id: 4001,
  applyPlanRunId: 3001,
  applyPlanId: 701,
  applyPlanItemId: 9001,
  operationKind: "future_move_preview",
  resultStatus: "pending_log",
  sourcePathAtExecution:
    "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\ResultHair.package",
  destinationPathAtExecution:
    "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\ResultHair.package",
  backupPath:
    "C:\\Users\\Player\\AppData\\Local\\SimSuite\\ApplyPlanBackups\\run-3001\\ResultHair.package",
  errorCode: null,
  errorMessage: null,
  userSummary: "Fixture proof metadata only. No files changed.",
  startedAt: null,
  finishedAt: null,
  createdAt: "2026-05-17T10:01:00.000Z",
  updatedAt: "2026-05-17T10:01:00.000Z",
};

const recoveryRestoreEntry: PersistedApplyPlanRestoreEntry = {
  id: 5001,
  applyPlanRunId: 3001,
  applyPlanResultId: 4001,
  applyPlanId: 701,
  applyPlanItemId: 9001,
  originalSourcePath:
    "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\Loose\\ResultHair.package",
  destinationPathAtExecution:
    "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods\\CAS\\ResultHair.package",
  backupPath:
    "C:\\Users\\Player\\AppData\\Local\\SimSuite\\ApplyPlanBackups\\run-3001\\ResultHair.package",
  fileHashBefore: "abc123",
  fileSizeBefore: 128,
  operationKind: "future_move_preview",
  operationResultStatus: "pending_log",
  restoreStatus: "design_only",
  restoreErrorCode: null,
  restoreErrorMessage: null,
  createdAt: "2026-05-17T10:02:00.000Z",
  updatedAt: "2026-05-17T10:02:00.000Z",
  restoredAt: null,
  failedAt: null,
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

beforeEach(() => {
  vi.mocked(api.listApplyPlanRunLogs).mockResolvedValue([]);
  vi.mocked(api.listApplyPlanResultLogs).mockResolvedValue([]);
  vi.mocked(api.listApplyPlanRestoreEntries).mockResolvedValue([]);
  vi.mocked(api.previewApplyPlanDryRun).mockResolvedValue(dryRunPreview);
  vi.mocked(api.previewApplyPlanOperations).mockResolvedValue(operationPreview);
  vi.mocked(api.issueApplyPlanConfirmationToken).mockResolvedValue(confirmationReceipt);
});

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
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue(previewResult);
  renderOrganize();

  fireEvent.change(screen.getByLabelText(/Folder path/i), {
    target: { value: "CAS/Hair" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  await waitFor(() => {
    expect(api.generateSortingPreviewPlan).toHaveBeenCalledWith(
      expect.objectContaining({
        scope: {
          kind: "library_folder",
          sourceLocation: "mods",
          folderPath: "CAS/Hair",
          recursive: true,
          limit: 60,
        },
        folderConfig: expect.objectContaining({ mode: "default" }),
      }),
    );
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
    ...previewResult,
    plan: {
      ...previewPlan,
      status: "blocked",
      summary: "No supported files were found for this bounded preview.",
      itemCount: 0,
      items: [],
    },
  });
  renderOrganize();

  fireEvent.click(screen.getByRole("button", { name: /Generate preview/i }));

  expect((await screen.findAllByText(/Blocked/i)).length).toBeGreaterThan(0);
  expect(screen.getByText(/No supported files were found/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);

  expect(api.applyPreviewOrganization).not.toHaveBeenCalled();
});

it("saves a generated preview plan through the backend-owned preview snapshot", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue(previewResult);
  vi.mocked(api.saveApplyPlanFromPreviewSnapshot).mockResolvedValue({
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
    expect(api.saveApplyPlanFromPreviewSnapshot).toHaveBeenCalledWith({
      previewSnapshotId: previewResult.previewSnapshotId,
      previewSnapshotHash: previewResult.previewSnapshotHash,
    });
  });

  expect(api.buildApplyPlanFromStagingPlan).not.toHaveBeenCalled();
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

it("saves generated previews from the preview-time snapshot after UI config changes", async () => {
  vi.mocked(api.generateSortingPreviewPlan).mockResolvedValue(previewResult);
  vi.mocked(api.saveApplyPlanFromPreviewSnapshot).mockResolvedValue({
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
  fireEvent.click(screen.getByLabelText(/Use custom folders/i));
  fireEvent.change(screen.getByLabelText(/^CAS$/i), {
    target: { value: "Changed/CAS" },
  });
  fireEvent.click(screen.getByRole("button", { name: /Save preview plan/i }));

  await waitFor(() => {
    expect(api.saveApplyPlanFromPreviewSnapshot).toHaveBeenCalledWith({
      previewSnapshotId: previewResult.previewSnapshotId,
      previewSnapshotHash: previewResult.previewSnapshotHash,
    });
  });
  expect(api.buildApplyPlanFromStagingPlan).not.toHaveBeenCalled();
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
    (await screen.findAllByText(/No current blocker found, but still preview-only/i)).length,
  ).toBeGreaterThan(0);
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

it("shows read-only confirmation design for saved plans without enabling execution", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));

  expect(await screen.findByRole("region", { name: /Confirmation design/i })).toBeInTheDocument();
  const bodyText = document.body.textContent ?? "";
  expect(bodyText).toMatch(/Confirmation Design V1/i);
  expect(bodyText).toMatch(/Backend-issued confirmation tokens bind/i);
  expect(bodyText).toMatch(/not issued in this view/i);
  expect(bodyText).toMatch(/Backend-generated sorting preview/i);
  expect(bodyText).toMatch(/Source kind: backend_generated_sorting_preview/i);
  expect(bodyText).toMatch(/Backend-owned plan hash recorded/i);
  expect(bodyText).toMatch(/0123456789ab…89abcdef/i);
  expect(bodyText).toMatch(/identity\/provenance only; it does not authorize Apply/i);
  expect(bodyText).toMatch(/Custom folder configurations/i);
  expect(bodyText).toMatch(/influences preview destinations only/i);
  expect(bodyText).toMatch(/Cross-system context trail/i);
  expect(bodyText).toMatch(/Library paths and package metadata/i);
  expect(bodyText).toMatch(/Creator and Category Audit confidence/i);
  expect(bodyText).toMatch(/Future operation list/i);
  expect(bodyText).toMatch(/VeryLongCreatorName_With_A_Long_CAS_Hair_File_Name/i);
  expect(bodyText).toMatch(/UnknownThing\.package/i);
  expect(bodyText).toMatch(/Backup must be created before any mutation/i);
  expect(bodyText).toMatch(/Restore map must be written by the backend executor/i);
  expect(bodyText).toMatch(/Validation must be re-run immediately before confirmation/i);
  expect(screen.getByRole("button", { name: /Issue read-only token/i })).toBeDisabled();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|restore now|run backup|run restore|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("shows read-only dry-run preview classifications for saved plans", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));

  expect((await screen.findAllByText(/Dry-run preview/i)).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Run a dry-run preview/i)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /Preview dry-run/i }));

  await waitFor(() => {
    expect(api.previewApplyPlanDryRun).toHaveBeenCalledWith({
      planId: savedPlanSummary.id,
    });
  });

  expect((await screen.findAllByText(/Apply is not ready yet/i)).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Future confirmation blocked/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Candidate after future safety gates/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Backup required/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Needs review/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Needs destination review/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Would be skipped/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Would be considered only after future safety gates/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Still required before any future Apply/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/canProceedToApply=false/i)).toBeInTheDocument();
  expect(screen.getByText(/canProceedToConfirmation=false/i)).toBeInTheDocument();
  expect(screen.getAllByText(/canApply=false/i).length).toBeGreaterThan(0);

  const bodyText = document.body.textContent ?? "";
  expect(bodyText).not.toMatch(/ready to apply|safe to move|confirmed safe|approved/i);
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|restore now|run backup|run restore|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("shows dry-run loading and error states without changing files", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  let resolveDryRun: (preview: ApplyPlanDryRunPreview) => void = () => {};
  vi.mocked(api.previewApplyPlanDryRun).mockReturnValueOnce(
    new Promise((resolve) => {
      resolveDryRun = resolve;
    }),
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Preview dry-run/i }));

  expect((await screen.findAllByText(/Checking dry-run preview/i)).length).toBeGreaterThan(0);

  await act(async () => {
    resolveDryRun(dryRunPreview);
  });

  expect((await screen.findAllByText(/CandidateHair\.package/i)).length).toBeGreaterThan(0);

  vi.mocked(api.previewApplyPlanDryRun).mockRejectedValueOnce(
    new Error("dry-run unavailable"),
  );
  fireEvent.click(screen.getByRole("button", { name: /Refresh dry-run preview/i }));

  expect(await screen.findByText(/Dry-run preview could not be loaded/i)).toBeInTheDocument();
  expect(screen.getByText(/dry-run unavailable/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.queryAllByText(/CandidateHair\.package/i)).toHaveLength(0);
});

it("clears dry-run preview results when switching saved plans", async () => {
  const secondPlanSummary: ApplyPlanListItem = {
    ...savedPlanSummary,
    id: 702,
    title: "Second saved dry-run draft",
    sourceStagingPlanId: "second-preview-plan-test",
  };
  const secondPlanDetails: PersistedApplyPlan = {
    ...savedPlanDetails,
    id: 702,
    title: "Second saved dry-run draft",
    sourceStagingPlanId: "second-preview-plan-test",
    items: [
      {
        ...savedPlanDetails.items[0],
        id: 9301,
        applyPlanId: 702,
        fileName: "SecondPlan.package",
        sourceItemId: "second-plan-item",
      },
    ],
  };

  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([
    savedPlanSummary,
    secondPlanSummary,
  ]);
  vi.mocked(api.getApplyPlan).mockImplementation(async (planId) =>
    planId === secondPlanSummary.id ? secondPlanDetails : savedPlanDetails,
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  const reviewButtons = await screen.findAllByRole("button", { name: /Review details/i });
  fireEvent.click(reviewButtons[0]);
  fireEvent.click(await screen.findByRole("button", { name: /Preview dry-run/i }));

  expect((await screen.findAllByText(/CandidateHair\.package/i)).length).toBeGreaterThan(0);
  fireEvent.click(await screen.findByRole("button", { name: /^Review details$/i }));

  expect(await screen.findByText(/Second saved dry-run draft/i)).toBeInTheDocument();
  expect(screen.queryAllByText(/CandidateHair\.package/i)).toHaveLength(0);
  expect(screen.getByText(/Run a dry-run preview/i)).toBeInTheDocument();
});

it("shows operation-set preview rows without enabling Apply", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Preview operation set/i }));

  expect(await screen.findByText(/Operation Set Preview V1/i)).toBeInTheDocument();
  expect((await screen.findAllByText(/OperationCandidate\.package/i)).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/OperationBackupProof\.package/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Preview operation/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Preview operations/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/operation canApply=false/i)).toBeInTheDocument();
  expect(screen.getAllByText(/canApply=false/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/canProceedToApply=false/i)).toBeInTheDocument();
  expect(screen.getByText(/canProceedToConfirmation=false/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);

  await waitFor(() => {
    expect(api.previewApplyPlanOperations).toHaveBeenCalledWith({
      planId: savedPlanSummary.id,
      expectedPlanHash: savedPlanDetails.planHash,
    });
  });
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|restore now|run backup|run restore|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("issues a backend confirmation token from an operation-set hash without unlocking Apply", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.listApplyPlanRunLogs).mockResolvedValue([
    {
      ...recoveryRunLog,
      id: confirmationReceipt.tokenId,
      confirmationToken: confirmationReceipt.token,
      summary: "confirmation_token_v1 read-only record. No files changed.",
    },
  ]);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Preview operation set/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Issue read-only token/i }));

  await waitFor(() => {
    expect(api.issueApplyPlanConfirmationToken).toHaveBeenCalledWith({
      planId: savedPlanSummary.id,
      expectedPlanHash: savedPlanDetails.planHash,
      expectedOperationSetHash: operationPreview.operationSetHash,
    });
  });

  const bodyText = document.body.textContent ?? "";
  expect(bodyText).toMatch(/Backend-issued confirmation token/i);
  expect(bodyText).toMatch(/Executor still locked/i);
  expect(bodyText).toMatch(/singleUseState=unused/i);
  expect(bodyText).toMatch(/canExecute=false/i);
  expect(bodyText).toMatch(/aptok_v1_test_confirmation_token/i);
  expect(bodyText).toMatch(/Apply executor is still locked/i);
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|restore now|run backup|run restore|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("shows blocked operation previews as empty and audit-only", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.previewApplyPlanOperations).mockResolvedValueOnce(blockedOperationPreview);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Preview operation set/i }));

  expect(await screen.findByText(/No operation rows returned/i)).toBeInTheDocument();
  expect(screen.getByText(/review\/audit-only/i)).toBeInTheDocument();
  expect(screen.getByText(/This draft remains blocked or audit-only/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
});

it("shows operation-set loading and error states without changing files", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  let resolveOperationPreview: (preview: ApplyPlanOperationPreview) => void = () => {};
  vi.mocked(api.previewApplyPlanOperations).mockReturnValueOnce(
    new Promise((resolve) => {
      resolveOperationPreview = resolve;
    }),
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Preview operation set/i }));

  expect((await screen.findAllByText(/Checking operation preview/i)).length).toBeGreaterThan(0);

  await act(async () => {
    resolveOperationPreview(operationPreview);
  });

  expect((await screen.findAllByText(/OperationCandidate\.package/i)).length).toBeGreaterThan(0);

  vi.mocked(api.previewApplyPlanOperations).mockRejectedValueOnce(
    new Error("operation preview unavailable"),
  );
  fireEvent.click(screen.getByRole("button", { name: /Refresh operation preview/i }));

  expect(await screen.findByText(/Operation preview could not be loaded/i)).toBeInTheDocument();
  expect(screen.getByText(/operation preview unavailable/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(screen.queryAllByText(/OperationCandidate\.package/i)).toHaveLength(0);
});

it("clears operation-set preview results when switching saved plans", async () => {
  const secondPlanSummary: ApplyPlanListItem = {
    ...savedPlanSummary,
    id: 702,
    title: "Second saved operation draft",
    sourceStagingPlanId: "second-operation-plan-test",
  };
  const secondPlanDetails: PersistedApplyPlan = {
    ...savedPlanDetails,
    id: 702,
    title: "Second saved operation draft",
    sourceStagingPlanId: "second-operation-plan-test",
    items: [
      {
        ...savedPlanDetails.items[0],
        id: 9401,
        applyPlanId: 702,
        fileName: "SecondOperationPlan.package",
        sourceItemId: "second-operation-plan-item",
      },
    ],
  };

  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([
    savedPlanSummary,
    secondPlanSummary,
  ]);
  vi.mocked(api.getApplyPlan).mockImplementation(async (planId) =>
    planId === secondPlanSummary.id ? secondPlanDetails : savedPlanDetails,
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  const reviewButtons = await screen.findAllByRole("button", { name: /Review details/i });
  fireEvent.click(reviewButtons[0]);
  fireEvent.click(await screen.findByRole("button", { name: /Preview operation set/i }));

  expect((await screen.findAllByText(/OperationCandidate\.package/i)).length).toBeGreaterThan(0);
  fireEvent.click(await screen.findByRole("button", { name: /^Review details$/i }));

  expect(await screen.findByText(/Second saved operation draft/i)).toBeInTheDocument();
  expect(screen.queryAllByText(/OperationCandidate\.package/i)).toHaveLength(0);
  expect(screen.getByText(/Run an operation preview/i)).toBeInTheDocument();
});

it("shows read-only recovery history empty states for saved plans", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));

  expect((await screen.findAllByText(/Recovery history/i)).length).toBeGreaterThan(0);
  expect(screen.getByText(/Result log and restore map/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Apply is not ready yet/i).length).toBeGreaterThan(0);
  expect(screen.getAllByText(/Restore is not ready yet/i).length).toBeGreaterThan(0);
  expect(await screen.findByText(/No result logs yet/i)).toBeInTheDocument();
  expect(screen.getByText(/No Apply run has happened/i)).toBeInTheDocument();
  expect(screen.getByText(/No restore entries yet/i)).toBeInTheDocument();
  expect(screen.getByText(/Restore is not available/i)).toBeInTheDocument();

  await waitFor(() => {
    expect(api.listApplyPlanRunLogs).toHaveBeenCalledWith({
      applyPlanId: savedPlanSummary.id,
      limit: 20,
    });
  });
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
  expect(enabledButtonLabels()).not.toEqual(
    expect.arrayContaining([
      expect.stringMatching(
        /apply|restore now|run backup|run restore|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation/i,
      ),
    ]),
  );
});

it("shows read-only result logs and restore-map records with safe labels", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.listApplyPlanRunLogs).mockResolvedValue([recoveryRunLog]);
  vi.mocked(api.listApplyPlanResultLogs).mockResolvedValue([recoveryResultLog]);
  vi.mocked(api.listApplyPlanRestoreEntries).mockResolvedValue([
    recoveryRestoreEntry,
  ]);
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));

  expect((await screen.findAllByText(/Result log 3001/i)).length).toBeGreaterThan(0);
  expect(screen.getByText(/Read-only metadata/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Pending log/i).length).toBeGreaterThan(0);
  expect(screen.getByText(/Design-only/i)).toBeInTheDocument();
  expect(screen.getByText(/Restore map record/i)).toBeInTheDocument();
  expect(screen.getByText(/Fixture proof metadata only/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Technical details/i).length).toBeGreaterThan(0);
  expect(
    screen.getAllByTitle(recoveryResultLog.sourcePathAtExecution ?? "")[0],
  ).toHaveTextContent(/\.\.\.\\Mods\\Loose\\ResultHair\.package/i);
  expect(screen.queryByText(/Ready to apply/i)).not.toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Restore now/i })).not.toBeInTheDocument();
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
});

it("shows recovery history load errors safely", async () => {
  vi.mocked(api.listSavedApplyPlans).mockResolvedValue([savedPlanSummary]);
  vi.mocked(api.getApplyPlan).mockResolvedValue(savedPlanDetails);
  vi.mocked(api.listApplyPlanRunLogs).mockRejectedValue(
    new Error("history unavailable"),
  );
  renderOrganize();

  fireEvent.click(screen.getByRole("tab", { name: /Saved plans/i }));
  fireEvent.click(await screen.findByRole("button", { name: /Review details/i }));

  expect(await screen.findByText(/Could not load recovery history/i)).toBeInTheDocument();
  expect(screen.getByText(/history unavailable/i)).toBeInTheDocument();
  expect(screen.getAllByText(/No files changed/i).length).toBeGreaterThan(0);
  expect(api.createApplyPlanRunLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanResultLog).not.toHaveBeenCalled();
  expect(api.recordApplyPlanRestoreEntry).not.toHaveBeenCalled();
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
