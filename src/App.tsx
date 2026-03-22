import { Suspense, lazy, useEffect, useEffectEvent, useRef, useState } from "react";
import { AnimatePresence, domAnimation, LazyMotion, m, MotionConfig } from "motion/react";
import { Sidebar } from "./components/layout/Sidebar";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { FieldGuide } from "./components/FieldGuide";
import { ToastContainer } from "./components/Toast";
import { ScannerOverlay } from "./components/ScannerOverlay";
import { ThemeBackdrop } from "./components/ThemeBackdrop";
import { useUiPreferences, UiPreferencesProvider } from "./components/UiPreferencesContext";
import { WorkspaceToolbar } from "./components/WorkspaceToolbar";
import { api, hasTauriRuntime } from "./lib/api";
import { toast } from "./components/Toast";
import {
  experienceModeToLegacyView,
  normalizeExperienceMode,
} from "./lib/experienceMode";
import { getScreenFrameMotion } from "./lib/motion";
import {
  createUpdatesNavigationParams,
  nextUpdatesParamsForPlainNavigation,
  resolveUpdatesNavigationParams,
} from "./lib/updatesNavigation";
import type {
  ExperienceMode,
  LibrarySettings,
  ScanProgress,
  ScanStatus,
  Screen,
  UserView,
  WatchListFilter,
  WorkspaceChange,
  WorkspaceDomain,
} from "./lib/types";

const WORKSPACE_DOMAINS: WorkspaceDomain[] = [
  "home",
  "downloads",
  "library",
  "updates",
  "organize",
  "review",
  "duplicates",
  "creatorAudit",
  "categoryAudit",
  "snapshots",
];

const HomeScreen = lazy(async () => ({
  default: (await import("./screens/HomeScreen")).HomeScreen,
}));
const DownloadsScreen = lazy(async () => ({
  default: (await import("./screens/DownloadsScreen")).DownloadsScreen,
}));
const LibraryScreen = lazy(async () => ({
  default: (await import("./screens/LibraryScreen")).LibraryScreen,
}));
const UpdatesScreen = lazy(async () => ({
  default: (await import("./screens/UpdatesScreen")).UpdatesScreen,
}));
const CreatorAuditScreen = lazy(async () => ({
  default: (await import("./screens/CreatorAuditScreen")).CreatorAuditScreen,
}));
const CategoryAuditScreen = lazy(async () => ({
  default: (await import("./screens/CategoryAuditScreen")).CategoryAuditScreen,
}));
const OrganizeScreen = lazy(async () => ({
  default: (await import("./screens/OrganizeScreen")).OrganizeScreen,
}));
const ReviewScreen = lazy(async () => ({
  default: (await import("./screens/ReviewScreen")).ReviewScreen,
}));
const DuplicatesScreen = lazy(async () => ({
  default: (await import("./screens/DuplicatesScreen")).DuplicatesScreen,
}));
const SettingsScreen = lazy(async () => ({
  default: (await import("./screens/SettingsScreen")).SettingsScreen,
}));

function createInitialWorkspaceVersions(): Record<WorkspaceDomain, number> {
  return WORKSPACE_DOMAINS.reduce(
    (versions, domain) => {
      versions[domain] = 0;
      return versions;
    },
    {} as Record<WorkspaceDomain, number>,
  );
}

function bumpWorkspaceVersions(
  current: Record<WorkspaceDomain, number>,
  domains: WorkspaceDomain[],
) {
  const next = { ...current };
  for (const domain of domains) {
    next[domain] += 1;
  }
  return next;
}

function combineWorkspaceVersions(
  versions: Record<WorkspaceDomain, number>,
  domains: WorkspaceDomain[],
) {
  return domains.reduce((total, domain) => total + versions[domain], 0);
}

function resolveInitialExperienceMode(): ExperienceMode {
  const stored = globalThis.localStorage?.getItem("simsuite:user-view");
  return normalizeExperienceMode(stored) ?? "seasoned";
}

function resolveInitialScreen(): Screen {
  const value = new URLSearchParams(globalThis.location?.search ?? "").get("screen");
  if (
    value === "home" ||
    value === "downloads" ||
    value === "library" ||
    value === "updates" ||
    value === "creatorAudit" ||
    value === "categoryAudit" ||
    value === "duplicates" ||
    value === "organize" ||
    value === "review" ||
    value === "settings"
  ) {
    return value;
  }

  return "home";
}

