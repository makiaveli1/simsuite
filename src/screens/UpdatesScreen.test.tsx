import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import type {
  LibraryFileRow,
  LibraryListResponse,
  LibraryWatchListResponse,
  LibraryWatchReviewResponse,
  LibraryWatchSetupResponse,
} from "../lib/types";
import { UpdatesScreen } from "./UpdatesScreen";

vi.mock("../lib/api", () => ({
  api: {
    listLibraryWatchItems: vi.fn(),
    listLibraryWatchSetupItems: vi.fn(),
    listLibraryFiles: vi.fn(),
    listLibraryWatchReviewItems: vi.fn(),
    getFileDetail: vi.fn(),
    saveWatchSourceForFile: vi.fn(),
    clearWatchSourceForFile: vi.fn(),
    refreshWatchSourceForFile: vi.fn(),
    refreshWatchedSources: vi.fn(),
  },
}));

const emptyInsights = {
  format: null,
  resourceSummary: [],
  scriptNamespaces: [],
  embeddedNames: [],
  creatorHints: [],
  versionHints: [],
  versionSignals: [],
  familyHints: [],
};

function makeRow(id: number, filename: string, creator: string | null): LibraryFileRow {
  return {
    id,
    filename,
    path: `C:/Mods/${filename}`,
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.9,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator,
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    insights: emptyInsights,
    watchStatus: "not_watched",
    hasDuplicate: false,
    sameFolderPeerCount: 0,
    samePackPeerCount: 0,
  };
}

const emptyTracked: LibraryWatchListResponse = {
  filter: "attention",
  total: 0,
  items: [],
};

