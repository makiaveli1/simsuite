import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import type {
  AppBehaviorSettings,
  GameInstallationProfile,
  GameInstallationProfileValidationReport,
  LibrarySettings,
} from "../lib/types";
import { SettingsScreen } from "./SettingsScreen";

vi.mock("../lib/api", () => ({
  api: {
    getAppBehaviorSettings: vi.fn(),
    getLibrarySettings: vi.fn(),
    getActiveGameInstallationProfile: vi.fn(),
    validateGameInstallationProfile: vi.fn(),
  },
}));

const appBehavior: AppBehaviorSettings = {
  keepRunningInBackground: false,
  automaticWatchChecks: false,
  watchCheckIntervalHours: 12,
  lastWatchCheckAt: null,
  lastWatchCheckError: null,
  downloadIgnorePatterns: [],
  silentSpecialModUpdates: null,
};

const librarySettings: LibrarySettings = {
  modsPath: "/Users/player/Documents/Electronic Arts/The Sims 4/Mods",
  trayPath: "/Users/player/Documents/Electronic Arts/The Sims 4/Tray",
  downloadsPath: "/Users/player/Downloads",
  downloadRejectFolder: null,
};

const profile: GameInstallationProfile = {
  profileId: "sims4-main",
  profileName: "Main Sims 4 setup",
  gameId: "sims4",
  operatingEnvironment: "native_macos",
  status: "needs_review",
  detectionMethod: "legacy_settings_migration",
  detectionEvidenceJson: "{}",
  confirmationState: "confirmed",
  confirmedAt: "2026-08-04T12:00:00Z",
  lastValidatedAt: null,
  createdAt: "2026-08-04T12:00:00Z",
  updatedAt: "2026-08-04T12:00:00Z",
  roots: [
    {
      profileId: "sims4-main",
      rootId: "mods",
      rootRole: "mods",
      configuredPath: librarySettings.modsPath!,
      required: true,
      validationState: "valid",
      filesystemCapabilitiesJson: "{}",
      lastValidatedAt: null,
      createdAt: "2026-08-04T12:00:00Z",
      updatedAt: "2026-08-04T12:00:00Z",
    },
    {
      profileId: "sims4-main",
      rootId: "tray",
      rootRole: "tray",
      configuredPath: librarySettings.trayPath!,
      required: true,
      validationState: "valid",
      filesystemCapabilitiesJson: "{}",
      lastValidatedAt: null,
      createdAt: "2026-08-04T12:00:00Z",
      updatedAt: "2026-08-04T12:00:00Z",
    },
  ],
};

const validation: GameInstallationProfileValidationReport = {
  profileId: profile.profileId,
  profileName: profile.profileName,
  gameId: profile.gameId,
  storedEnvironment: "native_macos",
  currentEnvironment: "native_macos",
  environmentCompatibility: "matches",
  state: "valid",
  genericRootState: "valid",
  gameSpecificValidationPending: false,
  readOnly: true,
  roots: profile.roots.map((root) => ({
    rootId: root.rootId,
    rootRole: root.rootRole,
    configuredPath: root.configuredPath,
    required: root.required,
    state: "valid",
    absolutePath: true,
    exists: true,
    metadataReadable: true,
    metadataState: "directory",
    canonicalPathDisplay: root.configuredPath,
    caseSensitivity: "insensitive",
    symlinkObserved: false,
    blockers: [],
    reviewNotes: [],
  })),
  gameReadiness: {
    adapterId: "sims4_v1",
    complete: true,
    state: "valid",
    supportedModExtensions: [".package", ".ts4script"],
    supportedTrayExtensions: [".trayitem", ".blueprint"],
    maxScriptFolderDepth: 1,
    maxPackageFolderDepth: 5,
    evidence: [
      {
        code: "sims4_user_data_inferred",
        strength: "strongly_supported",
        summary: "Mods and Tray are separate directories with one shared parent.",
        rootIds: ["mods", "tray"],
      },
    ],
    blockers: [],
    reviewNotes: [],
  },
  blockers: [],
  reviewNotes: [],
};

function renderSettings() {
  render(
    <UiPreferencesProvider mode="seasoned">
      <SettingsScreen experienceMode="seasoned" onExperienceModeChange={vi.fn()} />
    </UiPreferencesProvider>,
  );
}

async function openGameSetup() {
  fireEvent.click(screen.getByRole("button", { name: /Game setup/i }));
  await screen.findByRole("heading", { name: "Your active Sims 4 profile" });
}

beforeEach(() => {
  vi.mocked(api.getAppBehaviorSettings).mockResolvedValue(appBehavior);
  vi.mocked(api.getLibrarySettings).mockResolvedValue(librarySettings);
  vi.mocked(api.getActiveGameInstallationProfile).mockResolvedValue(profile);
  vi.mocked(api.validateGameInstallationProfile).mockResolvedValue(validation);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  localStorage.clear();
});

it("shows active profile readiness and evidence without mutation controls", async () => {
  renderSettings();
  expect(api.getActiveGameInstallationProfile).not.toHaveBeenCalled();

  await openGameSetup();

  expect(await screen.findByText(profile.profileName)).toBeInTheDocument();
  expect(screen.getByText("Overall readiness")).toBeInTheDocument();
  expect(screen.getAllByText("Ready").length).toBeGreaterThan(0);
  expect(screen.getByText("Shared user-data tree supported")).toBeInTheDocument();
  expect(screen.getByText(/No blockers or review notes/i)).toBeInTheDocument();
  expect(screen.getByText(/No files changed/i)).toBeInTheDocument();

  for (const name of [/Edit profile/i, /Create profile/i, /Select profile/i, /Discover profile/i, /^Apply$/i, /^Restore$/i]) {
    expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
  }
});

it("shows a truthful empty state when no active profile exists", async () => {
  vi.mocked(api.getActiveGameInstallationProfile).mockResolvedValue(null);

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText("No active game profile")).toBeInTheDocument();
  expect(screen.getByText(/creation and selection are not available/i)).toBeInTheDocument();
  expect(api.validateGameInstallationProfile).not.toHaveBeenCalled();
  expect(screen.getByText(/No files changed/i)).toBeInTheDocument();
});

it("keeps the saved profile visible when transient validation fails", async () => {
  vi.mocked(api.validateGameInstallationProfile).mockRejectedValue(
    new Error("Filesystem probe failed"),
  );

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText(profile.profileName)).toBeInTheDocument();
  expect(screen.getByText("Live validation could not finish")).toBeInTheDocument();
  expect(screen.getAllByText("Filesystem probe failed").length).toBeGreaterThan(0);
  expect(screen.getByText(/saved profile is still shown/i)).toBeInTheDocument();
  expect(screen.getByText(/No stored state was changed/i)).toBeInTheDocument();

  await waitFor(() => {
    expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(profile.profileId);
  });
});

it("shows a read error instead of an endless loading state when profile lookup fails", async () => {
  vi.mocked(api.getActiveGameInstallationProfile).mockRejectedValue(
    new Error("Profile lookup failed"),
  );

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText("Active game profile could not be read")).toBeInTheDocument();
  expect(screen.getByText("Profile lookup failed")).toBeInTheDocument();
  expect(screen.getByText(/No profile or filesystem state was changed/i)).toBeInTheDocument();
  expect(api.validateGameInstallationProfile).not.toHaveBeenCalled();
  expect(screen.getByText(/No files changed/i)).toBeInTheDocument();
});
