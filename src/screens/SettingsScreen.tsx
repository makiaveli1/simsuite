import { m } from "motion/react";
import {
  LayoutPanelLeft,
  Palette,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
} from "lucide-react";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { hoverLift, stagedListItem, tapPress } from "../lib/motion";
import { UI_THEMES, getThemeDefinition } from "../lib/themeMeta";
import type { UiDensity, UserView } from "../lib/types";
import { screenHelperLine } from "../lib/uiLanguage";

const USER_VIEWS: Array<{
  id: UserView;
  label: string;
  badge: string;
  headline: string;
  hint: string;
  traits: string[];
}> = [
  {
    id: "beginner",
    label: "Beginner",
    badge: "Guided",
    headline: "One big next move, fewer rabbit holes",
    hint: "Keeps the app calmer, puts the safest next button up front, and uses friendlier Sims-style wording.",
    traits: [
      "Shows the clearest next step first",
      "Starts previews in example mode",
      "Keeps the wording extra cozy",
    ],
  },
  {
    id: "standard",
    label: "Standard",
    badge: "Balanced",
    headline: "Enough clues to stay confident without the wall of receipts",
    hint: "Keeps the main next step easy to find, but leaves more clues open while you sort.",
    traits: [
      "Balances guidance and detail",
      "Keeps samples open by default",
      "Leaves more proof visible while you sort",
    ],
  },
  {
    id: "power",
    label: "Pro",
    badge: "Full receipts",
    headline: "Dense, direct, and ready for deep CC gremlin work",
    hint: "Opens the full receipts: raw paths, denser details, and deeper checks for fine control.",
    traits: [
      "Opens fuller file lists by default",
      "Shows denser path and evidence detail",
      "Best for deep cleanup marathons",
    ],
  },
];

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
  userView: UserView;
  onUserViewChange: (view: UserView) => void;
}

export function SettingsScreen({
  userView,
  onUserViewChange,
}: SettingsScreenProps) {
  const { theme, density, setTheme, setDensity, resetPanelSizes } =
    useUiPreferences();
  const activeTheme = getThemeDefinition(theme);
  const activeView = USER_VIEWS.find((item) => item.id === userView) ?? USER_VIEWS[1];
  const activeDensity =
    DENSITIES.find((item) => item.id === density) ?? DENSITIES[1];

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
              {screenHelperLine("settings", userView)}
            </p>
          </div>
        </div>

        <div className="workspace-toolbar-chip-group settings-header-chips">
          <span className="workspace-status-chip">{activeView.label} view</span>
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
              {USER_VIEWS.map((item) => (
                <m.button
                  key={item.id}
                  type="button"
                  className={`settings-view-card ${userView === item.id ? "is-active" : ""}`}
                  onClick={() => onUserViewChange(item.id)}
                  title={item.hint}
                  whileHover={hoverLift}
                  whileTap={tapPress}
                >
                  <div className="settings-view-card-topline">
                    <strong>{item.label}</strong>
                    <span className="ghost-chip">{item.badge}</span>
                  </div>
                  <span className="settings-view-headline">{item.headline}</span>
                  <p className="workspace-toolbar-copy">{item.hint}</p>
                  <div className="settings-view-traits">
                    {item.traits.map((trait) => (
                      <span key={trait} className="settings-view-trait">
                        {trait}
                      </span>
                    ))}
                  </div>
                </m.button>
              ))}
            </div>

            <div className="settings-summary-card">
              <span className="section-label">Current fit</span>
              <strong>{activeView.label} view</strong>
              <p className="workspace-toolbar-copy">{activeView.hint}</p>
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
                <span>View</span>
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
