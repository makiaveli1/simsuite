import { useEffect, useState } from "react";
import { m } from "motion/react";
import {
  AlertTriangle,
  Check,
  CheckSquare,
  ChevronDown,
  Folder,
  LayoutPanelLeft,
  LoaderCircle,
  Palette,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
  Square,
  Trash2,
  Workflow,
} from "lucide-react";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import {
  EXPERIENCE_MODE_ORDER,
  EXPERIENCE_MODE_PROFILES,
} from "../lib/experienceMode";
import { hoverLift, stagedListItem, tapPress } from "../lib/motion";
import { UI_THEMES, getThemeDefinition } from "../lib/themeMeta";
import type {
  AppBehaviorSettings,
  ExperienceMode,
  LibrarySettings,
  StagingAreasSummary,
  UiDensity,
  WatchRefreshSummary,
} from "../lib/types";
import { screenHelperLine } from "../lib/uiLanguage";

const EXPERIENCE_CARDS: Record<
  ExperienceMode,
  {
    headline: string;
    hint: string;
    traits: string[];
  }
> = {
  casual: {
    headline: "Simple, steady, and easy to follow",
    hint: "Keeps the app calmer, pushes the safest next move up front, and tucks heavier tools behind one More button.",
    traits: [
      "Home, Inbox, and Tidy Up stay front and center",
      "The app keeps the chatter lighter",
      "Great for everyday play sessions",
    ],
  },
  seasoned: {
    headline: "Balanced enough for regular mod wrangling",
    hint: "Keeps the workbench feel, leaves useful proof open, and still makes the next move easy to spot.",
    traits: [
      "Full rail stays visible",
      "Guidance and proof stay in balance",
      "Best for regular cleanup and updates",
    ],
  },
  creator: {
    headline: "Dense, tool-forward, and ready for receipts",
    hint: "Shows more system state, keeps heavier tools closer, and opens the deeper details you need for big cleanup or author work.",
    traits: [
      "Audit tools move higher in the flow",
      "More evidence stays open by default",
      "Best for creators and deep CC marathons",
    ],
  },
};

const DENSITIES: Array<{
  id: UiDensity;
  label: string;
  hint: string;
}> = [
  {
    id: "compact",
    label: "Snug",
    hint: "Fits more rows and panels on screen.",
  },
  {
    id: "balanced",
    label: "Normal",
    hint: "Best default balance for most desktop monitors.",
  },
  {
    id: "roomy",
    label: "Roomy",
    hint: "Adds more breathing room between rows and controls.",
  },
];

type SettingsSectionId =
  | "experience"
  | "appearance"
  | "density"
  | "automation"
  | "layout";

interface SettingsScreenProps {
  experienceMode: ExperienceMode;
  onExperienceModeChange: (view: ExperienceMode) => void;
}

