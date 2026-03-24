import { useEffect, useState, type ReactNode } from "react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import {
  Activity,
  Check,
  FolderCog,
  FolderOpen,
  LibraryBig,
  Palette,
  RefreshCw,
  ScanSearch,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  X,
} from "lucide-react";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { hoverLift, overlayTransition, panelSpring, stagedListItem, tapPress } from "../lib/motion";
import { isNudgeDismissed, clearNudgeDismissed } from "../lib/guidedFlowStorage";
import { UI_THEMES, getThemeDefinition } from "../lib/themeMeta";
import type {
  DetectedLibraryPaths,
  HomeOverview,
  LibrarySettings,
  Screen,
  UserView,
  WatchListFilter,
} from "../lib/types";
import {
  allowedHomeModules,
  HOME_DENSITY_OPTIONS,
  HOME_MODULE_LABELS,
  HOME_MODULE_ORDER,
  buildHeroState,
  defaultHomePrefs,
  describeHomeModule,
  formatHomeTime,
  formatTimestamp,
  getHomeGreeting,
  normalizeHomePrefs,
  readHomePrefs,
  saveHomePrefs,
  type HomeDisplayPrefs,
  type HomeHeroFocus,
  type HomeModuleId,
} from "./homeDisplay";

interface HomeScreenProps {
  refreshVersion: number;
  settings: LibrarySettings | null;
  onSettingsChange: (settings: LibrarySettings) => Promise<void>;
  onNavigate: (screen: Screen) => void;
  onNavigateWithParams: (
    screen: Screen,
    mode?: "tracked" | "setup" | "review",
    filter?: WatchListFilter,
  ) => void;
  onScan: () => Promise<void>;
  isScanning: boolean;
  userView: UserView;
}

