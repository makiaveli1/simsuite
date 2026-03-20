import type { ComponentProps } from "react";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { UpdatesScreen } from "./UpdatesScreen";
import type {
  FileDetail,
  LibraryWatchListResponse,
  LibraryWatchReviewResponse,
  LibraryWatchSetupResponse,
  WatchResult,
} from "../lib/types";

const apiMocks = vi.hoisted(() => ({
  listLibraryWatchItems: vi.fn(),
  listLibraryWatchSetupItems: vi.fn(),
  listLibraryWatchReviewItems: vi.fn(),
  getFileDetail: vi.fn(),
  saveWatchSourceForFile: vi.fn(),
  clearWatchSourceForFile: vi.fn(),
  refreshWatchSourceForFile: vi.fn(),
  refreshWatchedSources: vi.fn(),
}));

vi.mock("../lib/api", () => ({
  api: apiMocks,
}));

afterEach(() => {
  cleanup();
});

beforeEach(() => {
  vi.clearAllMocks();

  apiMocks.listLibraryWatchItems.mockImplementation(async () => TRACKED_RESPONSE);
  apiMocks.listLibraryWatchSetupItems.mockImplementation(async () => SETUP_RESPONSE);
  apiMocks.listLibraryWatchReviewItems.mockImplementation(async () => REVIEW_RESPONSE);
  apiMocks.getFileDetail.mockImplementation(async (fileId: number) => {
    const detail = FILE_DETAILS[fileId];
    if (!detail) {
      throw new Error(`Missing mock detail for ${fileId}`);
    }

    return detail;
  });
});

it("shows setup and review totals without making the user open those modes first", async () => {
  renderUpdatesScreen();

  await screen.findByText(/mccc_mccommandcenter\.ts4script/i);

  expect(
    await screen.findByRole("tab", { name: /tracked \(1\)/i }),
  ).toBeVisible();
  expect(
    await screen.findByRole("tab", { name: /need source \(2\)/i }),
  ).toBeVisible();
  expect(
    await screen.findByRole("tab", { name: /needs review \(1\)/i }),
  ).toBeVisible();
});

