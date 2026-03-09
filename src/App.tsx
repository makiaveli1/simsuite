import { useEffect, useEffectEvent, useRef, useState } from "react";
import { AnimatePresence, domAnimation, LazyMotion, m, MotionConfig } from "motion/react";
import { HomeScreen } from "./screens/HomeScreen";
import { DownloadsScreen } from "./screens/DownloadsScreen";
import { LibraryScreen } from "./screens/LibraryScreen";
import { CreatorAuditScreen } from "./screens/CreatorAuditScreen";
import { CategoryAuditScreen } from "./screens/CategoryAuditScreen";
import { OrganizeScreen } from "./screens/OrganizeScreen";
import { ReviewScreen } from "./screens/ReviewScreen";
import { DuplicatesScreen } from "./screens/DuplicatesScreen";
import { SettingsScreen } from "./screens/SettingsScreen";
import { Sidebar } from "./components/layout/Sidebar";
import { FieldGuide } from "./components/FieldGuide";
import { ScannerOverlay } from "./components/ScannerOverlay";
import { ThemeBackdrop } from "./components/ThemeBackdrop";
import { useUiPreferences, UiPreferencesProvider } from "./components/UiPreferencesContext";
import { WorkspaceToolbar } from "./components/WorkspaceToolbar";
import { api } from "./lib/api";
import { getScreenFrameMotion } from "./lib/motion";
import type {
  LibrarySettings,
  ScanProgress,
  ScanStatus,
  Screen,
  UserView,
} from "./lib/types";

function resolveInitialUserView(): UserView {
  const stored = globalThis.localStorage?.getItem("simsuite:user-view");
  if (stored === "beginner" || stored === "standard" || stored === "power") {
    return stored;
  }

  return "standard";
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
  return (
    <UiPreferencesProvider>
      <LazyMotion features={domAnimation}>
        <MotionConfig reducedMotion="user">
          <AppShell />
        </MotionConfig>
      </LazyMotion>
    </UiPreferencesProvider>
  );
}

function AppShell() {
  const { theme } = useUiPreferences();
  const [screen, setScreen] = useState<Screen>(resolveInitialScreen);
  const [settings, setSettings] = useState<LibrarySettings | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [refreshVersion, setRefreshVersion] = useState(0);
  const [isGuideOpen, setIsGuideOpen] = useState(false);
  const [userView, setUserView] = useState<UserView>(resolveInitialUserView);
  const lastTerminalScanKey = useRef<string | null>(null);

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
    globalThis.localStorage?.setItem("simsuite:user-view", userView);
  }, [userView]);

  const handleScanEvent = useEffectEvent((progress: ScanProgress) => {
    setScanProgress(progress);
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
      setRefreshVersion((current) => current + 1);
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

    return () => {
      void unlisten.then((dispose) => dispose());
      void unlistenStatus.then((dispose) => dispose());
    };
  }, [handleScanEvent, handleScanStatus]);

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
    setRefreshVersion((current) => current + 1);
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
        refreshVersion={refreshVersion}
        settings={settings}
        onSettingsChange={saveLibraryPaths}
        onNavigate={setScreen}
        onScan={startScan}
        isScanning={isScanning}
        userView={userView}
      />
    ) : screen === "downloads" ? (
      <DownloadsScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        onDataChanged={() => setRefreshVersion((current) => current + 1)}
        userView={userView}
      />
    ) : screen === "library" ? (
      <LibraryScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        userView={userView}
      />
    ) : screen === "creatorAudit" ? (
      <CreatorAuditScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        onDataChanged={() => setRefreshVersion((current) => current + 1)}
        userView={userView}
      />
    ) : screen === "categoryAudit" ? (
      <CategoryAuditScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        onDataChanged={() => setRefreshVersion((current) => current + 1)}
        userView={userView}
      />
    ) : screen === "duplicates" ? (
      <DuplicatesScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        userView={userView}
      />
    ) : screen === "organize" ? (
      <OrganizeScreen
        refreshVersion={refreshVersion}
        onNavigate={setScreen}
        onDataChanged={() => setRefreshVersion((current) => current + 1)}
        userView={userView}
      />
    ) : screen === "settings" ? (
      <SettingsScreen userView={userView} onUserViewChange={setUserView} />
    ) : (
      <ReviewScreen
        refreshVersion={refreshVersion}
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
        userView={userView}
        onNavigate={setScreen}
        onScan={() => void startScan()}
        isScanning={isScanning}
        onOpenGuide={() => setIsGuideOpen(true)}
      />

      <main className="main-shell">
        <WorkspaceToolbar
          userView={userView}
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
            {currentScreen}
          </m.div>
        </AnimatePresence>
      </main>

      <AnimatePresence>
        {isScanning ? (
          <ScannerOverlay progress={scanProgress} userView={userView} />
        ) : null}
      </AnimatePresence>
      <FieldGuide
        open={isGuideOpen}
        screen={screen}
        userView={userView}
        onClose={() => setIsGuideOpen(false)}
      />
    </div>
  );
}
