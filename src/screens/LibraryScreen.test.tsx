import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { api } from "../lib/api";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { LibraryScreen } from "./LibraryScreen";
import type {
  FolderTreeMetadata,
  LibraryFacets,
  LibraryFileRow,
  LibraryListResponse,
  LibrarySettings,
  LibrarySummary,
} from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    getLibraryFacets: vi.fn(),
    getLibrarySettings: vi.fn(),
    getLibrarySummary: vi.fn(),
    listLibraryFiles: vi.fn(),
    listLibraryFilesForTree: vi.fn(),
    listLibraryFolderFiles: vi.fn(),
    getFolderTreeMetadata: vi.fn(),
    getFileDetail: vi.fn(),
    revealFileInFolder: vi.fn(),
    saveCreatorLearning: vi.fn(),
    saveCategoryOverride: vi.fn(),
  },
}));

const emptyFacets: LibraryFacets = {
  creators: [],
  kinds: [],
  subtypes: [],
  sources: [],
  taxonomyKinds: [],
};

const emptySummary: LibrarySummary = {
  total: 0,
  tracked: 0,
  notTracked: 0,
  hasUpdates: 0,
  needsReview: 0,
  duplicates: 0,
  disabled: 0,
};

const librarySettings: LibrarySettings = {
  modsPath: "C:/Mods",
  trayPath: "C:/Tray",
  downloadsPath: "C:/Downloads",
  downloadRejectFolder: null,
};

const gameplayFile: LibraryFileRow = {
  id: 7,
  filename: "folder-item.package",
  path: "C:/Mods/Gameplay/folder-item.package",
  extension: ".package",
  kind: "Gameplay",
  subtype: null,
  confidence: 0.91,
  sourceLocation: "mods",
  size: 1024,
  modifiedAt: null,
  creator: "TestCreator",
  bundleName: null,
  bundleType: null,
  groupedFileCount: null,
  relativeDepth: 1,
  safetyNotes: [],
  parserWarnings: [],
  insights: {
    format: null,
    resourceSummary: [],
    scriptNamespaces: [],
    embeddedNames: [],
    creatorHints: [],
    versionHints: [],
    versionSignals: [],
    familyHints: [],
  },
  watchStatus: "not_watched",
  hasDuplicate: false,
  sameFolderPeerCount: 0,
  samePackPeerCount: 0,
};

const emptyRows: LibraryListResponse = {
  total: 0,
  items: [],
};

const folderRows: LibraryListResponse = {
  total: 1,
  items: [gameplayFile],
};

const folderMetadata: FolderTreeMetadata = {
  total_folders: 2,
  roots: [
    {
      path: "Mods",
      name: "Mods",
      depth: 0,
      sourceLocation: "mods",
      directFileCount: 0,
      childFolderCount: 1,
      totalFileCount: 1,
      children: [
        {
          path: "Mods/Gameplay",
          name: "Gameplay",
          depth: 1,
          sourceLocation: "mods",
          directFileCount: 1,
          childFolderCount: 0,
          totalFileCount: 1,
          children: [],
        },
      ],
    },
  ],
};

const emptyFolderMetadata: FolderTreeMetadata = {
  total_folders: 2,
  roots: [
    {
      path: "Mods",
      name: "Mods",
      depth: 0,
      sourceLocation: "mods",
      directFileCount: 0,
      childFolderCount: 1,
      totalFileCount: 0,
      children: [
        {
          path: "Mods/Empty",
          name: "Empty",
          depth: 1,
          sourceLocation: "mods",
          directFileCount: 0,
          childFolderCount: 0,
          totalFileCount: 0,
          children: [],
        },
      ],
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("loads selected folder contents from the folder-file command without prefetching every library row", async () => {
  vi.mocked(api.getLibraryFacets).mockResolvedValue(emptyFacets);
  vi.mocked(api.getLibrarySettings).mockResolvedValue(librarySettings);
  vi.mocked(api.getLibrarySummary).mockResolvedValue(emptySummary);
  vi.mocked(api.listLibraryFiles).mockResolvedValue(emptyRows);
  vi.mocked(api.getFolderTreeMetadata).mockResolvedValue(folderMetadata);
  vi.mocked(api.listLibraryFilesForTree).mockResolvedValue(emptyRows);
  vi.mocked(api.listLibraryFolderFiles).mockImplementation(async (query) =>
    query.folderPath === "Mods/Gameplay" ? folderRows : emptyRows,
  );

  render(
    <UiPreferencesProvider mode="seasoned">
      <LibraryScreen refreshVersion={0} onNavigate={() => {}} userView="standard" />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(api.getFolderTreeMetadata).toHaveBeenCalled();
  });
  expect(api.listLibraryFilesForTree).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: /folders view/i }));
  fireEvent.click(await screen.findByRole("button", { name: /gameplay/i }));

  await waitFor(() => {
    expect(api.listLibraryFolderFiles).toHaveBeenCalledWith(
      expect.objectContaining({
        folderPath: "Mods/Gameplay",
        recursive: true,
        includePreviews: false,
      }),
    );
  });
  expect(await screen.findByTitle("folder-item.package")).toBeInTheDocument();
});

it("shows an empty folder summary and opens the configured folder path", async () => {
  vi.mocked(api.getLibraryFacets).mockResolvedValue(emptyFacets);
  vi.mocked(api.getLibrarySettings).mockResolvedValue(librarySettings);
  vi.mocked(api.getLibrarySummary).mockResolvedValue(emptySummary);
  vi.mocked(api.listLibraryFiles).mockResolvedValue(emptyRows);
  vi.mocked(api.getFolderTreeMetadata).mockResolvedValue(emptyFolderMetadata);
  vi.mocked(api.listLibraryFilesForTree).mockResolvedValue(emptyRows);
  vi.mocked(api.listLibraryFolderFiles).mockResolvedValue(emptyRows);
  vi.mocked(api.revealFileInFolder).mockResolvedValue();

  render(
    <UiPreferencesProvider mode="seasoned">
      <LibraryScreen refreshVersion={0} onNavigate={() => {}} userView="standard" />
    </UiPreferencesProvider>,
  );

  await waitFor(() => {
    expect(api.getFolderTreeMetadata).toHaveBeenCalled();
  });

  fireEvent.click(screen.getByRole("button", { name: /folders view/i }));
  fireEvent.click(await screen.findByRole("button", { name: /empty/i }));

  expect(await screen.findByText("0 files.")).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: /open folder/i }));

  await waitFor(() => {
    expect(api.revealFileInFolder).toHaveBeenCalledWith("C:\\Mods\\Empty");
  });
});