const emptyReview: LibraryWatchReviewResponse = {
  total: 0,
  providerNeededCount: 0,
  referenceOnlyCount: 0,
  checkFailedCount: 0,
  unknownResultCount: 0,
  items: [],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("lets the source queue be filtered between possible matches and no-match items", async () => {
  const libraryRows: LibraryListResponse = {
    total: 2,
    items: [
      makeRow(1, "alpha.package", "Creator One"),
      makeRow(2, "beta.package", null),
    ],
  };

  const setupRows: LibraryWatchSetupResponse = {
    total: 1,
    truncated: false,
    exactPageTotal: 1,
    exactPageTruncated: false,
    exactPageItems: [
      {
        fileId: 1,
        filename: "alpha.package",
        creator: "Creator One",
        subjectLabel: "Alpha Mod",
        installedVersion: "1.0.0",
        suggestedSourceKind: "exact_page",
        setupHint: "Looks like a likely exact page.",
      },
    ],
    items: [
      {
        fileId: 1,
        filename: "alpha.package",
        creator: "Creator One",
        subjectLabel: "Alpha Mod",
        installedVersion: "1.0.0",
        suggestedSourceKind: "exact_page",
        setupHint: "Looks like a likely exact page.",
      },
    ],
  };

  vi.mocked(api.listLibraryWatchItems).mockResolvedValue(emptyTracked);
  vi.mocked(api.listLibraryWatchSetupItems).mockResolvedValue(setupRows);
  vi.mocked(api.listLibraryFiles).mockResolvedValue(libraryRows);
  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue(emptyReview);
  vi.mocked(api.getFileDetail).mockImplementation(async (fileId: number) => ({
    id: fileId,
    filename: fileId === 1 ? "alpha.package" : "beta.package",
    path: `C:/Mods/${fileId === 1 ? "alpha.package" : "beta.package"}`,
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.91,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: fileId === 1 ? "Creator One" : null,
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    installedVersionSummary: { version: fileId === 1 ? "1.0.0" : null },
    watchResult: {
      status: "not_watched",
      sourceKind: null,
      sourceOrigin: "none",
      sourceLabel: null,
      sourceUrl: null,
      source: null,
      patreonUrl: null,
      capability: "saved_reference_only",
      canRefreshNow: false,
      providerName: null,
      latestVersion: null,
      checkedAt: null,
      confidence: "unknown",
      note: null,
      evidence: [],
    },
    insights: fileId === 1
      ? { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"], versionHints: ["1.0.0"] }
      : emptyInsights,
  }) as never);

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen refreshVersion={0} onNavigate={() => {}} userView="standard" initialMode="setup" />
    </UiPreferencesProvider>,
  );

  expect(await screen.findByText("alpha.package")).toBeInTheDocument();
  expect(screen.getByText("beta.package")).toBeInTheDocument();

  fireEvent.click(screen.getAllByRole("button", { name: /^possible source$/i })[0]);

  await waitFor(() => {
    expect(screen.getAllByText("alpha.package").length).toBeGreaterThan(0);
    expect(screen.queryAllByText("beta.package")).toHaveLength(0);
  });

  fireEvent.click(screen.getAllByRole("button", { name: /^no update source$/i })[0]);

  await waitFor(() => {
    expect(screen.queryAllByText("alpha.package")).toHaveLength(0);
    expect(screen.getAllByText("beta.package").length).toBeGreaterThan(0);
  });
});

it("moves a saved setup item into the watching lane when the saved page supports explicit checks", async () => {
  let saved = false;

  const listLibraryFilesMock = vi.mocked(api.listLibraryFiles);
  const listLibraryWatchItemsMock = vi.mocked(api.listLibraryWatchItems);
  const listLibraryWatchSetupItemsMock = vi.mocked(api.listLibraryWatchSetupItems);

  listLibraryWatchItemsMock.mockImplementation(async () => ({
    filter: "all",
    total: saved ? 1 : 0,
    items: saved
      ? [
          {
            fileId: 1,
            filename: "alpha.package",
            creator: "Creator One",
            subjectLabel: "Alpha Mod",
            installedVersion: "1.0.0",
            watchResult: {
              status: "not_watched",
              sourceKind: "exact_page",
              sourceOrigin: "saved_by_user",
              sourceLabel: "GitHub Release",
              sourceUrl: "https://github.com/example/mod/releases",
              source: "website",
              patreonUrl: null,
              capability: "can_refresh_now",
              canRefreshNow: true,
              providerName: null,
              latestVersion: null,
              checkedAt: null,
              confidence: "unknown",
              note: "SimSuite can check this GitHub releases page now when you press Check now.",
              evidence: [],
            },
          },
        ]
      : [],
  }));
  listLibraryWatchSetupItemsMock.mockImplementation(async () => ({
    total: saved ? 0 : 1,
    truncated: false,
    exactPageTotal: saved ? 0 : 1,
    exactPageTruncated: false,
    exactPageItems: saved
      ? []
      : [
          {
            fileId: 1,
            filename: "alpha.package",
            creator: "Creator One",
            subjectLabel: "Alpha Mod",
            installedVersion: "1.0.0",
            suggestedSourceKind: "exact_page",
            setupHint: "Looks like a likely exact page.",
          },
        ],
    items: saved
      ? []
      : [
          {
            fileId: 1,
            filename: "alpha.package",
            creator: "Creator One",
            subjectLabel: "Alpha Mod",
            installedVersion: "1.0.0",
            suggestedSourceKind: "exact_page",
            setupHint: "Looks like a likely exact page.",
          },
        ],
  }));
  listLibraryFilesMock.mockImplementation(async () => ({
    total: saved ? 0 : 1,
    items: saved ? [] : [makeRow(1, "alpha.package", "Creator One")],
  }));

  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue(emptyReview);
  vi.mocked(api.saveWatchSourceForFile).mockImplementation(async () => {
    saved = true;
    return {} as never;
  });
  vi.mocked(api.getFileDetail).mockImplementation(async () => {
    if (!saved) {
      return {
        id: 1,
        filename: "alpha.package",
        path: "C:/Mods/alpha.package",
        extension: ".package",
        kind: "Gameplay",
        subtype: null,
        confidence: 0.91,
        sourceLocation: "mods",
        size: 1024,
        modifiedAt: null,
        creator: "Creator One",
        bundleName: null,
        bundleType: null,
        groupedFileCount: null,
        relativeDepth: 1,
        safetyNotes: [],
        parserWarnings: [],
        installedVersionSummary: { version: "1.0.0" },
        watchResult: {
          status: "not_watched",
          sourceKind: null,
          sourceOrigin: "none",
          sourceLabel: null,
          sourceUrl: null,
          source: null,
          patreonUrl: null,
          capability: "saved_reference_only",
          canRefreshNow: false,
          providerName: null,
          latestVersion: null,
          checkedAt: null,
          confidence: "unknown",
          note: null,
          evidence: [],
        },
        insights: { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"] },
      } as never;
    }

    return {
      id: 1,
      filename: "alpha.package",
      path: "C:/Mods/alpha.package",
      extension: ".package",
      kind: "Gameplay",
      subtype: null,
      confidence: 0.91,
      sourceLocation: "mods",
      size: 1024,
      modifiedAt: null,
      creator: "Creator One",
      bundleName: null,
      bundleType: null,
      groupedFileCount: null,
      relativeDepth: 1,
      safetyNotes: [],
      parserWarnings: [],
      installedVersionSummary: { version: "1.0.0" },
      watchResult: {
        status: "not_watched",
        sourceKind: "exact_page",
        sourceOrigin: "saved_by_user",
        sourceLabel: "GitHub Release",
        sourceUrl: "https://github.com/example/mod/releases",
        source: "website",
        patreonUrl: null,
        capability: "can_refresh_now",
        canRefreshNow: true,
        providerName: null,
        latestVersion: null,
        checkedAt: null,
        confidence: "unknown",
        note: "SimSuite can check this GitHub releases page now when you press Check now.",
        evidence: [],
      },
      insights: { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"] },
    } as never;
  });

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        onDataChanged={() => {}}
        userView="standard"
        initialMode="setup"
        initialFileId={1}
      />
    </UiPreferencesProvider>,
  );

  const urlInput = await screen.findByPlaceholderText("https://...");
  fireEvent.change(urlInput, { target: { value: "https://github.com/example/mod/releases" } });
  fireEvent.click(screen.getByRole("button", { name: /save source/i }));

  await waitFor(() => {
    expect(screen.getByRole("heading", { name: "Watched" })).toBeInTheDocument();
  });
  expect(screen.getByText("Source saved.")).toBeInTheDocument();
  expect(listLibraryWatchItemsMock).toHaveBeenCalledWith("all", 48);
});

it("uses possible-update wording for update leads without automatic-check claims", async () => {
  const trackedRows: LibraryWatchListResponse = {
    filter: "all",
    total: 1,
    items: [
      {
        fileId: 1,
        filename: "alpha.package",
        creator: "Creator One",
        subjectLabel: "Alpha Mod",
        installedVersion: "1.0.0",
        watchResult: {
          status: "exact_update_available",
          sourceKind: "exact_page",
          sourceOrigin: "saved_by_user",
          sourceLabel: "GitHub Release",
          sourceUrl: "https://github.com/example/mod/releases",
          source: "website",
          patreonUrl: null,
          capability: "can_refresh_now",
          canRefreshNow: true,
          providerName: null,
          latestVersion: "1.1.0",
          checkedAt: "2026-05-08T09:00:00Z",
          confidence: "strong",
          note: "A newer version may be available from the saved page.",
          evidence: ["Saved watch page last saw version 1.1.0."],
        },
      },
    ],
  };

  vi.mocked(api.listLibraryWatchItems).mockResolvedValue(trackedRows);
  vi.mocked(api.listLibraryWatchSetupItems).mockResolvedValue({
    total: 0,
    truncated: false,
    exactPageTotal: 0,
    exactPageTruncated: false,
    exactPageItems: [],
    items: [],
  });
  vi.mocked(api.listLibraryFiles).mockResolvedValue({ total: 0, items: [] });
  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue(emptyReview);
  vi.mocked(api.getFileDetail).mockResolvedValue({
    id: 1,
    filename: "alpha.package",
    path: "C:/Mods/alpha.package",
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.91,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: "Creator One",
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    installedVersionSummary: { version: "1.0.0" },
    watchResult: trackedRows.items[0].watchResult,
    insights: { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"] },
  } as never);

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        userView="standard"
        initialMode="attention"
        initialFileId={1}
      />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(screen.getAllByText("Update may be available").length).toBeGreaterThan(0);
  });
  expect(screen.getAllByText("Update checks enabled").length).toBeGreaterThan(0);
  expect(screen.queryByText("Update found")).toBeNull();
  expect(screen.queryByText(/automatic checks/i)).toBeNull();
  expect(screen.queryByText(/checks for updates automatically/i)).toBeNull();
});

it("renders persisted check-failed state as could-not-check without broken or up-to-date claims", async () => {
  const watchResult = {
    status: "check_failed" as const,
    sourceKind: "exact_page" as const,
    sourceOrigin: "saved_by_user" as const,
    sourceLabel: "GitHub Release",
    sourceUrl: "https://github.com/example/mod/releases",
    source: "website" as const,
    patreonUrl: null,
    capability: "can_refresh_now" as const,
    canRefreshNow: true,
    providerName: null,
    latestVersion: null,
    checkedAt: "2026-05-08T09:00:00Z",
    confidence: "unknown" as const,
    note: "SimSuite tried to check this source but could not finish. Try again later.",
    evidence: ["The saved source check did not finish."],
  };

  const trackedRows: LibraryWatchListResponse = {
    filter: "all",
    total: 1,
    items: [
      {
        fileId: 1,
        filename: "alpha.package",
        creator: "Creator One",
        subjectLabel: "Alpha Mod",
        installedVersion: "1.0.0",
        watchResult,
      },
    ],
  };

  vi.mocked(api.listLibraryWatchItems).mockResolvedValue(trackedRows);
  vi.mocked(api.listLibraryWatchSetupItems).mockResolvedValue({
    total: 0,
    truncated: false,
    exactPageTotal: 0,
    exactPageTruncated: false,
    exactPageItems: [],
    items: [],
  });
  vi.mocked(api.listLibraryFiles).mockResolvedValue({ total: 0, items: [] });
  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue({
    total: 1,
    providerNeededCount: 0,
    referenceOnlyCount: 0,
    checkFailedCount: 1,
    unknownResultCount: 0,
    items: [],
  });
  vi.mocked(api.getFileDetail).mockResolvedValue({
    id: 1,
    filename: "alpha.package",
    path: "C:/Mods/alpha.package",
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.91,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: "Creator One",
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    installedVersionSummary: { version: "1.0.0" },
    watchResult,
    insights: { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"] },
  } as never);

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        userView="standard"
        initialMode="attention"
        initialFileId={1}
      />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(screen.getAllByText("Could not check").length).toBeGreaterThan(0);
  });
  expect(screen.getByText(/Last check failed/i)).toBeInTheDocument();
  expect(screen.queryByText(/broken/i)).toBeNull();
  expect(screen.queryByText(/up to date/i)).toBeNull();
  expect(screen.queryByText(/definitely outdated/i)).toBeNull();
});

