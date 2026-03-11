import { Suspense, lazy, useEffect, useEffectEvent, useRef, useState } from "react";
import { AnimatePresence, domAnimation, LazyMotion, m, MotionConfig } from "motion/react";
import { Sidebar } from "./components/layout/Sidebar";
import { FieldGuide } from "./components/FieldGuide";
import { ScannerOverlay } from "./components/ScannerOverlay";
import { ThemeBackdrop } from "./components/ThemeBackdrop";
import { useUiPreferences, UiPreferencesProvider } from "./components/UiPreferencesContext";
import { WorkspaceToolbar } from "./components/WorkspaceToolbar";
import { api, hasTauriRuntime } from "./lib/api";
import {
  experienceModeToLegacyView,
  normalizeExperienceMode,
} from "./lib/experienceMode";
import { getScreenFrameMotion } from "./lib/motion";
import type {
  ExperienceMode,
  LibrarySettings,
  ScanProgress,
  ScanStatus,
  Screen,
  UserView,
  WorkspaceChange,
  WorkspaceDomain,
} from "./lib/types";

const WORKSPACE_DOMAINS: WorkspaceDomain[] = [
  "home",
  "downloads",
  "library",
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
  const lastTerminalScanKey = useRef<string | null>(null);
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
    const saved = await api.saveLibraryPaths(nextSettings);
    setSettings(saved);
    if (!hasTauriRuntime) {
      bumpWorkspaceDomains([
        "home",
        "downloads",
        "library",
        "organize",
        "review",
        "duplicates",
        "creatorAudit",
        "categoryAudit",
      ]);
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
      throw error;
    }
  }

  const currentScreen =
    screen === "home" ? (
      <HomeScreen
        refreshVersion={workspaceVersions.home}
        settings={settings}
        onSettingsChange={saveLibraryPaths}
        onNavigate={setScreen}
        onScan={startScan}
        isScanning={isScanning}
        userView={userView}
      />
    ) : screen === "downloads" ? (
      <DownloadsScreen
        refreshVersion={workspaceVersions.downloads}
        onNavigate={setScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "downloads",
              "library",
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
        onNavigate={setScreen}
        userView={userView}
      />
    ) : screen === "creatorAudit" ? (
      <CreatorAuditScreen
        refreshVersion={workspaceVersions.creatorAudit}
        onNavigate={setScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
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
        onNavigate={setScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
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
        onNavigate={setScreen}
        userView={userView}
      />
    ) : screen === "organize" ? (
      <OrganizeScreen
        refreshVersion={combineWorkspaceVersions(workspaceVersions, [
          "organize",
          "snapshots",
        ])}
        onNavigate={setScreen}
        onDataChanged={() => {
          if (!hasTauriRuntime) {
            bumpWorkspaceDomains([
              "home",
              "library",
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
        onNavigate={setScreen}
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
        onNavigate={setScreen}
        onScan={() => void startScan()}
        isScanning={isScanning}
        onOpenGuide={() => setIsGuideOpen(true)}
      />

      <main className="main-shell">
        <WorkspaceToolbar
          experienceMode={experienceMode}
          currentScreen={screen}
          onOpenSettings={() => setScreen("settings")}
        />
        <AnimatePresence mode="wait" initial={false}>
          <m.div
            key={screen}
            className="screen-frame"
            initial={screenFrameMotion.initial}
            animate={screenFrameMotion.animate}
            exit={screenFrameMotion.exit}
            transition={screenFrameMotion.transition}
          >
            <Suspense
              fallback={
                <div className="state-panel state-panel--loading">
                  Loading workspace view...
                </div>
              }
            >
              {currentScreen}
            </Suspense>
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
    </div>
  );
}
