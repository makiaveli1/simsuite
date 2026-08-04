import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { UiPreferencesProvider } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import type {
  AppBehaviorSettings,
  GameInstallationCandidate,
  GameInstallationProfile,
  GameInstallationProfileValidationReport,
  LibrarySettings,
} from "../lib/types";
import { SettingsScreen } from "./SettingsScreen";

vi.mock("../lib/api", () => ({
  api: {
    getAppBehaviorSettings: vi.fn(),
    getLibrarySettings: vi.fn(),
    listGameInstallationProfiles: vi.fn(),
    getActiveGameInstallationProfile: vi.fn(),
    detectGameInstallationCandidates: vi.fn(),
    createManualGameInstallationProfile: vi.fn(),
    confirmManualGameInstallationProfile: vi.fn(),
    setActiveGameInstallationProfile: vi.fn(),
    validateGameInstallationProfile: vi.fn(),
    pickFolder: vi.fn(),
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

const alternateProfile: GameInstallationProfile = {
  ...profile,
  profileId: "sims4-alt",
  profileName: "Alternate Sims 4 setup",
  roots: profile.roots.map((root) => ({
    ...root,
    profileId: "sims4-alt",
    configuredPath: root.configuredPath.replace("/Users/player", "/Users/alternate"),
  })),
};

const manualProfile: GameInstallationProfile = {
  ...profile,
  profileId: "sims4-manual",
  profileName: "Manual Sims 4 setup",
  status: "draft",
  detectionMethod: "manual",
  confirmationState: "unconfirmed",
  confirmedAt: null,
  roots: profile.roots.map((root) => ({
    ...root,
    profileId: "sims4-manual",
    validationState: "unvalidated",
    filesystemCapabilitiesJson: "{}",
    lastValidatedAt: null,
  })),
};

const foreignProfile: GameInstallationProfile = {
  ...profile,
  profileId: "sims4-windows",
  profileName: "Windows Sims 4 setup",
  operatingEnvironment: "native_windows",
  roots: profile.roots.map((root) => ({
    ...root,
    profileId: "sims4-windows",
    configuredPath: `C:\\Users\\Player\\${root.rootId}`,
  })),
};

const macosCandidate: GameInstallationCandidate = {
  candidateId: "macos_documents_directory",
  rank: 1,
  gameId: "sims4",
  operatingEnvironment: "native_macos",
  suggestedName: "Sims 4 in macOS Documents",
  confidence: "strong",
  readOnly: true,
  detectionEvidence: [
    "The macOS Documents location resolved successfully.",
    "An existing Sims 4 user-data directory was found.",
    "An existing Mods directory was found inside this setup.",
    "An existing Tray directory was found inside this setup.",
  ],
  warnings: [],
  suggestedRoots: [
    {
      rootId: "user_data",
      rootRole: "game_user_data",
      configuredPath: "/Users/player/Documents/Electronic Arts/The Sims 4",
      required: false,
      exists: true,
    },
    {
      rootId: "mods",
      rootRole: "installed_mods",
      configuredPath: librarySettings.modsPath!,
      required: true,
      exists: true,
    },
    {
      rootId: "tray",
      rootRole: "installed_tray",
      configuredPath: librarySettings.trayPath!,
      required: true,
      exists: true,
    },
    {
      rootId: "downloads",
      rootRole: "intake_downloads",
      configuredPath: librarySettings.downloadsPath!,
      required: false,
      exists: true,
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
  vi.mocked(api.listGameInstallationProfiles).mockResolvedValue([
    profile,
    alternateProfile,
    foreignProfile,
  ]);
  vi.mocked(api.getActiveGameInstallationProfile).mockResolvedValue(profile);
  vi.mocked(api.detectGameInstallationCandidates).mockResolvedValue({
    currentEnvironment: "native_macos",
    supported: true,
    candidates: [macosCandidate],
    readOnly: true,
    reviewNotes: [],
  });
  vi.mocked(api.createManualGameInstallationProfile).mockResolvedValue(manualProfile);
  vi.mocked(api.confirmManualGameInstallationProfile).mockResolvedValue({
    profile: {
      ...manualProfile,
      status: "valid",
      confirmationState: "confirmed",
      confirmedAt: "2026-08-04T15:00:00Z",
      lastValidatedAt: "2026-08-04T15:00:00Z",
    },
    validation: {
      ...validation,
      profileId: manualProfile.profileId,
      profileName: manualProfile.profileName,
      roots: manualProfile.roots.map((root) => ({
        ...validation.roots.find((item) => item.rootId === root.rootId)!,
        rootId: root.rootId,
        rootRole: root.rootRole,
        configuredPath: root.configuredPath,
        required: root.required,
      })),
    },
  });
  vi.mocked(api.pickFolder).mockResolvedValue(null);
  vi.mocked(api.setActiveGameInstallationProfile).mockImplementation(
    async (profileId) => {
      if (profileId === alternateProfile.profileId) {
        return alternateProfile;
      }
      if (profileId === manualProfile.profileId) {
        return {
          ...manualProfile,
          status: "valid",
          confirmationState: "confirmed",
          confirmedAt: "2026-08-04T15:00:00Z",
          lastValidatedAt: "2026-08-04T15:00:00Z",
        };
      }
      if (profileId === profile.profileId) {
        return profile;
      }
      throw new Error(`Game installation profile '${profileId}' does not exist.`);
    },
  );
  vi.mocked(api.validateGameInstallationProfile).mockImplementation(
    async (profileId) => {
      if (profileId === manualProfile.profileId) {
        return {
          ...validation,
          profileId: manualProfile.profileId,
          profileName: manualProfile.profileName,
          roots: manualProfile.roots.map((root) => ({
            ...validation.roots.find((item) => item.rootId === root.rootId)!,
            rootId: root.rootId,
            rootRole: root.rootRole,
            configuredPath: root.configuredPath,
            required: root.required,
          })),
        };
      }
      if (profileId === alternateProfile.profileId) {
        return {
          ...validation,
          profileId: alternateProfile.profileId,
          profileName: alternateProfile.profileName,
          roots: validation.roots.map((root) => ({
            ...root,
            configuredPath: root.configuredPath.replace(
              "/Users/player",
              "/Users/alternate",
            ),
            canonicalPathDisplay:
              root.canonicalPathDisplay?.replace(
                "/Users/player",
                "/Users/alternate",
              ) ?? null,
          })),
        };
      }
      if (profileId === foreignProfile.profileId) {
        return {
          ...validation,
          profileId: foreignProfile.profileId,
          profileName: foreignProfile.profileName,
          storedEnvironment: "native_windows",
          environmentCompatibility: "mismatch",
          state: "needs_review",
          genericRootState: "needs_review",
          gameSpecificValidationPending: true,
          gameReadiness: null,
          roots: foreignProfile.roots.map((root) => ({
            rootId: root.rootId,
            rootRole: root.rootRole,
            configuredPath: root.configuredPath,
            required: root.required,
            state: "needs_review" as const,
            absolutePath: null,
            exists: null,
            metadataReadable: null,
            metadataState: "not_checked" as const,
            canonicalPathDisplay: null,
            caseSensitivity: "not_checked" as const,
            symlinkObserved: null,
            blockers: [],
            reviewNotes: ["Foreign-native path probing was skipped."],
          })),
          reviewNotes: [
            "This profile targets a different native operating system.",
          ],
        };
      }
      return validation;
    },
  );
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  localStorage.clear();
});

it("shows active readiness with a guarded selector and no file apply controls", async () => {
  renderSettings();
  expect(api.getActiveGameInstallationProfile).not.toHaveBeenCalled();

  await openGameSetup();

  expect(await screen.findByText(profile.profileName)).toBeInTheDocument();
  expect(screen.getByRole("combobox", { name: /Saved Sims 4 setup/i })).toHaveValue(
    profile.profileId,
  );
  expect(screen.getByRole("button", { name: "Already active" })).toBeDisabled();
  expect(screen.getByText("Overall readiness")).toBeInTheDocument();
  expect(screen.getAllByText("Ready").length).toBeGreaterThan(0);
  expect(screen.getByText("Shared user-data tree supported")).toBeInTheDocument();
  expect(screen.getByText(/No blockers or review notes/i)).toBeInTheDocument();
  expect(screen.getAllByText(/does not run Apply or Restore/i).length).toBeGreaterThan(0);

  for (const name of [
    /Edit profile/i,
    /Create profile/i,
    /Discover profile/i,
    /^Apply$/i,
    /^Restore$/i,
  ]) {
    expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
  }
});

it("copies a read-only macOS suggestion into the manual form without saving or activating it", async () => {
  renderSettings();
  expect(api.detectGameInstallationCandidates).not.toHaveBeenCalled();

  await openGameSetup();

  expect(api.detectGameInstallationCandidates).toHaveBeenCalledTimes(1);
  expect(
    await screen.findByRole("heading", {
      name: "Existing setups found on this Mac",
    }),
  ).toBeInTheDocument();
  expect(screen.getByText(macosCandidate.suggestedName)).toBeInTheDocument();
  expect(screen.getByText("Strong match")).toBeInTheDocument();
  expect(screen.getByText(/temporary until you review and save it manually/i)).toBeInTheDocument();
  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();

  fireEvent.click(
    screen.getByRole("button", { name: /Review this suggestion/i }),
  );

  expect(
    await screen.findByDisplayValue(macosCandidate.suggestedName),
  ).toBeInTheDocument();
  expect(screen.getByDisplayValue(librarySettings.modsPath!)).toBeInTheDocument();
  expect(screen.getByDisplayValue(librarySettings.trayPath!)).toBeInTheDocument();
  expect(
    screen.getByText(/has not been saved, confirmed, or activated/i),
  ).toBeInTheDocument();
  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.confirmManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
});

it("keeps saved profile evidence available when candidate detection fails", async () => {
  vi.mocked(api.detectGameInstallationCandidates).mockRejectedValue(
    new Error("Candidate probe failed"),
  );

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText(profile.profileName)).toBeInTheDocument();
  expect(
    screen.getByText(/Automatic setup suggestions could not be checked/i),
  ).toBeInTheDocument();
  expect(screen.getByText(/Candidate probe failed/i)).toBeInTheDocument();
  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
});

it("surfaces macOS locations that could not be inspected", async () => {
  vi.mocked(api.detectGameInstallationCandidates).mockResolvedValue({
    currentEnvironment: "native_macos",
    supported: true,
    candidates: [],
    readOnly: true,
    reviewNotes: ["SimSuite could not inspect the macOS Documents location: permission denied"],
  });

  renderSettings();
  await openGameSetup();

  expect(
    await screen.findByText(/Some standard locations could not be inspected/i),
  ).toBeInTheDocument();
  expect(screen.getByText(/permission denied/i)).toBeInTheDocument();
  expect(screen.queryByText(/No standard macOS setup was found/i)).not.toBeInTheDocument();
  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
});

it("shows a truthful manual-only state on hosts without candidate support", async () => {
  vi.mocked(api.detectGameInstallationCandidates).mockResolvedValue({
    currentEnvironment: "native_windows",
    supported: false,
    candidates: [],
    readOnly: true,
    reviewNotes: [
      "Automatic Sims 4 folder suggestions are not available on this operating system yet. Use the guarded manual folder chooser.",
    ],
  });

  renderSettings();
  await openGameSetup();

  expect(
    await screen.findByRole("heading", {
      name: /Automatic suggestions are not available on this host/i,
    }),
  ).toBeInTheDocument();
  expect(screen.getByText(/Native Windows/i)).toBeInTheDocument();
  expect(screen.getByText(/Manual setup remains available/i)).toBeInTheDocument();
  expect(screen.queryByText(/No standard macOS setup was found/i)).not.toBeInTheDocument();
  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
});

it("lets an empty active state choose from real saved profiles", async () => {
  vi.mocked(api.getActiveGameInstallationProfile).mockResolvedValue(null);

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText("No active game profile")).toBeInTheDocument();
  expect(screen.getByText(/Choose one of the saved setups above/i)).toBeInTheDocument();
  expect(
    await screen.findByRole("button", { name: /Use selected profile/i }),
  ).toBeEnabled();
  expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(profile.profileId);
  expect(screen.getAllByText(/does not run Apply or Restore/i).length).toBeGreaterThan(0);
});

it("switches only after an explicit selection and confirmation click", async () => {
  renderSettings();
  await openGameSetup();

  const selector = screen.getByRole("combobox", { name: /Saved Sims 4 setup/i });
  fireEvent.change(selector, { target: { value: alternateProfile.profileId } });
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();

  fireEvent.click(
    await screen.findByRole("button", { name: /Use selected profile/i }),
  );

  await waitFor(() => {
    expect(api.setActiveGameInstallationProfile).toHaveBeenCalledWith(
      alternateProfile.profileId,
    );
  });
  expect(await screen.findByText(/is now the active setup/i)).toBeInTheDocument();
  expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(
    alternateProfile.profileId,
  );
  expect(selector).toHaveValue(alternateProfile.profileId);
  expect(screen.getByRole("button", { name: "Already active" })).toBeDisabled();
});

it("creates, validates, confirms, and activates a manual profile as separate steps", async () => {
  renderSettings();
  await openGameSetup();

  fireEvent.click(
    screen.getByRole("button", { name: /Choose folders manually/i }),
  );
  fireEvent.change(
    screen.getByPlaceholderText("For example, Main Sims 4 setup"),
    { target: { value: manualProfile.profileName } },
  );
  fireEvent.change(screen.getByPlaceholderText("Choose mods folder"), {
    target: { value: librarySettings.modsPath },
  });
  fireEvent.change(screen.getByPlaceholderText("Choose tray folder"), {
    target: { value: librarySettings.trayPath },
  });

  expect(api.createManualGameInstallationProfile).not.toHaveBeenCalled();
  fireEvent.click(
    screen.getByRole("button", { name: /Save unconfirmed draft/i }),
  );

  await waitFor(() => {
    expect(api.createManualGameInstallationProfile).toHaveBeenCalledWith({
      profileName: manualProfile.profileName,
      userDataPath: null,
      modsPath: librarySettings.modsPath,
      trayPath: librarySettings.trayPath,
      downloadsPath: null,
    });
  });
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
  expect(
    await screen.findByText(/was saved as an unconfirmed draft/i),
  ).toBeInTheDocument();
  await waitFor(() => {
    expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(
      manualProfile.profileId,
    );
  });
  expect(await screen.findByText("Selected draft evidence")).toBeInTheDocument();
  expect(
    screen.getByText(`Confirm “${manualProfile.profileName}”`),
  ).toBeInTheDocument();
  expect(screen.getByText("Live status")).toBeInTheDocument();
  expect(
    screen.getAllByText(librarySettings.modsPath ?? "").length,
  ).toBeGreaterThan(0);

  const confirmButton = await screen.findByRole("button", {
    name: /Confirm validated profile/i,
  });
  expect(confirmButton).toBeEnabled();
  fireEvent.click(confirmButton);

  await waitFor(() => {
    expect(api.confirmManualGameInstallationProfile).toHaveBeenCalledWith(
      manualProfile.profileId,
    );
  });
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
  expect(
    await screen.findByText(/is confirmed.*still not active/i),
  ).toBeInTheDocument();

  fireEvent.click(
    await screen.findByRole("button", { name: /Use selected profile/i }),
  );
  await waitFor(() => {
    expect(api.setActiveGameInstallationProfile).toHaveBeenCalledWith(
      manualProfile.profileId,
    );
  });
  expect(await screen.findByText(/is now the active setup/i)).toBeInTheDocument();
});

it("keeps a foreign-host profile visible but blocks activation", async () => {
  renderSettings();
  await openGameSetup();

  const selector = screen.getByRole("combobox", { name: /Saved Sims 4 setup/i });
  fireEvent.change(selector, { target: { value: foreignProfile.profileId } });

  expect(
    await screen.findByText(/belongs to a different native operating system/i),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: /Unavailable on this host/i }),
  ).toBeDisabled();
  expect(selector).toHaveValue(foreignProfile.profileId);
  expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(
    foreignProfile.profileId,
  );
  expect(api.setActiveGameInstallationProfile).not.toHaveBeenCalled();
});

it("reconciles to the stored profile when switching reports an error", async () => {
  vi.mocked(api.setActiveGameInstallationProfile).mockRejectedValue(
    new Error("Switch blocked"),
  );

  renderSettings();
  await openGameSetup();

  const selector = screen.getByRole("combobox", { name: /Saved Sims 4 setup/i });
  fireEvent.change(selector, { target: { value: alternateProfile.profileId } });
  fireEvent.click(
    await screen.findByRole("button", { name: /Use selected profile/i }),
  );

  expect(await screen.findByText(/Switch blocked/i)).toBeInTheDocument();
  expect(screen.getByText(/rechecked the database/i)).toBeInTheDocument();
  expect(selector).toHaveValue(profile.profileId);
  expect(screen.getByRole("button", { name: "Already active" })).toBeDisabled();
});

it("keeps active evidence visible when the saved profile list fails", async () => {
  vi.mocked(api.listGameInstallationProfiles).mockRejectedValue(
    new Error("Profile list failed"),
  );

  renderSettings();
  await openGameSetup();

  expect(await screen.findByText(profile.profileName)).toBeInTheDocument();
  expect(screen.getByText(/Saved profile list could not be read/i)).toBeInTheDocument();
  expect(screen.getByText(/switching is disabled/i)).toBeInTheDocument();
  expect(screen.queryByRole("combobox", { name: /Saved Sims 4 setup/i })).not.toBeInTheDocument();
  expect(api.validateGameInstallationProfile).toHaveBeenCalledWith(profile.profileId);
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
  expect(screen.getByText(/does not run Apply or Restore/i)).toBeInTheDocument();
});