it("renders persisted reminder-only state as manual follow-up, not a checked watch result", async () => {
  const watchResult = {
    status: "reminder_only" as const,
    sourceKind: "creator_page" as const,
    sourceOrigin: "saved_by_user" as const,
    sourceLabel: "Creator page",
    sourceUrl: "https://example.com/creator",
    source: "website" as const,
    patreonUrl: null,
    capability: "saved_reference_only" as const,
    canRefreshNow: false,
    providerName: null,
    latestVersion: null,
    checkedAt: null,
    confidence: "unknown" as const,
    note: "Creator pages are saved as reminders for now. SimSuite cannot check creator pages automatically yet.",
    evidence: [
      "Source saved for reference only. SimSuite cannot check this source automatically yet.",
    ],
  };

  const reviewRows: LibraryWatchReviewResponse = {
    total: 1,
    providerNeededCount: 0,
    referenceOnlyCount: 1,
    checkFailedCount: 0,
    unknownResultCount: 0,
    items: [
      {
        fileId: 1,
        filename: "alpha.package",
        creator: "Creator One",
        subjectLabel: "Alpha Mod",
        installedVersion: "1.0.0",
        watchResult,
        reviewReason: "reference_only",
        reviewHint:
          "This creator page is saved as a reminder only. Keep it if it helps, or replace it with an exact mod page.",
      },
    ],
  };

  vi.mocked(api.listLibraryWatchItems).mockResolvedValue(emptyTracked);
  vi.mocked(api.listLibraryWatchSetupItems).mockResolvedValue({
    total: 0,
    truncated: false,
    exactPageTotal: 0,
    exactPageTruncated: false,
    exactPageItems: [],
    items: [],
  });
  vi.mocked(api.listLibraryFiles).mockResolvedValue({ total: 0, items: [] });
  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue(reviewRows);
  vi.mocked(api.getFileDetail).mockResolvedValue({
    id: 1,
    filename: "alpha.package",
    path: "C:/Mods/alpha.package",
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.91,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: "Creator One",
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    installedVersionSummary: { version: "1.0.0" },
    watchResult,
    insights: { ...emptyInsights, embeddedNames: ["Alpha"], familyHints: ["alpha"] },
  } as never);

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        userView="standard"
        initialMode="reminders"
        initialFileId={1}
      />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(screen.getAllByText("Reminder only").length).toBeGreaterThan(0);
  });
  expect(screen.getAllByText(/manual follow-up/i).length).toBeGreaterThan(0);
  expect(screen.queryByText(/Checked recently/i)).toBeNull();
  expect(screen.queryByText(/No update found/i)).toBeNull();
  expect(screen.queryByText(/automatic checks/i)).toBeNull();
});

