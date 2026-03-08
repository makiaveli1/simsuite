import {
  LayoutTemplate,
  PanelTopClose,
  PanelTopOpen,
} from "lucide-react";

interface LayoutPresetBarProps {
  title: string;
  summary: string;
  presets: Array<{
    id: string;
    label: string;
    hint: string;
  }>;
  activePreset: string;
  onApplyPreset: (preset: string) => void;
  filterToggle?: {
    collapsed: boolean;
    onToggle: () => void;
    hiddenLabel: string;
    shownLabel: string;
  };
}

export function LayoutPresetBar({
  title,
  summary,
  presets,
  activePreset,
  onApplyPreset,
  filterToggle,
}: LayoutPresetBarProps) {
  const isCustom = !presets.some((preset) => preset.id === activePreset);

  return (
    <div className="panel-card layout-strip">
      <div className="layout-strip-group">
        <span className="section-label">
          <LayoutTemplate size={13} strokeWidth={2} />
          {title}
        </span>
        <div className="layout-preset-group" role="toolbar" aria-label={title}>
          {presets.map((preset) => (
            <button
              key={preset.id}
              type="button"
              className={`layout-preset-button ${
                activePreset === preset.id ? "is-active" : ""
              }`}
              onClick={() => onApplyPreset(preset.id)}
              title={preset.hint}
            >
              {preset.label}
            </button>
          ))}
          {isCustom ? <span className="ghost-chip">Custom</span> : null}
        </div>
      </div>

      {filterToggle ? (
        <button
          type="button"
          className="secondary-action layout-toggle-button"
          onClick={filterToggle.onToggle}
          title={
            filterToggle.collapsed
              ? filterToggle.hiddenLabel
              : filterToggle.shownLabel
          }
        >
          {filterToggle.collapsed ? (
            <PanelTopOpen size={14} strokeWidth={2} />
          ) : (
            <PanelTopClose size={14} strokeWidth={2} />
          )}
          {filterToggle.collapsed
            ? filterToggle.hiddenLabel
            : filterToggle.shownLabel}
        </button>
      ) : null}

      <p className="layout-strip-copy">{summary}</p>
    </div>
  );
}
