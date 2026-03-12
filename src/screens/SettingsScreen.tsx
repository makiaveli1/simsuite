import { useEffect, useState } from "react";
import { m } from "motion/react";
import {
  LayoutPanelLeft,
  LoaderCircle,
  Palette,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
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
import type { AppBehaviorSettings, ExperienceMode, UiDensity } from "../lib/types";
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
  const [isSavingBackgroundMode, setIsSavingBackgroundMode] = useState(false);
  const [backgroundModeError, setBackgroundModeError] = useState<string | null>(null);
  const activeTheme = getThemeDefinition(theme);
  const activeView = {
    ...EXPERIENCE_MODE_PROFILES[experienceMode],
    ...EXPERIENCE_CARDS[experienceMode],
  };
  const activeDensity =
    DENSITIES.find((item) => item.id === density) ?? DENSITIES[1];
  const keepRunningInBackground = appBehavior?.keepRunningInBackground ?? false;

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

    return () => {
      cancelled = true;
    };
  }, []);

  async function updateBackgroundMode(keepRunning: boolean) {
    if (appBehavior?.keepRunningInBackground === keepRunning) {
      return;
    }

    setIsSavingBackgroundMode(true);
    setBackgroundModeError(null);

    try {
      const nextSettings = await api.saveAppBehaviorSettings({
        keepRunningInBackground: keepRunning,
      });
      setAppBehavior(nextSettings);
    } catch (error) {
      setBackgroundModeError(toErrorMessage(error));
    } finally {
      setIsSavingBackgroundMode(false);
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

      <div className="settings-layout">
        <div className="settings-stack">
          <m.section className="panel-card" {...stagedListItem(0)}>
            <div className="panel-heading">
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
          </m.section>

          <m.section className="panel-card" {...stagedListItem(1)}>
            <div className="panel-heading">
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
                  onClick={() => setTheme(item.id)}
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
          </m.section>

          <m.section className="panel-card" {...stagedListItem(2)}>
            <div className="panel-heading">
              <div>
                <span className="section-label">
                  <LayoutPanelLeft size={14} strokeWidth={2} />
                  Workspace size
                </span>
                <h2>Choose panel density</h2>
              </div>
              <p className="workspace-toolbar-copy">
                Density changes row spacing, panel padding, and how tightly the app
                packs information.
              </p>
            </div>

            <div className="segmented-control" role="tablist" aria-label="Density">
              {DENSITIES.map((item) => (
                <m.button
                  key={item.id}
                  type="button"
                  className={`segment-button ${density === item.id ? "is-active" : ""}`}
                  onClick={() => setDensity(item.id)}
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
          </m.section>

          <m.section className="panel-card" {...stagedListItem(3)}>
            <div className="panel-heading">
              <div>
                <span className="section-label">
                  <Workflow size={14} strokeWidth={2} />
                  Background mode
                </span>
                <h2>Choose what happens on close</h2>
              </div>
              <p className="workspace-toolbar-copy">
                Keep SimSuite alive in the tray only if you want Downloads watching to
                keep going after you close the main window.
              </p>
            </div>

            <div className="segmented-control" role="tablist" aria-label="Close behavior">
              <m.button
                type="button"
                className={`segment-button ${
                  appBehavior && !keepRunningInBackground ? "is-active" : ""
                }`}
                onClick={() => void updateBackgroundMode(false)}
                disabled={!appBehavior || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Close app
              </m.button>
              <m.button
                type="button"
                className={`segment-button ${
                  keepRunningInBackground ? "is-active" : ""
                }`}
                onClick={() => void updateBackgroundMode(true)}
                disabled={!appBehavior || isSavingBackgroundMode}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                Hide to tray
              </m.button>
            </div>

            <div className="settings-summary-card">
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
              {backgroundModeError ? (
                <p className="workspace-toolbar-copy workspace-toolbar-copy-muted">
                  {backgroundModeError}
                </p>
              ) : null}
            </div>
          </m.section>

          <m.section className="panel-card" {...stagedListItem(4)}>
            <div className="panel-heading">
              <div>
                <span className="section-label">
                  <RotateCcw size={14} strokeWidth={2} />
                  Layout memory
                </span>
                <h2>Reset saved panels</h2>
              </div>
              <p className="workspace-toolbar-copy">
                Use this when panels, docks, or saved workspace presets feel too far
                from the default layout.
              </p>
            </div>

            <div className="settings-action-row">
              <m.button
                type="button"
                className="secondary-action"
                onClick={resetPanelSizes}
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                <RotateCcw size={16} strokeWidth={2} />
                Reset panels
              </m.button>
              <p className="workspace-toolbar-copy settings-inline-note">
                This keeps your files and scan data untouched. It only resets saved
                widths, heights, and dock arrangements.
              </p>
            </div>
          </m.section>
        </div>

        <aside className="settings-aside">
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
                Saved locally on this PC so your workspace feels the same next time.
              </div>
              <div className="settings-note">
                Safe actions still follow the same scan, review, approval, and restore
                flow no matter which skin or view you pick.
              </div>
              <div className="settings-note">
                Use Home for folders and scan status, then come back here only when you
                want to personalize the interface.
              </div>
            </div>
          </m.section>
        </aside>
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