it("keeps focused no-source files useful even when they are not in the current setup page", async () => {
  vi.mocked(api.listLibraryWatchItems).mockResolvedValue(emptyTracked);
  vi.mocked(api.listLibraryWatchSetupItems).mockResolvedValue({
    total: 0,
    truncated: false,
    exactPageTotal: 0,
    exactPageTruncated: false,
    exactPageItems: [],
    items: [],
  });
  vi.mocked(api.listLibraryFiles).mockResolvedValue({ total: 0, items: [] });
  vi.mocked(api.listLibraryWatchReviewItems).mockResolvedValue(emptyReview);
  vi.mocked(api.getFileDetail).mockResolvedValue({
    id: 42,
    filename: "lonely.package",
    path: "C:/Mods/lonely.package",
    extension: ".package",
    kind: "Gameplay",
    subtype: null,
    confidence: 0.74,
    sourceLocation: "mods",
    size: 1024,
    modifiedAt: null,
    creator: "Creator Two",
    bundleName: null,
    bundleType: null,
    groupedFileCount: null,
    relativeDepth: 1,
    safetyNotes: [],
    parserWarnings: [],
    installedVersionSummary: null,
    watchResult: {
      status: "not_watched",
      sourceKind: null,
      sourceOrigin: "none",
      sourceLabel: null,
      sourceUrl: null,
      source: null,
      patreonUrl: null,
      capability: "saved_reference_only",
      canRefreshNow: false,
      providerName: null,
      latestVersion: null,
      checkedAt: null,
      confidence: "unknown",
      note: "No approved watch source is saved for this installed content yet.",
      evidence: [],
    },
    insights: { ...emptyInsights, creatorHints: ["Creator Two"], familyHints: ["lonely"] },
  } as never);

  render(
    <UiPreferencesProvider mode="seasoned">
      <UpdatesScreen
        refreshVersion={0}
        onNavigate={() => {}}
        userView="standard"
        initialMode="setup"
        initialFileId={42}
      />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(screen.getAllByText("lonely.package").length).toBeGreaterThan(0);
  });
  expect(screen.getByText(/Focused from Library or Preflight/i)).toBeInTheDocument();
  expect(screen.getAllByText("No update source").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Add or confirm a source").length).toBeGreaterThan(0);
  expect(screen.getByText("What SimSuite can check")).toBeInTheDocument();
  expect(screen.getByText("What SimSuite cannot check")).toBeInTheDocument();
});