it("keeps setup rows in the queue until the user opens the source editor", async () => {
  renderUpdatesScreen({
    initialMode: "setup",
  });

  await screen.findByText(/miiko_eyebrows\.package/i);

  await waitFor(() => {
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

it("requests a fuller setup backlog instead of the tiny default slice", async () => {
  renderUpdatesScreen({
    initialMode: "setup",
  });

  await screen.findByText(/miiko_eyebrows\.package/i);

  await waitFor(() => {
    expect(apiMocks.listLibraryWatchSetupItems).toHaveBeenCalledWith(200);
  });
});

it("keeps the no-source sidebar focused on the suggested source and next step", async () => {
  renderUpdatesScreen({
    initialMode: "setup",
  });

  await screen.findByText(/miiko_eyebrows\.package/i);

  const inspector = screen.getByLabelText(/update details/i);

  expect(await within(inspector).findByText(/suggested source/i)).toBeVisible();
  expect(within(inspector).getByText(/set source/i)).toBeVisible();
  expect(within(inspector).queryByText(/check selected/i)).not.toBeInTheDocument();
  expect(within(inspector).queryByText(/latest helper version/i)).not.toBeInTheDocument();
});

it("keeps the setup queue compact by moving long hints out of the main list", async () => {
  renderUpdatesScreen({
    initialMode: "setup",
  });

  const listRegion = await screen.findByRole("region", {
    name: /update source setup list/i,
  });

  expect(within(listRegion).queryByText(/has creator and version clues/i)).not.toBeInTheDocument();
  expect(within(listRegion).queryByRole("columnheader", { name: /hint/i })).not.toBeInTheDocument();
});

it("opens source editing inside the inspector instead of a separate dialog", async () => {
  renderUpdatesScreen({
    initialMode: "setup",
  });

  await screen.findByText(/miiko_eyebrows\.package/i);

  fireEvent.click(await screen.findByRole("button", { name: /set source/i }));

  const inspector = screen.getByLabelText(/update details/i);

  expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  expect(within(inspector).getByLabelText(/source type/i)).toBeVisible();
  expect(within(inspector).getByLabelText(/^label$/i)).toBeVisible();
  expect(within(inspector).getByLabelText(/^url$/i)).toBeVisible();
  expect(within(inspector).getByRole("button", { name: /save source/i })).toBeVisible();
});

function renderUpdatesScreen(
  overrides: Partial<ComponentProps<typeof UpdatesScreen>> = {},
) {
  return render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={1}
        onNavigate={() => {}}
        onDataChanged={() => {}}
        userView="standard"
        {...overrides}
      />
    </UiPreferencesProvider>,
  );
}

const TRACKED_WATCH_RESULT: WatchResult = {
  status: "exact_update_available",
  sourceKind: "exact_page",
  sourceOrigin: "built_in_special",
  sourceLabel: "MC Command Center",
  sourceUrl: "https://deaderpool-mccc.com/downloads.html",
  capability: "can_refresh_now",
  canRefreshNow: true,
  providerName: null,
  latestVersion: "2026.4.0",
  checkedAt: "2026-03-11T10:00:00.000Z",
  confidence: "strong",
  note: "Official latest is a helper check and does not replace local install truth.",
  evidence: ["The official MCCC page shows a newer release than the installed copy."],
};

const REVIEW_WATCH_RESULT: WatchResult = {
  status: "unknown",
  sourceKind: "creator_page",
  sourceOrigin: "saved_by_user",
  sourceLabel: "AHarris00",
  sourceUrl: "https://www.patreon.com/aharris00britney",
  capability: "saved_reference_only",
  canRefreshNow: false,
  providerName: null,
  latestVersion: null,
  checkedAt: "2026-03-10T09:15:00.000Z",
  confidence: "weak",
  note: "This creator page is saved as a reminder only.",
  evidence: ["Creator pages are saved as reminders for now."],
};

const UNTRACKED_WATCH_RESULT: WatchResult = {
  status: "not_watched",
  sourceKind: null,
  sourceOrigin: "none",
  sourceLabel: null,
  sourceUrl: null,
  capability: "saved_reference_only",
  canRefreshNow: false,
  providerName: null,
  latestVersion: null,
  checkedAt: null,
  confidence: "weak",
  note: null,
  evidence: [],
};

const TRACKED_RESPONSE: LibraryWatchListResponse = {
  filter: "attention",
  total: 1,
  items: [
    {
      fileId: 4,
      filename: "MCCC_MCCommandCenter.ts4script",
      creator: "Deaderpool",
      subjectLabel: "MC Command Center",
      installedVersion: "2026.3.0",
      watchResult: TRACKED_WATCH_RESULT,
    },
  ],
};

const SETUP_RESPONSE: LibraryWatchSetupResponse = {
  total: 2,
  truncated: false,
  exactPageTotal: 1,
  exactPageTruncated: false,
  exactPageItems: [
    {
      fileId: 10,
      filename: "Miiko_Eyebrows.package",
      creator: "Miiko",
      subjectLabel: "Miiko",
      installedVersion: "2.4",
      suggestedSourceKind: "exact_page",
      setupHint: "Has creator and version clues, so an exact mod page should work well here.",
    },
  ],
  items: [
    {
      fileId: 10,
      filename: "Miiko_Eyebrows.package",
      creator: "Miiko",
      subjectLabel: "Miiko",
      installedVersion: "2.4",
      suggestedSourceKind: "exact_page",
      setupHint: "Has creator and version clues, so an exact mod page should work well here.",
    },
    {
      fileId: 11,
      filename: "AHarris00_CozyKitchen.package",
      creator: "AHarris00",
      subjectLabel: "AHarris00",
      installedVersion: null,
      suggestedSourceKind: "creator_page",
      setupHint:
        "Has strong creator clues, so a creator page is a reasonable reminder if no exact page is handy.",
    },
  ],
};

const REVIEW_RESPONSE: LibraryWatchReviewResponse = {
  total: 1,
  providerNeededCount: 0,
  referenceOnlyCount: 1,
  unknownResultCount: 0,
  items: [
    {
      fileId: 12,
      filename: "AHarris00_CozyStairs.package",
      creator: "AHarris00",
      subjectLabel: "AHarris00",
      installedVersion: null,
      watchResult: REVIEW_WATCH_RESULT,
      reviewReason: "reference_only",
      reviewHint:
        "This creator page is saved as a reminder only. Keep it if it helps, or replace it with an exact mod page.",
    },
  ],
};

const FILE_DETAILS: Record<number, FileDetail> = {
  4: makeFileDetail({
    id: 4,
    filename: "MCCC_MCCommandCenter.ts4script",
    extension: ".ts4script",
    kind: "Script Mods",
    subtype: "Core",
    creator: "Deaderpool",
    watchResult: TRACKED_WATCH_RESULT,
    installedVersionSummary: {
      subjectLabel: "MC Command Center",
      subjectKey: "mccc",
      version: "2026.3.0",
      signature: "sig-1",
      confidence: "strong",
      evidence: ["Installed copy found."],
    },
  }),
  10: makeFileDetail({
    id: 10,
    filename: "Miiko_Eyebrows.package",
    kind: "CAS",
    creator: "Miiko",
    watchResult: UNTRACKED_WATCH_RESULT,
    installedVersionSummary: {
      subjectLabel: "Miiko",
      subjectKey: "miiko-eyebrows",
      version: "2.4",
      signature: null,
      confidence: "medium",
      evidence: ["Version clue found in filename."],
    },
  }),
  11: makeFileDetail({
    id: 11,
    filename: "AHarris00_CozyKitchen.package",
    kind: "Build/Buy",
    creator: "AHarris00",
    watchResult: UNTRACKED_WATCH_RESULT,
    installedVersionSummary: null,
  }),
  12: makeFileDetail({
    id: 12,
    filename: "AHarris00_CozyStairs.package",
    kind: "Build/Buy",
    creator: "AHarris00",
    watchResult: REVIEW_WATCH_RESULT,
    installedVersionSummary: null,
  }),
};

function makeFileDetail(overrides: Partial<FileDetail>): FileDetail {
  const id = overrides.id ?? 1;
  const filename = overrides.filename ?? `MockFile${id}.package`;

  return {
    id,
    filename,
    path: overrides.path ?? `Mods\\Mock\\${filename}`,
    extension: overrides.extension ?? ".package",
    kind: overrides.kind ?? "Gameplay",
    subtype: overrides.subtype ?? null,
    confidence: overrides.confidence ?? 0.82,
    sourceLocation: overrides.sourceLocation ?? "mods",
    size: overrides.size ?? 2048,
    modifiedAt: overrides.modifiedAt ?? "2026-03-20T10:00:00.000Z",
    creator: overrides.creator ?? null,
    bundleName: overrides.bundleName ?? null,
    bundleType: overrides.bundleType ?? null,
    relativeDepth: overrides.relativeDepth ?? 1,
    safetyNotes: overrides.safetyNotes ?? [],
    hash: overrides.hash ?? "mock-hash",
    createdAt: overrides.createdAt ?? "2026-03-19T10:00:00.000Z",
    parserWarnings: overrides.parserWarnings ?? [],
    insights: overrides.insights ?? {
      format: "package",
      resourceSummary: [],
      scriptNamespaces: [],
      embeddedNames: [],
      creatorHints: [],
      versionHints: [],
      versionSignals: [],
      familyHints: [],
    },
    installedVersionSummary: overrides.installedVersionSummary ?? null,
    watchResult: overrides.watchResult ?? null,
    creatorLearning: overrides.creatorLearning ?? {
      lockedByUser: false,
      preferredPath: null,
      learnedAliases: [],
    },
    categoryOverride: overrides.categoryOverride ?? {
      savedByUser: false,
      kind: null,
      subtype: null,
    },
  };
}
