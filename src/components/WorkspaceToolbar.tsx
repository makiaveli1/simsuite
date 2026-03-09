import { m } from "motion/react";
import { LayoutPanelLeft, SlidersHorizontal } from "lucide-react";
import type { Screen, UiDensity, UserView } from "../lib/types";
import { hoverLift, tapPress } from "../lib/motion";
import { getThemeDefinition } from "../lib/themeMeta";
import { useUiPreferences } from "./UiPreferencesContext";

const USER_VIEW_LABELS: Record<UserView, string> = {
  beginner: "Easy",
  standard: "Standard",
  power: "Power",
};

const DENSITY_LABELS: Record<UiDensity, string> = {
  compact: "Snug",
  balanced: "Normal",
  roomy: "Roomy",
};

const SCREEN_LABELS: Record<Screen, string> = {
  home: "Home",
  downloads: "New files",
  library: "My CC",
  creatorAudit: "Creator names",
  categoryAudit: "Mod types",
  duplicates: "Same file?",
  organize: "Tidy up",
  review: "Needs help",
  settings: "Settings",
};

interface WorkspaceToolbarProps {
  userView: UserView;
  currentScreen: Screen;
  onOpenSettings: () => void;
}

export function WorkspaceToolbar({
  userView,
  currentScreen,
  onOpenSettings,
}: WorkspaceToolbarProps) {
  const { theme, density } = useUiPreferences();
  const activeTheme = getThemeDefinition(theme);
  const viewLabel = USER_VIEW_LABELS[userView];
  const densityLabel = DENSITY_LABELS[density];

  return (
    <div className="workspace-toolbar">
      <div className="workspace-toolbar-status">
        <div className="workspace-toolbar-heading">
          <span className="section-label">
            <LayoutPanelLeft size={14} strokeWidth={2} />
            {SCREEN_LABELS[currentScreen]}
          </span>
          <div className="workspace-toolbar-meta" aria-label="Current workspace preferences">
            <span className="workspace-toolbar-meta-item">{viewLabel}</span>
            <span className="workspace-toolbar-meta-divider" aria-hidden="true" />
            <span className="workspace-toolbar-meta-item">{densityLabel}</span>
            <span className="workspace-toolbar-meta-divider" aria-hidden="true" />
            <span className="workspace-toolbar-meta-item">{activeTheme.label}</span>
          </div>
        </div>
      </div>

      <m.button
        type="button"
        className="secondary-action workspace-toolbar-settings-link"
        onClick={onOpenSettings}
        disabled={currentScreen === "settings"}
        whileHover={currentScreen === "settings" ? undefined : hoverLift}
        whileTap={currentScreen === "settings" ? undefined : tapPress}
      >
        <SlidersHorizontal size={16} strokeWidth={2} />
        {currentScreen === "settings" ? "Settings" : "Open settings"}
      </m.button>
    </div>
  );
}
