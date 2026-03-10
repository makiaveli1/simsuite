import { m } from "motion/react";
import { LayoutPanelLeft, SlidersHorizontal } from "lucide-react";
import type { ExperienceMode, Screen, UiDensity } from "../lib/types";
import { getExperienceModeProfile } from "../lib/experienceMode";
import { hoverLift, tapPress } from "../lib/motion";
import { getThemeDefinition } from "../lib/themeMeta";
import { useUiPreferences } from "./UiPreferencesContext";
import { screenLabel, viewModeLabel } from "../lib/uiLanguage";

const DENSITY_LABELS: Record<UiDensity, string> = {
  compact: "Snug",
  balanced: "Normal",
  roomy: "Roomy",
};

interface WorkspaceToolbarProps {
  experienceMode: ExperienceMode;
  currentScreen: Screen;
  onOpenSettings: () => void;
}

export function WorkspaceToolbar({
  experienceMode,
  currentScreen,
  onOpenSettings,
}: WorkspaceToolbarProps) {
  const { theme, density } = useUiPreferences();
  const activeTheme = getThemeDefinition(theme);
  const modeProfile = getExperienceModeProfile(experienceMode);
  const viewLabel = viewModeLabel(experienceMode);
  const densityLabel = DENSITY_LABELS[density];

  return (
    <div className="workspace-toolbar">
      <div className="workspace-toolbar-status">
        <div className="workspace-toolbar-heading">
          <span className="section-label">
            <LayoutPanelLeft size={14} strokeWidth={2} />
            {screenLabel(currentScreen, experienceMode)}
          </span>
          <p className="workspace-toolbar-copy">{modeProfile.workspaceSummary}</p>
          <div className="workspace-toolbar-meta" aria-label="Current workspace preferences">
            <span className="workspace-toolbar-meta-item">{viewLabel}</span>
            <span className="workspace-toolbar-meta-divider" aria-hidden="true" />
            <span className="workspace-toolbar-meta-item">{modeProfile.badge}</span>
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