export default function App() {
  const [experienceMode, setExperienceMode] = useState<ExperienceMode>(
    resolveInitialExperienceMode,
  );

  return (
    <UiPreferencesProvider mode={experienceMode}>
      <LazyMotion features={domAnimation}>
        <MotionConfig reducedMotion="user">
          <AppShell
            experienceMode={experienceMode}
            onExperienceModeChange={setExperienceMode}
          />
        </MotionConfig>
      </LazyMotion>
    </UiPreferencesProvider>
  );
}

function AppShell({
  experienceMode,
  onExperienceModeChange,
}: {
  experienceMode: ExperienceMode;
  onExperienceModeChange: (mode: ExperienceMode) => void;
}) {
  const { theme } = useUiPreferences();
  const [screen, setScreen] = useState<Screen>(resolveInitialScreen);
  const [settings, setSettings] = useState<LibrarySettings | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [workspaceVersions, setWorkspaceVersions] = useState(
    createInitialWorkspaceVersions,
  );
  const [isGuideOpen, setIsGuideOpen] = useState(false);
  const [updatesParams, setUpdatesParams] = useState(() =>
    resolveUpdatesNavigationParams(globalThis.location?.search ?? ""),
  );
  const lastTerminalScanKey = useRef<string | null>(null);
  const startupRefreshAttempted = useRef(false);
  const screenFrameRef = useRef<HTMLDivElement | null>(null);
  const userView: UserView = experienceModeToLegacyView(experienceMode);

  useEffect(() => {
    void api.getLibrarySettings().then(setSettings);
    void api.getScanStatus().then((status) => {
      if (status.state === "running") {
        setIsScanning(true);
        if (status.phase && status.currentItem) {
          setScanProgress({
            totalFiles: status.totalFiles,
            processedFiles: status.processedFiles,
            currentItem: status.currentItem,
            phase: status.phase,
          });
        }
        return;
      }

      setIsScanning(false);
      if (status.state === "succeeded") {
        setScanProgress({
          totalFiles: status.totalFiles,
          processedFiles: status.processedFiles,
          currentItem: status.currentItem ?? "Scan finished",
          phase: status.phase ?? "done",
        });
      }
    });
  }, []);

  useEffect(() => {
    globalThis.scrollTo?.({ top: 0, left: 0, behavior: "auto" });
    screenFrameRef.current?.scrollTo({ top: 0, left: 0, behavior: "auto" });
  }, [screen]);

  useEffect(() => {
    globalThis.localStorage?.setItem("simsuite:user-view", experienceMode);
    document.documentElement.dataset.userView = experienceMode;
  }, [experienceMode]);

  const handleScanEvent = useEffectEvent((progress: ScanProgress) => {
    setScanProgress(progress);
  });

  const applyWorkspaceChange = useEffectEvent((change: WorkspaceChange) => {
    if (!change.domains.length) {
      return;
    }

    setWorkspaceVersions((current) => bumpWorkspaceVersions(current, change.domains));
  });

  const bumpWorkspaceDomains = useEffectEvent((domains: WorkspaceDomain[]) => {
    setWorkspaceVersions((current) => bumpWorkspaceVersions(current, domains));
  });

  const navigateToScreen = useEffectEvent((targetScreen: Screen) => {
    setUpdatesParams((current) =>
      nextUpdatesParamsForPlainNavigation(targetScreen, current),
    );
    setScreen(targetScreen);
  });

  const navigateWithParams = useEffectEvent(
    (
      targetScreen: Screen,
      mode?: "tracked" | "setup" | "review",
      filter?: WatchListFilter,
      fileId?: number,
    ) => {
      if (targetScreen === "updates") {
        setUpdatesParams(createUpdatesNavigationParams(mode, filter, fileId));
      }
      setScreen(targetScreen);
    },
  );

  const handleScanStatus = useEffectEvent((status: ScanStatus) => {
    if (status.state === "running") {
      lastTerminalScanKey.current = null;
      setIsScanning(true);
      if (status.phase && status.currentItem) {
        setScanProgress({
          totalFiles: status.totalFiles,
          processedFiles: status.processedFiles,
          currentItem: status.currentItem,
          phase: status.phase,
        });
      }
      return;
    }

    if (status.state === "succeeded") {
      const terminalKey = `${status.state}:${status.finishedAt ?? ""}:${status.processedFiles}`;
      if (lastTerminalScanKey.current === terminalKey) {
        setIsScanning(false);
        return;
      }

      lastTerminalScanKey.current = terminalKey;
      setScanProgress({
        totalFiles: status.totalFiles,
        processedFiles: status.processedFiles,
        currentItem: status.currentItem ?? "Scan finished",
        phase: status.phase ?? "done",
      });
      setIsScanning(false);
      if (!hasTauriRuntime) {
          bumpWorkspaceDomains([
            "home",
            "library",
            "updates",
            "organize",
            "review",
            "duplicates",
            "creatorAudit",
            "categoryAudit",
          "snapshots",
        ]);
      }
      return;
    }

    if (status.state === "failed") {
      const terminalKey = `${status.state}:${status.finishedAt ?? ""}:${status.error ?? ""}`;
      if (lastTerminalScanKey.current === terminalKey) {
        setIsScanning(false);
        return;
      }

      lastTerminalScanKey.current = terminalKey;
      setScanProgress(null);
      setIsScanning(false);
      if (status.error) {
        console.error(status.error);
      }
      return;
    }

    setIsScanning(false);
  });

  useEffect(() => {
    const unlisten = api.listenToScanProgress(handleScanEvent);
    const unlistenStatus = api.listenToScanStatus(handleScanStatus);
    const unlistenWorkspace = api.listenToWorkspaceChanges(applyWorkspaceChange);

    return () => {
      void unlisten.then((dispose) => dispose());
      void unlistenStatus.then((dispose) => dispose());
      void unlistenWorkspace.then((dispose) => dispose());
    };
  }, [applyWorkspaceChange, handleScanEvent, handleScanStatus]);

  useEffect(() => {
    if (!isScanning) {
      return;
    }

    let cancelled = false;
    const poll = globalThis.setInterval(() => {
      void api.getScanStatus().then((status) => {
        if (cancelled) {
          return;
        }

        if (status.state !== "running") {
          handleScanStatus(status);
        }
      });
    }, 900);

    return () => {
      cancelled = true;
      globalThis.clearInterval(poll);
    };
  }, [isScanning, handleScanStatus]);

  async function saveLibraryPaths(nextSettings: LibrarySettings) {
    try {
      const saved = await api.saveLibraryPaths(nextSettings);
      setSettings(saved);
      toast("success", "Library paths saved successfully");
      if (!hasTauriRuntime) {
        bumpWorkspaceDomains([
            "home",
            "downloads",
            "library",
            "updates",
            "organize",
            "review",
            "duplicates",
            "creatorAudit",
            "categoryAudit",
        ]);
      }
    } catch (error) {
      toast("error", "Failed to save library paths");
      throw error;
    }
  }

  async function startScan() {
    if (!settings?.modsPath && !settings?.trayPath) {
      return;
    }

    setIsScanning(true);
    setScanProgress({
      totalFiles: 0,
      processedFiles: 0,
      currentItem: "Walking configured library folders",
      phase: "collecting",
    });

    try {
      const status = await api.startScan();
      setIsScanning(status.state === "running");
      if (status.phase && status.currentItem) {
        setScanProgress({
          totalFiles: status.totalFiles,
          processedFiles: status.processedFiles,
          currentItem: status.currentItem,
          phase: status.phase,
        });
      }
    } catch (error) {
      setIsScanning(false);
      toast("error", "Failed to start scan. Check your library paths.");
      throw error;
    }
  }

  const attemptStartupRefresh = useEffectEvent(() => {
    void startScan().catch((error) => {
      console.error("Startup library refresh failed.", error);
      startupRefreshAttempted.current = false;
    });
  });

  useEffect(() => {
    if (startupRefreshAttempted.current || isScanning) {
      return;
    }

    if (!settings?.modsPath && !settings?.trayPath) {
      return;
    }

    let cancelled = false;
    void api
      .getHomeOverview()
      .then((overview) => {
        if (cancelled || startupRefreshAttempted.current) {
          return;
        }

        if (!overview.scanNeedsRefresh) {
          startupRefreshAttempted.current = true;
          return;
        }

        startupRefreshAttempted.current = true;
        attemptStartupRefresh();
      })
      .catch((error) => {
        if (!cancelled) {
          console.error("Could not check library refresh state.", error);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [attemptStartupRefresh, isScanning, settings?.modsPath, settings?.trayPath]);

  const currentScreen =
    screen === "home" ? (
      <HomeScreen
        refreshVersion={workspaceVersions.home}
        settings={settings}
        onSettingsChange={saveLibraryPaths}
        onNavigate={navigateToScreen}
        onNavigateWithParams={navigateWithParams}
        onScan={startScan}
        isScanning={isScanning}
        userView={userView}
      />
    ) : screen === "downloads" ? (
      <DownloadsScreen
        refreshVersion={workspaceVersions.downloads}
        onNavigate={navigateToScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "downloads",
              "library",
              "updates",
              "organize",
              "review",
              "duplicates",
              "snapshots",
            ]);
          }
        }}
        userView={userView}
      />
    ) : screen === "library" ? (
      <LibraryScreen
        refreshVersion={workspaceVersions.library}
        onNavigate={navigateToScreen}
        onNavigateWithParams={navigateWithParams}
        userView={userView}
      />
    ) : screen === "updates" ? (
      <UpdatesScreen
        refreshVersion={workspaceVersions.updates}
        onNavigate={navigateToScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains(["home", "library", "updates"]);
          }
        }}
        userView={userView}
        initialMode={updatesParams.mode}
        initialFilter={updatesParams.filter}
        initialFileId={updatesParams.fileId}
      />
    ) : screen === "creatorAudit" ? (
      <CreatorAuditScreen
        refreshVersion={workspaceVersions.creatorAudit}
        onNavigate={navigateToScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
              "updates",
              "organize",
              "review",
              "creatorAudit",
            ]);
          }
        }}
        userView={userView}
      />
    ) : screen === "categoryAudit" ? (
      <CategoryAuditScreen
        refreshVersion={workspaceVersions.categoryAudit}
        onNavigate={navigateToScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
              "updates",
              "organize",
              "review",
              "categoryAudit",
            ]);
          }
        }}
        userView={userView}
      />
    ) : screen === "duplicates" ? (
      <DuplicatesScreen
        refreshVersion={workspaceVersions.duplicates}
        onNavigate={navigateToScreen}
        userView={userView}
      />
    ) : screen === "organize" ? (
      <OrganizeScreen
        refreshVersion={combineWorkspaceVersions(workspaceVersions, [
          "organize",
          "snapshots",
        ])}
        onNavigate={navigateToScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
              "updates",
              "organize",
              "review",
              "duplicates",
              "snapshots",
            ]);
          }
        }}
        userView={userView}
      />
    ) : screen === "settings" ? (
      <SettingsScreen
        experienceMode={experienceMode}
        onExperienceModeChange={onExperienceModeChange}
      />
    ) : (
      <ReviewScreen
        refreshVersion={workspaceVersions.review}
        onNavigate={navigateToScreen}
        userView={userView}
      />
    );

  const screenFrameMotion = getScreenFrameMotion(theme, screen);

  return (
    <div className="app-shell">
      <ThemeBackdrop theme={theme} screen={screen} />
      <Sidebar
        currentScreen={screen}
        experienceMode={experienceMode}
        onNavigate={navigateToScreen}
        onScan={() => void startScan()}
        isScanning={isScanning}
        onOpenGuide={() => setIsGuideOpen(true)}
      />

      <main className="main-shell">
        <WorkspaceToolbar
          experienceMode={experienceMode}
          currentScreen={screen}
          onOpenSettings={() => navigateToScreen("settings")}
        />
        <AnimatePresence mode="wait" initial={false}>
          <m.div
            ref={screenFrameRef}
            className="screen-frame"
            initial={screenFrameMotion.initial}
            animate={screenFrameMotion.animate}
            exit={screenFrameMotion.exit}
            transition={screenFrameMotion.transition}
          >
            <ErrorBoundary>
              <Suspense
                fallback={
                  <div className="state-panel state-panel--loading">
                    Loading workspace view...
                  </div>
                }
              >
                {currentScreen}
              </Suspense>
            </ErrorBoundary>
          </m.div>
        </AnimatePresence>
      </main>

      <AnimatePresence>
        {isScanning ? (
          <ScannerOverlay progress={scanProgress} experienceMode={experienceMode} />
        ) : null}
      </AnimatePresence>
      <FieldGuide
        open={isGuideOpen}
        screen={screen}
        experienceMode={experienceMode}
        onClose={() => setIsGuideOpen(false)}
      />
      <ToastContainer />
    </div>
  );
}
