import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { HomeScreen } from "./HomeScreen";
import type { DetectedLibraryPaths, HomeOverview, LibrarySettings, Screen } from "../lib/types";

vi.mock("../lib/api", () => ({
  api: {
    getHomeOverview: vi.fn(),
    detectDefaultLibraryPaths: vi.fn(),
    pickFolder: vi.fn(),
  },
}));

const overview: HomeOverview = {
  totalFiles: 42,
  modsCount: 30,
  trayCount: 4,
  downloadsCount: 3,
  scriptModsCount: 2,
  creatorCount: 8,
  bundlesCount: 5,
  duplicatesCount: 1,
  reviewCount: 2,
  unsafeCount: 1,
  exactUpdateItems: 1,
  possibleUpdateItems: 2,
  checkFailedWatchItems: 1,
  unknownWatchItems: 1,
  watchReviewItems: 1,
  watchSetupItems: 1,
  lastScanAt: "2026-05-28T10:00:00.000Z",
  scanNeedsRefresh: true,
  readOnlyMode: true,
};

const detectedPaths: DetectedLibraryPaths = {
  modsPath: null,
  trayPath: null,
  downloadsPath: null,
};

const settings: LibrarySettings = {
  modsPath: "C:/Sims/Mods",
  trayPath: "C:/Sims/Tray",
  downloadsPath: "C:/Sims/Downloads",
  downloadRejectFolder: null,
};

function renderHome({ userView = "standard" as const, homeSettings = settings } = {}) {
  const onNavigate = vi.fn<(screen: Screen) => void>();
  const onSettingsChange = vi.fn<(next: LibrarySettings) => Promise<void>>().mockResolvedValue(undefined);
  const onScan = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);

  vi.mocked(api.getHomeOverview).mockResolvedValue(overview);
  vi.mocked(api.detectDefaultLibraryPaths).mockResolvedValue(detectedPaths);

  render(
    <UiPreferencesProvider mode="seasoned">
      <HomeScreen
        refreshVersion={0}
        settings={homeSettings}
        onSettingsChange={onSettingsChange}
        onNavigate={onNavigate}
        onNavigateWithParams={() => {}}
        onScan={onScan}
        isScanning={false}
        userView={userView}
      />
    </UiPreferencesProvider>,
  );

  return { onNavigate, onScan, onSettingsChange };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
});

it("shows a first-run journey from setup through organize preview without implying file changes", async () => {
  renderHome();

  expect(await screen.findByText("Start here")).toBeInTheDocument();
  expect(screen.getAllByText("Scan Library").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Review Inbox").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Check Duplicates").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Review Updates").length).toBeGreaterThan(0);
  expect(screen.getAllByText("Create Organization Preview").length).toBeGreaterThan(0);

  const copy = document.body.textContent ?? "";
  expect(copy).toMatch(/No files changed/i);
  expect(copy).toMatch(/Preview only/i);
  expect(copy).toMatch(/Apply not ready yet/i);
  expect(copy).toMatch(/Restore not ready yet/i);
  expect(copy).not.toMatch(/ready to apply|safe to delete|cleaned up|automatically fixed/i);
});

it("summarizes what SimSuite knows, suspects, and blocks on Home", async () => {
  renderHome();

  expect(await screen.findByText("What SimSuite knows")).toBeInTheDocument();
  expect(screen.getByText("Exact match")).toBeInTheDocument();
  expect(screen.getByText("Evidence-backed cue")).toBeInTheDocument();
  expect(screen.getByText("Heuristic hint")).toBeInTheDocument();
  expect(screen.getByText("Manual review needed")).toBeInTheDocument();
  expect(screen.getByText("Not supported yet")).toBeInTheDocument();
  expect(screen.getByText(/Library configured/i)).toBeInTheDocument();
  expect(screen.getByText(/Last scan/i)).toBeInTheDocument();
  expect(screen.getAllByText(/Review queues/i).length).toBeGreaterThan(0);
});

it("journey cards route users to the matching review and preview screens", async () => {
  const { onNavigate } = renderHome();
  await screen.findByText("Start here");

  fireEvent.click(screen.getByRole("button", { name: /Review Inbox/i }));
  expect(onNavigate).toHaveBeenCalledWith("downloads");

  fireEvent.click(screen.getByRole("button", { name: /Check Duplicates/i }));
  expect(onNavigate).toHaveBeenCalledWith("duplicates");

  fireEvent.click(screen.getByRole("button", { name: /Create Organization Preview/i }));
  expect(onNavigate).toHaveBeenCalledWith("organize");
});

it("keeps setup as the first action when required folders are missing", async () => {
  renderHome({
    homeSettings: {
      modsPath: null,
      trayPath: null,
      downloadsPath: null,
      downloadRejectFolder: null,
    },
  });

  expect(await screen.findByText("Start here")).toBeInTheDocument();
  await waitFor(() => {
    expect(screen.getByText(/Library not configured/i)).toBeInTheDocument();
  });
  expect(screen.getByRole("button", { name: /Choose folders/i })).toBeInTheDocument();
});
