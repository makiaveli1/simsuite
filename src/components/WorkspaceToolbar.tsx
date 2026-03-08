import { LayoutPanelLeft, Palette, SlidersHorizontal } from "lucide-react";
import type { UiDensity, UiTheme, UserView } from "../lib/types";
import { useUiPreferences } from "./UiPreferencesContext";

const USER_VIEWS: Array<{
  id: UserView;
  label: string;
  hint: string;
}> = [
  {
    id: "beginner",
    label: "Easy",
    hint: "Shows the basics first and keeps the screen calmer.",
  },
  {
    id: "standard",
    label: "Standard",
    hint: "Balanced detail for everyday CC sorting.",
  },
  {
    id: "power",
    label: "Power",
    hint: "Shows raw paths, deeper filters, and rule detail.",
  },
];

const THEMES: Array<{
  id: UiTheme;
  label: string;
  hint: string;
}> = [
  {
    id: "plumbob",
    label: "Plumbob",
    hint: "Classic SimSuite green with calm dark chrome.",
  },
  {
    id: "buildbuy",
    label: "Build/Buy",
    hint: "Warm catalog brass with workshop-style contrast.",
  },
  {
    id: "cas",
    label: "CAS",
    hint: "Cool studio blue for hair, skin, and outfit work.",
  },
  {
    id: "neighborhood",
    label: "Neighborhood",
    hint: "Dusk map colors with coral signals and teal glass.",
  },
  {
    id: "debuggrid",
    label: "Debug Grid",
    hint: "Industrial slate and safety orange for power sorting.",
  },
  {
    id: "sunroom",
    label: "Sunroom",
    hint: "Bright parchment panels with teal controls and softer glare.",
  },
];

const DENSITIES: Array<{
  id: UiDensity;
  label: string;
}> = [
  { id: "compact", label: "Snug" },
  { id: "balanced", label: "Normal" },
  { id: "roomy", label: "Roomy" },
];

interface WorkspaceToolbarProps {
  userView: UserView;
  onChange: (view: UserView) => void;
}

export function WorkspaceToolbar({
  userView,
  onChange,
}: WorkspaceToolbarProps) {
  const active = USER_VIEWS.find((item) => item.id === userView) ?? USER_VIEWS[1];
  const { theme, density, setTheme, setDensity, resetPanelSizes } =
    useUiPreferences();
  const activeTheme = THEMES.find((item) => item.id === theme) ?? THEMES[0];

  return (
    <div className="workspace-toolbar">
      <div className="workspace-toolbar-group workspace-toolbar-primary">
        <span className="section-label">View mode</span>
        <div className="segmented-control" role="tablist" aria-label="User view">
          {USER_VIEWS.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`segment-button ${userView === item.id ? "is-active" : ""}`}
              onClick={() => onChange(item.id)}
              title={item.hint}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      <div className="workspace-toolbar-group workspace-toolbar-settings">
        <label className="toolbar-select">
          <span className="toolbar-select-label">
            <Palette size={14} strokeWidth={2} />
            Theme
          </span>
          <select
            value={theme}
            onChange={(event) => setTheme(event.target.value as UiTheme)}
          >
            {THEMES.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>

        <label className="toolbar-select">
          <span className="toolbar-select-label">
            <SlidersHorizontal size={14} strokeWidth={2} />
            Size
          </span>
          <select
            value={density}
            onChange={(event) => setDensity(event.target.value as UiDensity)}
          >
            {DENSITIES.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>

        <button
          type="button"
          className="secondary-action toolbar-reset"
          onClick={resetPanelSizes}
          title="Reset saved panel sizes and workspace layouts"
        >
          <LayoutPanelLeft size={14} strokeWidth={2} />
          Reset panels
        </button>
      </div>

      <p className="workspace-toolbar-copy">
        {active.hint} Skin: {activeTheme.hint}
      </p>
    </div>
  );
}