export function SettingsScreen({
  experienceMode,
  onExperienceModeChange,
}: SettingsScreenProps) {
  const { theme, density, setTheme, setDensity, resetPanelSizes } =
    useUiPreferences();
  const [appBehavior, setAppBehavior] = useState<AppBehaviorSettings | null>(null);
  const [librarySettings, setLibrarySettings] = useState<LibrarySettings | null>(null);
  const [isSavingBackgroundMode, setIsSavingBackgroundMode] = useState(false);
  const [backgroundModeError, setBackgroundModeError] = useState<string | null>(null);
  const [isRefreshingWatchedSources, setIsRefreshingWatchedSources] = useState(false);
  const [watchAutomationMessage, setWatchAutomationMessage] = useState<string | null>(null);
  const [activeSection, setActiveSection] =
    useState<SettingsSectionId>("experience");
  const activeTheme = getThemeDefinition(theme);
  const activeView = {
    ...EXPERIENCE_MODE_PROFILES[experienceMode],
    ...EXPERIENCE_CARDS[experienceMode],
  };
  const activeDensity =
    DENSITIES.find((item) => item.id === density) ?? DENSITIES[1];
  const keepRunningInBackground = appBehavior?.keepRunningInBackground ?? false;
  const automaticWatchChecks = appBehavior?.automaticWatchChecks ?? false;
  const watchCheckIntervalHours = appBehavior?.watchCheckIntervalHours ?? 12;
  const settingsSections = [
    {
      id: "experience" as const,
      label: "Experience",
      title: "How the app talks to you",
      summary: `${activeView.label} mode`,
      hint: "Choose how much help and proof stays open while you sort.",
      icon: Sparkles,
    },
    {
      id: "appearance" as const,
      label: "Appearance",
      title: "Color and mood",
      summary: activeTheme.label,
      hint: "Pick the skin that makes long cleanup sessions easier on your eyes.",
      icon: Palette,
    },
    {
      id: "density" as const,
      label: "Workspace size",
      title: "How tight the layout feels",
      summary: activeDensity.label,
      hint: "Change how much fits on screen before anything feels cramped.",
      icon: LayoutPanelLeft,
    },
    {
      id: "automation" as const,
      label: "Background and updates",
      title: "What keeps running quietly",
      summary: automaticWatchChecks
        ? `Checks ${watchIntervalLabel(watchCheckIntervalHours)}`
        : keepRunningInBackground
          ? "Tray stays awake"
          : "Manual only",
      hint: "Control tray behavior and safe watched-page checks.",
      icon: Workflow,
    },
    {
      id: "layout" as const,
      label: "Layout memory",
      title: "Reset saved panel sizes",
      summary: "Restore defaults",
      hint: "Use this if the workspace starts feeling over-tuned or awkward.",
      icon: RotateCcw,
    },
  ];
  const activeSectionMeta =
    settingsSections.find((section) => section.id === activeSection) ??
    settingsSections[0];

  useEffect(() => {
    let cancelled = false;

    void api
      .getAppBehaviorSettings()
      .then((settings) => {
        if (!cancelled) {
          setAppBehavior(settings);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setBackgroundModeError(toErrorMessage(error));
        }
      });

    void api
      .getLibrarySettings()
      .then((settings) => {
        if (!cancelled) {
          setLibrarySettings(settings);
        }
      })
      .catch(() => {
        // Library settings are not critical; fail silently
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function saveBehaviorSettings(nextValues: Partial<AppBehaviorSettings>) {
    if (!appBehavior) {
      return;
    }

    setIsSavingBackgroundMode(true);
    setBackgroundModeError(null);

    try {
      const nextSettings = await api.saveAppBehaviorSettings({
        ...appBehavior,
        ...nextValues,
      });
      setAppBehavior(nextSettings);
    } catch (error) {
      setBackgroundModeError(toErrorMessage(error));
    } finally {
      setIsSavingBackgroundMode(false);
    }
  }

  async function updateBackgroundMode(keepRunning: boolean) {
    if (appBehavior?.keepRunningInBackground === keepRunning) {
      return;
    }

    await saveBehaviorSettings({
      keepRunningInBackground: keepRunning,
    });
  }

  async function updateAutomaticWatchChecks(enabled: boolean) {
    if (appBehavior?.automaticWatchChecks === enabled) {
      return;
    }

    await saveBehaviorSettings({
      automaticWatchChecks: enabled,
    });
  }

  async function updateWatchCheckInterval(hours: number) {
    if (appBehavior?.watchCheckIntervalHours === hours) {
      return;
    }

    await saveBehaviorSettings({
      watchCheckIntervalHours: hours,
    });
  }

  async function updateIgnorePatterns(patterns: string[]) {
    if (
      JSON.stringify(appBehavior?.downloadIgnorePatterns ?? []) ===
      JSON.stringify(patterns)
    ) {
      return;
    }

    await saveBehaviorSettings({
      downloadIgnorePatterns: patterns,
    });
  }

  async function updateSpecialModAlerts(silent: boolean | null) {
    if (appBehavior?.silentSpecialModUpdates === silent) {
      return;
    }

    await saveBehaviorSettings({
      silentSpecialModUpdates: silent,
    });
  }

  async function updateRejectFolder(path: string | null) {
    if (!librarySettings) {
      return;
    }
    await api.saveLibraryPaths({
      ...librarySettings,
      downloadRejectFolder: path,
    });
    setLibrarySettings((current) =>
      current ? { ...current, downloadRejectFolder: path } : null,
    );
  }

  async function pickRejectFolder() {
    const picked = await api.pickFolder("Choose a quick-reject folder");
    if (picked) {
      await updateRejectFolder(picked);
    }
  }

  async function refreshWatchedSources() {
    setIsRefreshingWatchedSources(true);
    setWatchAutomationMessage(null);
    setBackgroundModeError(null);

    try {
      const summary = await api.refreshWatchedSources();
      setAppBehavior((current) =>
        current
          ? {
              ...current,
              lastWatchCheckAt: summary.checkedAt,
              lastWatchCheckError: null,
            }
          : current,
      );
      setWatchAutomationMessage(formatWatchRefreshMessage(summary));
    } catch (error) {
      const message = toErrorMessage(error);
      setBackgroundModeError(message);
      setAppBehavior((current) =>
        current
          ? {
              ...current,
              lastWatchCheckError: message,
            }
          : current,
      );
    } finally {
      setIsRefreshingWatchedSources(false);
    }
  }

  return (
    <div className="screen-shell settings-screen">
      <header className="screen-header-row">
        <div className="screen-title-group">
          <span className="section-label">
            <SlidersHorizontal size={14} strokeWidth={2} />
            Personal setup
          </span>
          <div>
            <h1>Settings</h1>
            <p className="workspace-toolbar-copy">
              {screenHelperLine("settings", experienceMode)}
            </p>
          </div>
        </div>

        <div className="workspace-toolbar-chip-group settings-header-chips">
          <span className="workspace-status-chip">{activeView.label} mode</span>
          <span className="workspace-status-chip">{activeDensity.label} size</span>
          <span className="workspace-status-chip">{activeTheme.label}</span>
        </div>
      </header>

      <div className="settings-layout settings-preferences-layout">
        <aside className="settings-nav-column">
          <m.section className="panel-card settings-nav-panel" {...stagedListItem(0)}>
            <div className="panel-heading">
              <div>
                <span className="section-label">
                  <SlidersHorizontal size={14} strokeWidth={2} />
                  Preferences
                </span>
                <h2>Pick one section at a time</h2>
              </div>
              <p className="workspace-toolbar-copy">
                Only the group you are changing stays open, so the screen feels more like
                a proper desktop preferences window.
              </p>
            </div>

            <div className="settings-section-list" aria-label="Settings sections">
              {settingsSections.map((section) => {
                const Icon = section.icon;
                return (
                  <m.button
                    key={section.id}
                    type="button"
                    className={`settings-section-button ${
                      activeSection === section.id ? "is-active" : ""
                    }`}
                    onClick={() => setActiveSection(section.id)}
                    whileHover={hoverLift}
                    whileTap={tapPress}
                    aria-pressed={activeSection === section.id}
                  >
                    <div className="settings-section-button-topline">
                      <span className="section-label">
                        <Icon size={14} strokeWidth={2} />
                        {section.label}
                      </span>
                      <span className="ghost-chip">{section.summary}</span>
                    </div>
                    <strong>{section.title}</strong>
                    <p className="workspace-toolbar-copy">{section.hint}</p>
                  </m.button>
                );
              })}
            </div>
          </m.section>

          <m.section className="panel-card settings-side-panel" {...stagedListItem(1)}>
            <div className="panel-heading">
              <div>
                <span className="section-label">Saved here</span>
                <h2>Applies right away</h2>
              </div>
            </div>

            <div className="summary-matrix">
              <div className="summary-stat">
                <span>Mode</span>
                <strong>{activeView.label}</strong>
              </div>
              <div className="summary-stat">
                <span>Skin</span>
                <strong>{activeTheme.label}</strong>
              </div>
              <div className="summary-stat">
                <span>Size</span>
                <strong>{activeDensity.label}</strong>
              </div>
            </div>

            <div className="settings-note-list">
              <div className="settings-note">
                Your choices save on this PC right away, so the app feels familiar next
                time too.
              </div>
              <div className="settings-note">
                These options only change the feel of the app. They do not move files or
                weaken the safety flow.
              </div>
              <div className="settings-note">
                Current focus: <strong>{activeSectionMeta.label}</strong>.
              </div>
            </div>
          </m.section>
        </aside>

        <div className="settings-detail-column">
          <m.section
            key={activeSection}
            className="panel-card settings-focus-panel"
            {...stagedListItem(2)}
          >
            {activeSection === "experience" ? (
              <SettingsExperienceSection
                experienceMode={experienceMode}
                activeView={activeView}
                onExperienceModeChange={onExperienceModeChange}
              />
            ) : null}

            {activeSection === "appearance" ? (
              <SettingsAppearanceSection
                activeTheme={activeTheme}
                theme={theme}
                onThemeChange={setTheme}
              />
            ) : null}

            {activeSection === "density" ? (
              <SettingsDensitySection
                density={density}
                activeDensity={activeDensity}
                onDensityChange={setDensity}
              />
            ) : null}

            {activeSection === "automation" ? (
              <SettingsAutomationSection
                appBehavior={appBehavior}
                librarySettings={librarySettings}
                keepRunningInBackground={keepRunningInBackground}
                automaticWatchChecks={automaticWatchChecks}
                watchCheckIntervalHours={watchCheckIntervalHours}
                isSavingBackgroundMode={isSavingBackgroundMode}
                isRefreshingWatchedSources={isRefreshingWatchedSources}
                backgroundModeError={backgroundModeError}
                watchAutomationMessage={watchAutomationMessage}
                onUpdateBackgroundMode={updateBackgroundMode}
                onUpdateAutomaticWatchChecks={updateAutomaticWatchChecks}
                onUpdateWatchCheckInterval={updateWatchCheckInterval}
                onRefreshWatchedSources={refreshWatchedSources}
                onUpdateIgnorePatterns={updateIgnorePatterns}
                onUpdateRejectFolder={updateRejectFolder}
                onPickRejectFolder={pickRejectFolder}
                onUpdateSpecialModAlerts={updateSpecialModAlerts}
              />
            ) : null}

            {activeSection === "layout" ? (
              <SettingsLayoutSection onResetPanelSizes={resetPanelSizes} />
            ) : null}
          </m.section>
        </div>
      </div>

      <StagingAreaBrowserSection />
    </div>
  </div>
  );
}

function SettingsExperienceSection({
  experienceMode,
  activeView,
  onExperienceModeChange,
}: {
  experienceMode: ExperienceMode;
  activeView: {
    label: string;
    hint: string;
    workspaceSummary: string;
  };
  onExperienceModeChange: (view: ExperienceMode) => void;
}) {
  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Sparkles size={14} strokeWidth={2} />
            Experience
          </span>
          <h2>Pick your household vibe</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Each view changes how loud or quiet the app feels while you sort.
        </p>
      </div>

      <div className="settings-view-grid" role="tablist" aria-label="User view">
        {EXPERIENCE_MODE_ORDER.map((mode) => {
          const profile = EXPERIENCE_MODE_PROFILES[mode];
          const card = EXPERIENCE_CARDS[mode];
          return (
            <m.button
              key={mode}
              type="button"
              className={`settings-view-card ${experienceMode === mode ? "is-active" : ""}`}
              onClick={() => onExperienceModeChange(mode)}
              title={card.hint}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              <div className="settings-view-card-topline">
                <strong>{profile.label}</strong>
                <span className="ghost-chip">{profile.badge}</span>
              </div>
              <span className="settings-view-headline">{card.headline}</span>
              <p className="workspace-toolbar-copy">{card.hint}</p>
              <div className="settings-view-traits">
                {card.traits.map((trait) => (
                  <span key={trait} className="settings-view-trait">
                    {trait}
                  </span>
                ))}
              </div>
            </m.button>
          );
        })}
      </div>

      <div className="settings-summary-card">
        <span className="section-label">Current fit</span>
        <strong>{activeView.label} mode</strong>
        <p className="workspace-toolbar-copy">{activeView.hint}</p>
        <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
          {activeView.workspaceSummary}
        </p>
      </div>
    </>
  );
}

function SettingsAppearanceSection({
  activeTheme,
  theme,
  onThemeChange,
}: {
  activeTheme: ReturnType<typeof getThemeDefinition>;
  theme: ReturnType<typeof getThemeDefinition>["id"];
  onThemeChange: (themeId: ReturnType<typeof getThemeDefinition>["id"]) => void;
}) {
  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Palette size={14} strokeWidth={2} />
            Appearance
          </span>
          <h2>Pick a skin</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Themes change color, surface tone, motion feel, and backdrop mood.
        </p>
      </div>

      <div className="theme-strip settings-theme-strip">
        {UI_THEMES.map((item) => (
          <m.button
            key={item.id}
            type="button"
            className={`theme-chip ${theme === item.id ? "is-active" : ""}`}
            onClick={() => onThemeChange(item.id)}
            title={`${item.label}: ${item.hint}`}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            <div className="theme-chip-swatches" aria-hidden="true">
              {item.swatch.map((value) => (
                <span
                  key={value}
                  className="theme-chip-swatch"
                  style={{ background: value }}
                />
              ))}
            </div>
            <div className="theme-chip-copy">
              <strong>{item.label}</strong>
              <span>{item.mood}</span>
            </div>
          </m.button>
        ))}
      </div>

      <div className="settings-theme-summary">
        <div className="workspace-toolbar-summary-swatch" aria-hidden="true">
          {activeTheme.swatch.map((value) => (
            <span
              key={value}
              className="theme-chip-swatch"
              style={{ background: value }}
            />
          ))}
        </div>
        <div className="settings-theme-copy">
          <strong>{activeTheme.label}</strong>
          <p className="workspace-toolbar-copy">{activeTheme.hint}</p>
          <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
            Signature: {activeTheme.signature}
          </p>
        </div>
      </div>
    </>
  );
}

function SettingsDensitySection({
  density,
  activeDensity,
  onDensityChange,
}: {
  density: UiDensity;
  activeDensity: (typeof DENSITIES)[number];
  onDensityChange: (density: UiDensity) => void;
}) {
  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <LayoutPanelLeft size={14} strokeWidth={2} />
            Workspace size
          </span>
          <h2>Choose panel density</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Density changes row spacing, panel padding, and how tightly the app packs
          information.
        </p>
      </div>

      <div className="segmented-control" role="tablist" aria-label="Density">
        {DENSITIES.map((item) => (
          <m.button
            key={item.id}
            type="button"
            className={`segment-button ${density === item.id ? "is-active" : ""}`}
            onClick={() => onDensityChange(item.id)}
            title={item.hint}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            {item.label}
          </m.button>
        ))}
      </div>

      <div className="settings-summary-card">
        <span className="section-label">Current spacing</span>
        <strong>{activeDensity.label}</strong>
        <p className="workspace-toolbar-copy">{activeDensity.hint}</p>
      </div>
    </>
  );
}

function SettingsAutomationSection({
  appBehavior,
  librarySettings,
  keepRunningInBackground,
  automaticWatchChecks,
  watchCheckIntervalHours,
  isSavingBackgroundMode,
  isRefreshingWatchedSources,
  backgroundModeError,
  watchAutomationMessage,
  onUpdateBackgroundMode,
  onUpdateAutomaticWatchChecks,
  onUpdateWatchCheckInterval,
  onRefreshWatchedSources,
  onUpdateIgnorePatterns,
  onUpdateRejectFolder,
  onPickRejectFolder,
  onUpdateSpecialModAlerts,
}: {
  appBehavior: AppBehaviorSettings | null;
  librarySettings: LibrarySettings | null;
  keepRunningInBackground: boolean;
  automaticWatchChecks: boolean;
  watchCheckIntervalHours: number;
  isSavingBackgroundMode: boolean;
  isRefreshingWatchedSources: boolean;
  backgroundModeError: string | null;
  watchAutomationMessage: string | null;
  onUpdateBackgroundMode: (keepRunning: boolean) => Promise<void>;
  onUpdateAutomaticWatchChecks: (enabled: boolean) => Promise<void>;
  onUpdateWatchCheckInterval: (hours: number) => Promise<void>;
  onRefreshWatchedSources: () => Promise<void>;
  onUpdateIgnorePatterns: (patterns: string[]) => Promise<void>;
  onUpdateRejectFolder: (path: string | null) => Promise<void>;
  onPickRejectFolder: () => Promise<void>;
  onUpdateSpecialModAlerts: (silent: boolean | null) => Promise<void>;
}) {
  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Workflow size={14} strokeWidth={2} />
            Background and updates
          </span>
          <h2>Keep watching after the window closes</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Use the tray if you want SimSuite to keep watching Downloads and safely
          checking saved mod pages while the main window is hidden.
        </p>
      </div>

      <div className="settings-focus-grid">
        <div className="settings-summary-card settings-focus-block">
          <span className="section-label">Current close action</span>
          <strong>
            {!appBehavior
              ? "Loading close behavior"
              : keepRunningInBackground
                ? "Hide to tray"
                : "Close app"}
          </strong>
          <p className="workspace-toolbar-copy">
            {!appBehavior
              ? "Reading your saved preference."
              : keepRunningInBackground
                ? "Closing the main window keeps SimSuite running in the tray so the Downloads watcher can stay awake."
                : "Closing the main window exits SimSuite completely, so Downloads watching stops until you open the app again."}
          </p>

          <div className="segmented-control" role="tablist" aria-label="Close behavior">
            <m.button
              type="button"
              className={`segment-button ${
                appBehavior && !keepRunningInBackground ? "is-active" : ""
              }`}
              onClick={() => void onUpdateBackgroundMode(false)}
              disabled={!appBehavior || isSavingBackgroundMode}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              Close app
            </m.button>
            <m.button
              type="button"
              className={`segment-button ${keepRunningInBackground ? "is-active" : ""}`}
              onClick={() => void onUpdateBackgroundMode(true)}
              disabled={!appBehavior || isSavingBackgroundMode}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              Hide to tray
            </m.button>
          </div>

          <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
            {isSavingBackgroundMode ? (
              <span className="settings-inline-status">
                <LoaderCircle size={14} strokeWidth={2} className="spin" />
                Saving close behavior...
              </span>
            ) : keepRunningInBackground ? (
              "The tray menu gives you Open SimSuite and Exit SimSuite."
            ) : (
              "Best if you only want SimSuite running while the window is open."
            )}
          </p>
        </div>

        <div className="settings-summary-card settings-focus-block">
          <span className="section-label">Automatic update checks</span>
          <strong>{automaticWatchChecks ? "On" : "Off"}</strong>
          <p className="workspace-toolbar-copy">
            SimSuite only checks saved pages that are safe to read directly. It does not
            try to break through protected sites, logins, or anti-bot walls.
          </p>

          <div
            className="segmented-control"
            role="tablist"
            aria-label="Automatic update checks"
          >
            <m.button
              type="button"
              className={`segment-button ${!automaticWatchChecks ? "is-active" : ""}`}
              onClick={() => void onUpdateAutomaticWatchChecks(false)}
              disabled={!appBehavior || isSavingBackgroundMode}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              Off
            </m.button>
            <m.button
              type="button"
              className={`segment-button ${automaticWatchChecks ? "is-active" : ""}`}
              onClick={() => void onUpdateAutomaticWatchChecks(true)}
              disabled={!appBehavior || isSavingBackgroundMode}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              On
            </m.button>
          </div>

          <label className="field">
            <span>How often</span>
            <select
              value={String(watchCheckIntervalHours)}
              onChange={(event) =>
                void onUpdateWatchCheckInterval(Number(event.target.value))
              }
              disabled={!appBehavior || isSavingBackgroundMode}
            >
              <option value="1">Every hour</option>
              <option value="6">Every 6 hours</option>
              <option value="12">Every 12 hours</option>
              <option value="24">Every day</option>
            </select>
          </label>

          <div className="settings-summary-card settings-focus-block">
            <span className="section-label">Special mod update alerts</span>
            <p className="workspace-toolbar-copy">
              When SimSuite finds an update for MCCC or another tracked special mod, decide
              how to be notified.
            </p>
            <div
              className="segmented-control"
              role="tablist"
              aria-label="Special mod update alerts"
            >
              <m.button
                type="button"
                className={`segment-button ${
                  appBehavior?.silentSpecialModUpdates === null ? "is-active" : ""
                }`}
                onClick={() =>
                  void onUpdateSpecialModAlerts(null)
                }
                disabled={!appBehavior || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Ask
              </m.button>
              <m.button
                type="button"
                className={`segment-button ${
                  appBehavior?.silentSpecialModUpdates === true ? "is-active" : ""
                }`}
                onClick={() =>
                  void onUpdateSpecialModAlerts(true)
                }
                disabled={!appBehavior || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Silent
              </m.button>
              <m.button
                type="button"
                className={`segment-button ${
                  appBehavior?.silentSpecialModUpdates === false ? "is-active" : ""
                }`}
                onClick={() =>
                  void onUpdateSpecialModAlerts(false)
                }
                disabled={!appBehavior || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Notify
              </m.button>
            </div>
            <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
              {appBehavior?.silentSpecialModUpdates === null &&
                "When an update is found, SimSuite will ask before applying."}
              {appBehavior?.silentSpecialModUpdates === true &&
                "Special mod updates are checked but hidden from counts and tray."}
              {appBehavior?.silentSpecialModUpdates === false &&
                "You'll be notified through counts and tray when updates are found."}
            </p>
          </div>

          <div className="settings-action-row">
            <m.button
              type="button"
              className="secondary-action"
              onClick={() => void onRefreshWatchedSources()}
              disabled={isRefreshingWatchedSources}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              {isRefreshingWatchedSources
                ? "Checking watched pages..."
                : "Check watched pages now"}
            </m.button>
            <p className="workspace-toolbar-copy settings-inline-note">
              {automaticWatchChecks
                ? `Automatic checks are set to ${watchIntervalLabel(
                    watchCheckIntervalHours,
                  )}.`
                : "While this is off, SimSuite only checks watched pages when you ask it to."}
            </p>
          </div>

          <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
            Last watch check:{" "}
            {appBehavior?.lastWatchCheckAt
              ? new Date(appBehavior.lastWatchCheckAt).toLocaleString()
              : "Not checked yet"}
          </p>
          {watchAutomationMessage ? (
            <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
              {watchAutomationMessage}
            </p>
          ) : null}
          <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
            Exact mod pages are best. Creator pages stay reminder links until a better
            provider path exists.
          </p>
        </div>
      </div>

      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Workflow size={14} strokeWidth={2} />
            Quick reject folder
          </span>
          <h2>Auto-ignore a Downloads subfolder</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Anything in the chosen folder is automatically ignored when it appears in your
          Inbox. Useful for non-Sims downloads that land in your Downloads folder.
        </p>
      </div>

      <div className="settings-summary-card settings-focus-block">
        <span className="section-label">Reject folder</span>
        {librarySettings?.downloadRejectFolder ? (
          <>
            <div className="path-display-card">
              <code className="settings-folder-path">
                {librarySettings.downloadRejectFolder}
              </code>
            </div>
            <div className="settings-action-row">
              <m.button
                type="button"
                className="secondary-action"
                onClick={() => void onPickRejectFolder()}
                disabled={!librarySettings || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Change folder
              </m.button>
              <m.button
                type="button"
                className="ghost-chip"
                onClick={() => void onUpdateRejectFolder(null)}
                disabled={!librarySettings || isSavingBackgroundMode}
              >
                Remove
              </m.button>
            </div>
          </>
        ) : (
          <div className="settings-action-row">
            <m.button
              type="button"
              className="secondary-action"
              onClick={() => void onPickRejectFolder()}
              disabled={!librarySettings || isSavingBackgroundMode}
              whileHover={hoverLift}
              whileTap={tapPress}
            >
              Browse...
            </m.button>
            <span className="workspace-toolbar-copy workspace-toolbar-copy-muted">
              No folder selected. Set one to auto-reject non-Sims content.
            </span>
          </div>
        )}
      </div>

      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Workflow size={14} strokeWidth={2} />
            Auto-ignore patterns
          </span>
          <h2>Skip files matching patterns</h2>
        </div>
        <p className="workspace-toolbar-copy">
          SimSuite can auto-ignore new Downloads files whose filenames contain any of these patterns. Enter one pattern per line. Patterns are case-insensitive substrings.
        </p>
      </div>

      <IgnorePatternsEditor
        patterns={appBehavior?.downloadIgnorePatterns ?? []}
        onSave={onUpdateIgnorePatterns}
        disabled={!appBehavior || isSavingBackgroundMode}
      />

      {backgroundModeError ? (
        <div className="settings-status-banner">{backgroundModeError}</div>
      ) : null}
    </>
  );
}

function StagingAreaBrowserSection() {
  const [summary, setSummary] = useState<StagingAreasSummary | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isReloading, setIsReloading] = useState(false);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [expandedAreas, setExpandedAreas] = useState<Set<string>>(new Set());
  const [confirmingPaths, setConfirmingPaths] = useState<string[] | null>(null);
  const [isCleaning, setIsCleaning] = useState(false);
  const [cleanupResult, setCleanupResult] = useState<{
    deletedCount: number;
    freedBytes: number;
    errors: string[];
  } | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    void loadAreas();
  }, []);

  async function loadAreas() {
    setIsLoading(true);
    setLoadError(null);
    try {
      const data = await api.getStagingAreas();
      setSummary(data);
      // Auto-expand all areas by default
      setExpandedAreas(new Set(data.areas.map((a) => a.itemId)));
    } catch (err) {
      setLoadError(
        `Could not load staging areas: ${
          err instanceof Error ? err.message : String(err)
        }`,
      );
    } finally {
      setIsLoading(false);
    }
  }

  async function handleClean(paths: string[]) {
    setIsCleaning(true);
    setConfirmingPaths(null);
    try {
      const result = await api.cleanupStagingAreas(paths);
      setCleanupResult(result);
      // Deselect cleaned paths and reload
      setSelectedPaths((prev) => {
        const next = new Set(prev);
        paths.forEach((p) => next.delete(p));
        return next;
      });
      await loadAreas();
    } catch (err) {
      setCleanupResult({
        deletedCount: 0,
        freedBytes: 0,
        errors: [err instanceof Error ? err.message : String(err)],
      });
    } finally {
      setIsCleaning(false);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }

  function formatAge(isoDate: string | null): string {
    if (!isoDate) return "Unknown";
    const ms = Date.now() - new Date(isoDate).getTime();
    const mins = Math.floor(ms / 60000);
    if (mins < 1) return "Just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  const selectedArray = Array.from(selectedPaths);
  const totalSelectedBytes = summary?.areas
    .flatMap((a) => a.subdirectories)
    .filter((s) => selectedPaths.has(s.path))
    .reduce((sum, s) => sum + s.totalBytes, 0) ?? 0;

  const allSubdirPaths = summary?.areas.flatMap((a) => a.subdirectories.map((s) => s.path)) ?? [];
  const allSelected = allSubdirPaths.length > 0 && allSubdirPaths.every((p) => selectedPaths.has(p));

  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <Folder size={14} strokeWidth={2} />
            Inbox staging area
          </span>
          <h2>Review and clean your Downloads inbox cache</h2>
        </div>
        <p className="workspace-toolbar-copy">
          When SimSuite processes incoming Downloads, it stages files here before
          moving them to Mods or Tray. This shows what&apos;s accumulated and lets
          you reclaim disk space.
        </p>
      </div>

      <div className="settings-summary-card settings-focus-block">
        {/* Header row */}
        <div className="staging-header-row">
          <div className="staging-summary-stats">
            {isLoading ? (
              <span className="staging-loading-label">
                <LoaderCircle size={13} strokeWidth={2} className="spin" />
                Loading...
              </span>
            ) : summary ? (
              <>
                <span className="staging-stat">
                  <strong>{summary.totalFileCount.toLocaleString()}</strong>
                  <span className="workspace-toolbar-copy-muted"> files</span>
                </span>
                <span className="staging-stat-sep">·</span>
                <span className="staging-stat">
                  <strong>{formatBytes(summary.totalBytes)}</strong>
                  <span className="workspace-toolbar-copy-muted"> total</span>
                </span>
                <span className="staging-stat-sep">·</span>
                <span className="staging-stat">
                  <strong>{summary.areas.length}</strong>
                  <span className="workspace-toolbar-copy-muted">
                    {" "}
                    area{summary.areas.length !== 1 ? "s" : ""}
                  </span>
                </span>
              </>
            ) : (
              <span className="workspace-toolbar-copy-muted">—</span>
            )}
          </div>

          <div className="staging-header-actions">
            {selectedArray.length > 0 && (
              <span className="staging-selection-summary workspace-toolbar-copy-muted">
                {selectedArray.length} selected ({formatBytes(totalSelectedBytes)})
              </span>
            )}
            <m.button
              type="button"
              className="ghost-chip"
              onClick={() => void loadAreas()}
              disabled={isReloading || isLoading}
              whileHover={hoverLift}
              whileTap={tapPress}
              title="Reload staging areas"
            >
              <LoaderCircle
                size={13}
                strokeWidth={2}
                className={isReloading ? "spin" : undefined}
              />
              Refresh
            </m.button>
          </div>
        </div>

        {/* Error state */}
        {loadError && (
          <div className="staging-error-row">
            <AlertTriangle size={13} strokeWidth={2} />
            <span>{loadError}</span>
          </div>
        )}

        {/* Cleanup result banner */}
        {cleanupResult && (
          <div
            className={`staging-cleanup-banner ${
              cleanupResult.errors.length > 0
                ? "staging-cleanup-banner--partial"
                : "staging-cleanup-banner--ok"
            }`}
          >
            {cleanupResult.errors.length === 0 ? (
              <Check size={13} strokeWidth={2} />
            ) : (
              <AlertTriangle size={13} strokeWidth={2} />
            )}
            <span>
              {cleanupResult.deletedCount > 0
                ? `Removed ${cleanupResult.deletedCount} item${
                    cleanupResult.deletedCount !== 1 ? "s" : ""
                  } and freed ${formatBytes(cleanupResult.freedBytes)}.`
                : "Nothing was deleted."}
              {cleanupResult.errors.length > 0 &&
                ` ${cleanupResult.errors.length} error${
                  cleanupResult.errors.length !== 1 ? "s" : ""
                }.`}
            </span>
            <button
              type="button"
              className="staging-banner-close"
              onClick={() => setCleanupResult(null)}
              aria-label="Dismiss"
            >
              ×
            </button>
          </div>
        )}

        {/* Empty state */}
        {!isLoading && summary && summary.areas.length === 0 && (
          <div className="staging-empty-state">
            <Folder size={28} strokeWidth={1.5} />
            <p>No staging areas found.</p>
            <span className="workspace-toolbar-copy-muted">
              The inbox cache is clean. Staging areas appear here when SimSuite
              processes Downloads.
            </span>
          </div>
        )}

        {/* Area list */}
        {summary && summary.areas.length > 0 && (
          <div className="staging-areas-list">
            {/* Bulk select row */}
            <div className="staging-bulk-row">
              <m.button
                type="button"
                className="ghost-chip"
                onClick={() => {
                  if (allSelected) {
                    setSelectedPaths(new Set());
                  } else {
                    setSelectedPaths(new Set(allSubdirPaths));
                  }
                }}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                {allSelected ? (
                  <CheckSquare size={13} strokeWidth={2} />
                ) : (
                  <Square size={13} strokeWidth={2} />
                )}
                {allSelected ? "Deselect all" : "Select all"}
              </m.button>

              {selectedArray.length > 0 && (
                <m.button
                  type="button"
                  className="danger-action"
                  onClick={() => setConfirmingPaths(selectedArray)}
                  disabled={isCleaning}
                  whileHover={hoverLift}
                  whileTap={tapPress}
                >
                  <Trash2 size={13} strokeWidth={2} />
                  Clean selected
                </m.button>
              )}
            </div>

            {/* Areas */}
            {summary.areas.map((area) => (
              <div key={area.itemId} className="staging-area-card">
                <div className="staging-area-header">
                  <button
                    type="button"
                    className="staging-expand-btn"
                    onClick={() => {
                      setExpandedAreas((prev) => {
                        const next = new Set(prev);
                        if (next.has(area.itemId)) {
                          next.delete(area.itemId);
                        } else {
                          next.add(area.itemId);
                        }
                        return next;
                      });
                    }}
                    aria-expanded={expandedAreas.has(area.itemId)}
                  >
                    <ChevronDown
                      size={14}
                      strokeWidth={2}
                      className={`staging-chevron ${
                        expandedAreas.has(area.itemId) ? "staging-chevron--open" : ""
                      }`}
                    />
                    <span className="staging-area-label">{area.itemId}</span>
                    <span className="workspace-toolbar-copy-muted staging-area-count">
                      {area.subdirectories.length} subdir
                      {area.subdirectories.length !== 1 ? "s" : ""}
                    </span>
                  </button>
                </div>

                {expandedAreas.has(area.itemId) && (
                  <div className="staging-subdirs">
                    {area.subdirectories.length === 0 && (
                      <span className="workspace-toolbar-copy-muted staging-empty-subdir">
                        No subdirectories.
                      </span>
                    )}
                    {area.subdirectories.map((sub) => {
                      const isSelected = selectedPaths.has(sub.path);
                      return (
                        <div
                          key={sub.path}
                          className={`staging-subdir-row ${
                            isSelected ? "staging-subdir-row--selected" : ""
                          }`}
                        >
                          <m.button
                            type="button"
                            className="staging-checkbox-btn"
                            onClick={() => {
                              setSelectedPaths((prev) => {
                                const next = new Set(prev);
                                if (next.has(sub.path)) {
                                  next.delete(sub.path);
                                } else {
                                  next.add(sub.path);
                                }
                                return next;
                              });
                            }}
                            whileHover={hoverLift}
                            whileTap={tapPress}
                            aria-pressed={isSelected}
                          >
                            {isSelected ? (
                              <CheckSquare size={15} strokeWidth={2} />
                            ) : (
                              <Square size={15} strokeWidth={2} />
                            )}
                          </m.button>

                          <div className="staging-subdir-info">
                            <code className="staging-subdir-name">{sub.name}</code>
                            <span className="workspace-toolbar-copy-muted staging-subdir-path">
                              {sub.path}
                            </span>
                          </div>

                          <div className="staging-subdir-meta">
                            <span
                              className={`staging-badge ${
                                sub.fileCount > 50
                                  ? "staging-badge--warn"
                                  : "staging-badge--neutral"
                              }`}
                            >
                              {sub.fileCount.toLocaleString()} files
                            </span>
                            <span className="staging-badge staging-badge--size">
                              {formatBytes(sub.totalBytes)}
                            </span>
                            <span className="workspace-toolbar-copy-muted staging-age">
                              {formatAge(sub.createdAt)}
                            </span>
                          </div>

                          <m.button
                            type="button"
                            className="staging-delete-btn"
                            onClick={() => setConfirmingPaths([sub.path])}
                            disabled={isCleaning}
                            whileHover={hoverLift}
                            whileTap={tapPress}
                            title="Delete this subdirectory"
                          >
                            <Trash2 size={13} strokeWidth={2} />
                          </m.button>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Confirmation dialog */}
      {confirmingPaths !== null && (
        <div className="staging-confirm-overlay" role="dialog" aria-modal="true">
          <div className="staging-confirm-dialog">
            <div className="staging-confirm-header">
              <AlertTriangle size={20} strokeWidth={2} />
              <h3>Clean staging area?</h3>
            </div>
            <p className="workspace-toolbar-copy">
              This will permanently delete{" "}
              <strong>{confirmingPaths.length}</strong> director
              {confirmingPaths.length !== 1 ? "ies" : "y"} and all their contents.
              SimSuite will not be able to recover them.
            </p>
            <ul className="staging-confirm-paths">
              {confirmingPaths.slice(0, 5).map((p) => (
                <li key={p}>
                  <code>{p.split(/[/\\]/).pop()}</code>
                </li>
              ))}
              {confirmingPaths.length > 5 && (
                <li className="workspace-toolbar-copy-muted">
                  ...and {confirmingPaths.length - 5} more
                </li>
              )}
            </ul>
            <div className="staging-confirm-actions">
              <m.button
                type="button"
                className="ghost-chip"
                onClick={() => setConfirmingPaths(null)}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Cancel
              </m.button>
              <m.button
                type="button"
                className="danger-action"
                onClick={() => void handleClean(confirmingPaths)}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                {isCleaning ? (
                  <>
                    <LoaderCircle size={13} strokeWidth={2} className="spin" />
                    Deleting...
                  </>
                ) : (
                  <>
                    <Trash2 size={13} strokeWidth={2} />
                    Delete permanently
                  </>
                )}
              </m.button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

function SettingsLayoutSection({
  onResetPanelSizes,
}: {
  onResetPanelSizes: () => void;
}) {
  return (
    <>
      <div className="panel-heading settings-focus-heading">
        <div>
          <span className="section-label">
            <RotateCcw size={14} strokeWidth={2} />
            Layout memory
          </span>
          <h2>Reset saved panels</h2>
        </div>
        <p className="workspace-toolbar-copy">
          Use this when panels, docks, or saved workspace presets feel too far from the
          default layout.
        </p>
      </div>

      <div className="settings-summary-card settings-focus-block">
        <span className="section-label">Reset only the layout</span>
        <strong>Keep your files and scan data untouched</strong>
        <p className="workspace-toolbar-copy">
          This only resets saved widths, heights, and dock arrangements. It does not
          touch your library, review queues, or learned fixes.
        </p>

        <div className="settings-action-row">
          <m.button
            type="button"
            className="secondary-action"
            onClick={onResetPanelSizes}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            <RotateCcw size={16} strokeWidth={2} />
            Reset panels
          </m.button>
          <p className="workspace-toolbar-copy settings-inline-note">
            Handy after lots of resizing, or after switching between very different
            monitor sizes.
          </p>
        </div>
      </div>
    </>
  );
}

function IgnorePatternsEditor({
  patterns,
  onSave,
  disabled,
}: {
  patterns: string[];
  onSave: (patterns: string[]) => Promise<void>;
  disabled: boolean;
}) {
  const [localPatterns, setLocalPatterns] = useState(patterns.join("\n"));
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Sync local state when props change
  useEffect(() => {
    setLocalPatterns(patterns.join("\n"));
  }, [patterns]);

  async function handleSave() {
    const trimmed = localPatterns
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    setIsSaving(true);
    setError(null);
    try {
      await onSave(trimmed);
    } catch (err) {
      setError(toErrorMessage(err));
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="settings-focus-grid">
      <div className="settings-summary-card settings-focus-block">
        <span className="section-label">Current patterns</span>
        <textarea
          className="ignore-patterns-textarea"
          value={localPatterns}
          onChange={(e) => setLocalPatterns(e.target.value)}
          onBlur={handleSave}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void handleSave();
            }
          }}
          disabled={disabled || isSaving}
          rows={Math.max(4, localPatterns.split("\n").length + 1)}
          placeholder="e.g. sample, wallpaper, default"

        />
        <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
          {isSaving ? (
            <span className="settings-inline-status">
              <LoaderCircle size={14} strokeWidth={2} className="spin" />
              Saving...
            </span>
          ) : (
            "One pattern per line. Case-insensitive. Changes save automatically."
          )}
        </p>
        {error ? (
          <p className="workspace-toolbar-copy workspace-toolbar-copy-muted settings-status-banner">
            {error}
          </p>
        ) : null}
      </div>
    </div>
  );
}

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

function watchIntervalLabel(hours: number) {
  if (hours <= 1) {
    return "every hour";
  }

  if (hours === 24) {
    return "every day";
  }

  return `every ${hours} hours`;
}

function formatWatchRefreshMessage(summary: WatchRefreshSummary) {
  const parts = [
    `${summary.checkedSubjects} watched item${
      summary.checkedSubjects === 1 ? "" : "s"
    } checked`,
    `${summary.exactUpdateItems} confirmed update${
      summary.exactUpdateItems === 1 ? "" : "s"
    }`,
  ];

  if (summary.possibleUpdateItems > 0) {
    parts.push(
      `${summary.possibleUpdateItems} possible update${
        summary.possibleUpdateItems === 1 ? "" : "s"
      }`,
    );
  }

  return `${parts.join(", ")}.`;
}