export function HomeScreen({
  refreshVersion,
  settings,
  onSettingsChange,
  onScan,
  isScanning,
  userView,
}: HomeScreenProps) {
  const [overview, setOverview] = useState<HomeOverview | null>(null);
  const [detectedPaths, setDetectedPaths] = useState<DetectedLibraryPaths | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [customizing, setCustomizing] = useState(false);
  const [displayPrefs, setDisplayPrefs] = useState<HomeDisplayPrefs>(() =>
    readHomePrefs(userView),
  );
  const { theme, density, setTheme, setDensity } = useUiPreferences();
  const reducedMotion = useReducedMotion();
  const activeTheme = getThemeDefinition(theme);
  const ambientHero = displayPrefs.ambientMotion && !reducedMotion;

  useEffect(() => {
    void api.getHomeOverview().then(setOverview);
  }, [refreshVersion]);

  useEffect(() => {
    void api.detectDefaultLibraryPaths().then(setDetectedPaths);
  }, []);

  useEffect(() => {
    setDisplayPrefs(readHomePrefs(userView));
  }, [userView]);

  useEffect(() => {
    saveHomePrefs(userView, displayPrefs);
  }, [displayPrefs, userView]);

  async function chooseFolder(kind: "modsPath" | "trayPath" | "downloadsPath") {
    const title =
      kind === "modsPath"
        ? "Choose your Sims Mods folder"
        : kind === "trayPath"
          ? "Choose your Sims Tray folder"
          : "Choose your Downloads folder";
    const picked = await api.pickFolder(title);
    if (!picked || !settings) {
      return;
    }

    setIsSaving(true);
    try {
      await onSettingsChange({ ...settings, [kind]: picked });
    } finally {
      setIsSaving(false);
    }
  }

  async function applyDetectedPaths() {
    if (!detectedPaths || !settings) {
      return;
    }

    setIsSaving(true);
    try {
      await onSettingsChange({
        modsPath: detectedPaths.modsPath ?? settings.modsPath,
        trayPath: detectedPaths.trayPath ?? settings.trayPath,
        downloadsPath: detectedPaths.downloadsPath ?? settings.downloadsPath,
        downloadRejectFolder: settings.downloadRejectFolder,
      });
    } finally {
      setIsSaving(false);
    }
  }

  async function chooseFirstMissingFolder() {
    if (!settings) {
      return;
    }
    if (!settings.modsPath) {
      await chooseFolder("modsPath");
      return;
    }
    if (!settings.trayPath) {
      await chooseFolder("trayPath");
      return;
    }
    if (!settings.downloadsPath) {
      await chooseFolder("downloadsPath");
    }
  }

  function updateDisplayPrefs(next: Partial<HomeDisplayPrefs>) {
    setDisplayPrefs((current) => normalizeHomePrefs({ ...current, ...next }, userView));
  }

  function setModuleVisible(moduleId: HomeModuleId, visible: boolean) {
    setDisplayPrefs((current) => {
      const nextVisible = { ...current.visibleModules, [moduleId]: visible };
      if (!visible && Object.values(nextVisible).filter(Boolean).length === 0) {
        return current;
      }
      return { ...current, visibleModules: nextVisible };
    });
  }

  const now = new Date();
  const greeting = getHomeGreeting(now);
  const canScan = Boolean(settings?.modsPath || settings?.trayPath);
  const sourceCount =
    Number(Boolean(settings?.modsPath)) +
    Number(Boolean(settings?.trayPath)) +
    Number(Boolean(settings?.downloadsPath));
  const hasDetectedPathSuggestion = Boolean(
    detectedPaths &&
      ((!settings?.modsPath && detectedPaths.modsPath) ||
        (!settings?.trayPath && detectedPaths.trayPath) ||
        (!settings?.downloadsPath && detectedPaths.downloadsPath)),
  );
  const totalAttentionCount =
    (overview?.downloadsCount ?? 0) +
    (overview?.reviewCount ?? 0) +
    (overview?.exactUpdateItems ?? 0) +
    (overview?.watchSetupItems ?? 0);
  const totalWatchCount =
    (overview?.exactUpdateItems ?? 0) +
    (overview?.possibleUpdateItems ?? 0) +
    (overview?.unknownWatchItems ?? 0) +
    (overview?.watchReviewItems ?? 0) +
    (overview?.watchSetupItems ?? 0);
  const calmDetails = userView === "beginner";
  const denseDetails = userView === "power";
  const allowedModules = allowedHomeModules(userView);
  const heroState = buildHeroState({
    focus: displayPrefs.focus,
    overview,
    userView,
    sourceCount,
    totalWatchCount,
    totalAttentionCount,
  });
  const visibleModules = HOME_MODULE_ORDER.filter(
    (moduleId) => displayPrefs.visibleModules[moduleId],
  ).filter((moduleId) => allowedModules.includes(moduleId));
  const moduleBands = buildModuleBands(userView, visibleModules);
  const watchSetupLabel = userView === "beginner" ? "Pages to save" : "Need source setup";
  const watchFollowupLabel = userView === "beginner" ? "Needs follow-up" : "Watch review";

  const snapshotRows = [
    ["Inbox", (overview?.downloadsCount ?? 0).toLocaleString(), "Fresh downloads still waiting for a safe pass."],
    [userView === "beginner" ? "Needs review" : "Review", (overview?.reviewCount ?? 0).toLocaleString(), "Files that still need a human check."],
    ["Confirmed updates", (overview?.exactUpdateItems ?? 0).toLocaleString(), "Tracked pages with a clear newer version waiting."],
    ...(!denseDetails
      ? []
      : [[
          "Pages to set",
          (overview?.watchSetupItems ?? 0).toLocaleString(),
          "Installed files that still need one saved page.",
        ]]),
  ] as const;

  const healthRows = [
    ["Scan state", overview?.scanNeedsRefresh ? "Needs refresh" : "Current"],
    ["Safety mode", overview?.readOnlyMode ? "Read-only on" : "Read-only off"],
    ["Risky files", (overview?.unsafeCount ?? 0).toLocaleString()],
    ["Duplicates", (overview?.duplicatesCount ?? 0).toLocaleString()],
    ...(!denseDetails ? [] : [[userView === "power" ? "Script mods" : "Scripts", (overview?.scriptModsCount ?? 0).toLocaleString()]]),
  ] as const;

  const watchRows = [
    ["Confirmed updates", (overview?.exactUpdateItems ?? 0).toLocaleString(), "Pages that already look like real new versions."],
    ["Possible updates", (overview?.possibleUpdateItems ?? 0).toLocaleString(), "Pages that changed but still need a little caution."],
    [watchSetupLabel, (overview?.watchSetupItems ?? 0).toLocaleString(), "Installed items that still need a saved page first."],
    ...(!calmDetails
      ? [[watchFollowupLabel, (overview?.watchReviewItems ?? 0).toLocaleString(), "Reminder-only or provider-backed pages that stay cautious."]]
      : []),
    ...(!denseDetails
      ? []
      : [["Unclear results", (overview?.unknownWatchItems ?? 0).toLocaleString(), "Saved pages that still do not give a clean answer."]]),
  ] as const;

  const libraryRows = [
    ["Indexed files", (overview?.totalFiles ?? 0).toLocaleString()],
    ["Mods", (overview?.modsCount ?? 0).toLocaleString()],
    ["Tray", (overview?.trayCount ?? 0).toLocaleString()],
    ["Creators", (overview?.creatorCount ?? 0).toLocaleString()],
    ...(!calmDetails ? [["Bundles", (overview?.bundlesCount ?? 0).toLocaleString()]] : []),
  ] as const;

  const statusChips = [
    [overview?.scanNeedsRefresh ? "Scan needs refresh" : "Scan current", overview?.scanNeedsRefresh ? "warn" : "good"],
    [`${sourceCount}/3 folders ready`, sourceCount < 3 ? "warn" : "good"],
    [`${overview?.unsafeCount ?? 0} risky`, (overview?.unsafeCount ?? 0) > 0 ? "danger" : "good"],
    [`${totalWatchCount} watched`, (overview?.exactUpdateItems ?? 0) > 0 || (overview?.watchSetupItems ?? 0) > 0 ? "warn" : "neutral"],
  ] as const;

  return (
    <div className={`screen-shell home-hub-screen home-hub-view-${userView}`}>
      <div className="home-hub-shell">
        <m.header className="home-hub-topbar" {...stagedListItem(0)}>
          <div className="home-hub-intro">
            <span className="section-label">
              <Sparkles size={14} strokeWidth={2} />
              Home
            </span>
            <div>
              <h1>{greeting}</h1>
              <p className="workspace-toolbar-copy">
                {userView === "beginner"
                  ? "A calm, easy read of your mod setup before you dive into the game."
                  : userView === "power"
                    ? "Your personal landing page for library health, watch status, and setup truth."
                    : "A steady overview of system health without turning the page into a dashboard wall."}
              </p>
            </div>
          </div>

          <div className="home-hub-actions">
            <span className="ghost-chip"><Palette size={12} strokeWidth={2} />{activeTheme.label}</span>
            <span className="ghost-chip">{formatHomeTime(now)}</span>
            <m.button type="button" className="secondary-action" onClick={() => setCustomizing(true)} whileHover={hoverLift} whileTap={tapPress}>
              <SlidersHorizontal size={14} strokeWidth={2} />
              Customize Home
            </m.button>
            <m.button type="button" className="primary-action" onClick={() => void onScan()} disabled={!canScan || isScanning} whileHover={!canScan || isScanning ? undefined : hoverLift} whileTap={!canScan || isScanning ? undefined : tapPress}>
              <ScanSearch size={14} strokeWidth={2} />
              {isScanning ? "Scanning..." : "Scan"}
            </m.button>
          </div>
        </m.header>

        <m.section
          className={`home-hero home-hero-${heroState.tone}`}
          data-atmosphere={ambientHero ? "ambient" : "still"}
          {...stagedListItem(1)}
        >
          <div className="home-hero-surface" aria-hidden="true">
            <span className="home-hero-gridwash" />
            <span className="home-hero-ridge" />
            <m.span
              className="home-hero-aura home-hero-aura-a"
              initial={false}
              animate={
                ambientHero
                  ? {
                      x: [-10, 12, -8, -10],
                      y: [0, 10, -6, 0],
                      opacity: [0.18, 0.38, 0.24, 0.18],
                      scale: [1, 1.04, 0.98, 1],
                    }
                  : { x: 0, y: 0, opacity: 0.12, scale: 0.98 }
              }
              transition={
                ambientHero
                  ? { duration: 18, repeat: Number.POSITIVE_INFINITY, ease: "easeInOut" }
                  : { duration: 0.24, ease: "easeOut" }
              }
            />
            <m.span
              className="home-hero-aura home-hero-aura-b"
              initial={false}
              animate={
                ambientHero
                  ? {
                      x: [8, -12, 6, 8],
                      y: [0, -12, 8, 0],
                      opacity: [0.14, 0.3, 0.2, 0.14],
                      scale: [1, 0.96, 1.03, 1],
                    }
                  : { x: 0, y: 0, opacity: 0.1, scale: 0.96 }
              }
              transition={
                ambientHero
                  ? { duration: 22, repeat: Number.POSITIVE_INFINITY, ease: "easeInOut" }
                  : { duration: 0.24, ease: "easeOut" }
              }
            />
            <m.span
              className="home-hero-pulse"
              initial={false}
              animate={
                ambientHero
                  ? {
                      opacity: [0.16, 0.34, 0.16],
                      scale: [0.98, 1.05, 0.98],
                    }
                  : { opacity: 0, scale: 0.98 }
              }
              transition={
                ambientHero
                  ? { duration: 14, repeat: Number.POSITIVE_INFINITY, ease: "easeInOut" }
                  : { duration: 0.24, ease: "easeOut" }
              }
            />
            <m.span
              className="home-hero-trace"
              initial={false}
              animate={
                ambientHero
                  ? {
                      x: ["-6%", "10%", "-2%", "-6%"],
                      opacity: [0.14, 0.44, 0.18, 0.14],
                    }
                  : { x: "0%", opacity: 0 }
              }
              transition={
                ambientHero
                  ? { duration: 12, repeat: Number.POSITIVE_INFINITY, ease: "easeInOut" }
                  : { duration: 0.24, ease: "easeOut" }
              }
            />
          </div>

          <div className="home-hero-copy">
            <p className="eyebrow">{heroState.eyebrow}</p>
            <h2>{heroState.title}</h2>
            <p className="home-hero-summary">{heroState.summary}</p>
            <div className="health-chip-group home-hero-chip-row">
              {statusChips.slice(0, denseDetails ? statusChips.length : 3).map(([label, tone]) => (
                <span key={label} className={`health-chip ${tone === "good" ? "is-good" : tone === "warn" ? "is-warn" : tone === "danger" ? "is-danger" : ""}`}>
                  <span className="health-chip-dot"></span>
                  {label}
                </span>
              ))}
            </div>
            <p className="home-hero-footnote">{heroState.footnote}</p>
          </div>

          <div className="home-hero-metrics">
            {heroState.metrics.map((metric) => (
              <div key={metric.label} className="home-hero-metric">
                <span>{metric.label}</span>
                <strong>{metric.value}</strong>
                <small>{metric.note}</small>
              </div>
            ))}
          </div>
        </m.section>

        {userView === "beginner" &&
        (overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0) > 0 ? (
          <div
            className="casual-home-cta-card"
            role="button"
            tabIndex={0}
            onClick={() => {
              clearNudgeDismissed();
              onNavigate("downloads");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                clearNudgeDismissed();
                onNavigate("downloads");
              }
            }}
          >
            <div className="casual-home-cta-icon">
              <svg viewBox="0 0 24 24" fill="none" strokeWidth="1.8">
                <polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
                <path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
              </svg>
            </div>
            <div className="casual-home-cta-text">
              <p className="casual-home-cta-headline">
                You have{" "}
                {(overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0)}{" "}
                item
                {(overview?.waitingOnYouItems ?? overview?.needsReviewItems ?? 0) !== 1
                  ? "s"
                  : ""}{" "}
                waiting in Downloads
              </p>
              <p className="casual-home-cta-sub">
                Review them before they get added to your game
              </p>
            </div>
            <div className="casual-home-cta-arrow">
              <svg viewBox="0 0 24 24" fill="none" strokeWidth="2">
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </div>
          </div>
        ) : null}

        <div className="home-module-stack">
          {moduleBands.map((band, bandIndex) => (
            <div
              key={`band-${bandIndex + 1}`}
              className={`home-module-band home-module-band-${bandIndex + 1}`}
            >
              {band.map((moduleId) => {
                if (moduleId === "snapshot") {
                  return (
                    <HomeModuleCard
                      key={moduleId}
                      index={2}
                      moduleId={moduleId}
                      label="Today snapshot"
                      title={
                        userView === "beginner"
                          ? "What is waiting right now"
                          : "What is active right now"
                      }
                      icon={<Activity size={14} strokeWidth={2} />}
                    >
                      {snapshotRows.map(([label, value, note]) => (
                        <HomeGlanceRow
                          key={label}
                          label={label}
                          value={value}
                          note={calmDetails ? undefined : note}
                        />
                      ))}
                    </HomeModuleCard>
                  );
                }

                if (moduleId === "health") {
                  return (
                    <HomeModuleCard
                      key={moduleId}
                      index={3}
                      moduleId={moduleId}
                      label="System health"
                      title={
                        userView === "beginner"
                          ? "What SimSuite knows"
                          : "Current system truth"
                      }
                      icon={<ShieldCheck size={14} strokeWidth={2} />}
                    >
                      <div className="health-chip-group">
                        <span
                          className={`health-chip ${
                            overview?.scanNeedsRefresh ? "is-warn" : "is-good"
                          }`}
                        >
                          <span className="health-chip-dot"></span>
                          {overview?.scanNeedsRefresh ? "Refresh scan" : "Scan current"}
                        </span>
                        <span
                          className={`health-chip ${
                            (overview?.unsafeCount ?? 0) > 0 ? "is-danger" : "is-good"
                          }`}
                        >
                          <span className="health-chip-dot"></span>
                          {(overview?.unsafeCount ?? 0) > 0
                            ? "Needs care"
                            : "No active risks"}
                        </span>
                      </div>
                      <div className="home-fact-list">
                        {healthRows.map(([label, value]) => (
                          <HomeFactRow key={label} label={label} value={value} />
                        ))}
                      </div>
                    </HomeModuleCard>
                  );
                }

                if (moduleId === "watch") {
                  return (
                    <HomeModuleCard
                      key={moduleId}
                      index={4}
                      moduleId={moduleId}
                      label="Update watch"
                      title={
                        userView === "beginner"
                          ? "Tracked page follow-up"
                          : "Watched page summary"
                      }
                      icon={<RefreshCw size={14} strokeWidth={2} />}
                    >
                      {watchRows.map(([label, value, note]) => (
                        <HomeGlanceRow
                          key={label}
                          label={label}
                          value={value}
                          note={calmDetails ? undefined : note}
                        />
                      ))}
                    </HomeModuleCard>
                  );
                }

                if (moduleId === "folders") {
                  return (
                    <HomeModuleCard
                      key={moduleId}
                      index={5}
                      moduleId={moduleId}
                      label="Folders"
                      title={
                        sourceCount < 3
                          ? "Finish setup gently"
                          : "Library roots are linked"
                      }
                      icon={<FolderOpen size={14} strokeWidth={2} />}
                      badge={`${sourceCount}/3 ready`}
                      extraClass="home-folders-module"
                    >
                      <div className="home-folder-list">
                        <HomeFolderRow
                          label="Mods"
                          path={settings?.modsPath}
                          onChoose={() => void chooseFolder("modsPath")}
                          disabled={isSaving}
                        />
                        <HomeFolderRow
                          label="Tray"
                          path={settings?.trayPath}
                          onChoose={() => void chooseFolder("trayPath")}
                          disabled={isSaving}
                        />
                        <HomeFolderRow
                          label="Downloads"
                          path={settings?.downloadsPath}
                          onChoose={() => void chooseFolder("downloadsPath")}
                          disabled={isSaving}
                        />
                      </div>
                      <div className="home-folder-actions">
                        {sourceCount < 3 ? (
                          <button
                            type="button"
                            className="secondary-action"
                            onClick={() => void chooseFirstMissingFolder()}
                            disabled={isSaving}
                          >
                            <FolderOpen size={14} strokeWidth={2} />
                            Choose next folder
                          </button>
                        ) : null}
                        {hasDetectedPathSuggestion ? (
                          <button
                            type="button"
                            className="secondary-action"
                            onClick={() => void applyDetectedPaths()}
                            disabled={isSaving}
                          >
                            <FolderCog size={14} strokeWidth={2} />
                            Use detected folders
                          </button>
                        ) : null}
                      </div>
                      <p className="home-module-note">
                        {sourceCount < 3
                          ? "Once the missing folders are linked, scans and inbox checks can stay calm and reliable."
                          : "These roots are ready, so SimSuite can keep using one steady desktop flow."}
                      </p>
                    </HomeModuleCard>
                  );
                }

                return (
                  <HomeModuleCard
                    key={moduleId}
                    index={6}
                    moduleId={moduleId}
                    label="Library facts"
                    title={
                      userView === "power"
                        ? "Receipts at a glance"
                        : "The shape of the library"
                    }
                    icon={<LibraryBig size={14} strokeWidth={2} />}
                  >
                    <div className="summary-matrix home-library-matrix">
                      {libraryRows.map(([label, value]) => (
                        <div key={label} className="summary-stat home-library-stat">
                          <span>{label}</span>
                          <strong>{value}</strong>
                        </div>
                      ))}
                    </div>
                    <div className="home-fact-list">
                      <HomeFactRow
                        label="Last scan"
                        value={formatTimestamp(overview?.lastScanAt)}
                      />
                      <HomeFactRow label="Theme" value={activeTheme.label} />
                    </div>
                  </HomeModuleCard>
                );
              })}
            </div>
          ))}
        </div>
      </div>

      <AnimatePresence>
        {customizing ? (
          <m.div className="workbench-sheet-shell" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={overlayTransition} onClick={() => setCustomizing(false)}>
            <m.aside className="workbench-sheet home-customize-sheet" role="dialog" aria-modal="true" aria-labelledby="home-customize-title" initial={{ opacity: 0, x: 52 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 58 }} transition={panelSpring} onClick={(event) => event.stopPropagation()}>
              <div className="workbench-sheet-header">
                <div>
                  <p className="eyebrow">Customize Home</p>
                  <h2 id="home-customize-title">Make this page feel more like yours</h2>
                  <p className="workbench-sheet-copy">Pick the parts that matter most to you here. Your app view still decides how much detail SimSuite shows overall.</p>
                </div>
                <button type="button" className="workspace-toggle" onClick={() => setCustomizing(false)} aria-label="Close Customize Home">
                  <X size={14} strokeWidth={2} />
                </button>
              </div>

              <div className="workbench-sheet-body">
                <HomeCustomSection label="Hero focus" copy="Choose what the main panel puts front and center." icon={<Sparkles size={14} strokeWidth={2} />}>
                  <SegmentPicker<HomeHeroFocus> options={[["health", "Library health"], ["watch", "Update watch"], ["setup", "Folder setup"]]} value={displayPrefs.focus} onChange={(value) => updateDisplayPrefs({ focus: value })} />
                </HomeCustomSection>

                <HomeCustomSection label="Show on Home" copy="Keep only the parts you want to glance at here." icon={<SlidersHorizontal size={14} strokeWidth={2} />}>
                  <div className="home-toggle-grid">
                    {allowedModules.map((moduleId) => (
                      <m.button key={moduleId} type="button" className={`home-toggle-card ${displayPrefs.visibleModules[moduleId] ? "is-active" : ""}`} onClick={() => setModuleVisible(moduleId, !displayPrefs.visibleModules[moduleId])} whileHover={hoverLift} whileTap={tapPress}>
                        <span className="home-toggle-check" aria-hidden="true">{displayPrefs.visibleModules[moduleId] ? <Check size={12} strokeWidth={2.4} /> : null}</span>
                        <div className="home-toggle-copy">
                          <strong>{HOME_MODULE_LABELS[moduleId]}</strong>
                          <span>{describeHomeModule(moduleId)}</span>
                        </div>
                      </m.button>
                    ))}
                  </div>
                </HomeCustomSection>

                <HomeCustomSection label="Theme and spacing" copy="Personalize the color mood and how airy the page feels." icon={<Palette size={14} strokeWidth={2} />}>
                  <div className="theme-strip home-theme-strip">
                    {UI_THEMES.map((item) => (
                      <m.button key={item.id} type="button" className={`theme-chip ${theme === item.id ? "is-active" : ""}`} onClick={() => setTheme(item.id)} whileHover={hoverLift} whileTap={tapPress} title={`${item.label}: ${item.hint}`}>
                        <div className="theme-chip-swatches" aria-hidden="true">
                          {item.swatch.map((value) => <span key={value} className="theme-chip-swatch" style={{ background: value }} />)}
                        </div>
                        <div className="theme-chip-copy">
                          <strong>{item.label}</strong>
                          <span>{item.mood}</span>
                        </div>
                      </m.button>
                    ))}
                  </div>
                  <SegmentPicker options={HOME_DENSITY_OPTIONS.map((option) => [option.id, option.label])} value={density} onChange={setDensity} />
                </HomeCustomSection>

                <HomeCustomSection label="Atmosphere" copy="Still keeps the hero settled. Ambient adds a soft drift and light sweep." icon={<RefreshCw size={14} strokeWidth={2} />}>
                  <SegmentPicker options={[["still", "Still"], ["ambient", "Ambient"]]} value={displayPrefs.ambientMotion ? "ambient" : "still"} onChange={(value) => updateDisplayPrefs({ ambientMotion: value === "ambient" })} />
                </HomeCustomSection>
              </div>

              <div className="workbench-sheet-footer">
                <button type="button" className="secondary-action" onClick={() => setDisplayPrefs(defaultHomePrefs(userView))}>Reset Home</button>
                <button type="button" className="primary-action" onClick={() => setCustomizing(false)}>Done</button>
              </div>
            </m.aside>
          </m.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}

function HomeModuleCard({
  children,
  index,
  moduleId,
  label,
  title,
  icon,
  badge,
  extraClass,
}: {
  children: ReactNode;
  index: number;
  moduleId: HomeModuleId;
  label: string;
  title: string;
  icon: ReactNode;
  badge?: string;
  extraClass?: string;
}) {
  return (
    <m.section
      className={`panel-card home-module-card home-module-card-${moduleId}${
        extraClass ? ` ${extraClass}` : ""
      }`}
      {...stagedListItem(index)}
    >
      <div className="panel-heading home-module-heading">
        <div>
          <span className="section-label">{icon}{label}</span>
          <h2>{title}</h2>
        </div>
        {badge ? <span className="ghost-chip">{badge}</span> : null}
      </div>
      {children}
    </m.section>
  );
}

function HomeCustomSection({ children, label, copy, icon }: { children: ReactNode; label: string; copy: string; icon: ReactNode }) {
  return (
    <section className="home-custom-section">
      <div className="home-custom-section-heading">
        <span className="section-label">{icon}{label}</span>
        <p className="workspace-toolbar-copy">{copy}</p>
      </div>
      {children}
    </section>
  );
}

function SegmentPicker<T extends string>({ options, value, onChange }: { options: Array<readonly [T, string]>; value: T; onChange: (value: T) => void }) {
  return (
    <div className="segmented-control" role="tablist">
      {options.map(([id, label]) => (
        <m.button key={id} type="button" className={`segment-button ${value === id ? "is-active" : ""}`} onClick={() => onChange(id)} whileHover={hoverLift} whileTap={tapPress}>
          {label}
        </m.button>
      ))}
    </div>
  );
}

function HomeGlanceRow({ label, value, note }: { label: string; value: string; note?: string }) {
  return (
    <div className="home-glance-row">
      <div className="home-glance-copy">
        <strong>{label}</strong>
        {note ? <span>{note}</span> : null}
      </div>
      <span className="ghost-chip home-value-chip">{value}</span>
    </div>
  );
}

function HomeFactRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="home-fact-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function HomeFolderRow({ label, path, onChoose, disabled }: { label: string; path: string | null | undefined; onChoose: () => void; disabled: boolean }) {
  return (
    <div className={`home-folder-row ${path ? "is-ready" : "is-empty"}`}>
      <div className="home-folder-copy">
        <strong>{label}</strong>
        <span className="text-path">{path || "Not chosen yet"}</span>
      </div>
      <button type="button" className="secondary-action folder-action" onClick={onChoose} disabled={disabled}>
        <FolderOpen size={12} strokeWidth={2} />
        Choose
      </button>
    </div>
  );
}

function buildModuleBands(
  userView: UserView,
  visibleModules: HomeModuleId[],
) {
  if (userView === "beginner") {
    return [visibleModules];
  }

  const preferredBands =
    userView === "power"
      ? [
          ["watch", "health"],
          ["snapshot", "folders"],
          ["library"],
        ]
      : [
          ["snapshot", "health"],
          ["watch", "folders"],
          ["library"],
        ];

  const remaining = new Set(visibleModules);
  const bands = preferredBands
    .map((band) => band.filter((moduleId) => remaining.has(moduleId as HomeModuleId)))
    .filter((band) => band.length > 0)
    .map((band) => {
      band.forEach((moduleId) => remaining.delete(moduleId as HomeModuleId));
      return band as HomeModuleId[];
    });

  if (remaining.size > 0) {
    bands.push(Array.from(remaining));
  }

  return bands;
}
