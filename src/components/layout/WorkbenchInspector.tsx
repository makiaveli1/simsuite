import { useState, type ReactNode } from "react";
import { ResizableEdgeHandle } from "../ResizableEdgeHandle";
import { useUiPreferences } from "../UiPreferencesContext";

interface WorkbenchInspectorProps {
  children: ReactNode;
  className?: string;
  ariaLabel?: string;
  width?: number;
  onWidthChange?: (width: number) => void;
  minWidth?: number;
  maxWidth?: number;
  /** Make the inspector collapsible with a toggle button */
  collapsible?: boolean;
  /** Start collapsed (only used when collapsible=true) */
  defaultCollapsed?: boolean;
  noPadding?: boolean;
  noBorder?: boolean;
  hideHandle?: boolean;
}

export function WorkbenchInspector({
  children,
  className,
  ariaLabel = "Inspector",
  width,
  onWidthChange,
  minWidth = 280,
  maxWidth = 720,
  collapsible = false,
  defaultCollapsed = false,
  noPadding = false,
  noBorder = false,
  hideHandle = false,
}: WorkbenchInspectorProps) {
  const { inspectorWidth, setInspectorWidth, density } = useUiPreferences();
  const activeWidth = width ?? inspectorWidth;
  const handleWidthChange = onWidthChange ?? setInspectorWidth;
  const densityClass =
    density === "compact"
      ? "inspector-compact"
      : density === "balanced"
        ? "inspector-balanced"
        : "inspector-spacious";

  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  const classes = [
    "workbench-inspector",
    "resizable-panel",
    densityClass,
    className,
    noBorder ? "inspector-no-border" : null,
    collapsed ? "inspector-collapsed" : null,
  ]
    .filter(Boolean)
    .join(" ");

  const inlineStyle = collapsed
    ? { width: 0, minWidth: 0, overflow: "hidden" }
    : { width: `${activeWidth}px` };

  return (
    <aside
      className={classes}
      aria-label={ariaLabel}
      aria-expanded={collapsible ? !collapsed : undefined}
      style={inlineStyle}
    >
      {/* Collapse button — shown when open and collapsible */}
      {collapsible && !collapsed && (
        <button
          type="button"
          className="inspector-collapse-btn"
          onClick={() => setCollapsed(true)}
          aria-label="Collapse inspector panel"
          title="Collapse inspector"
        >
          ‹
        </button>
      )}

      {/* Expand trigger — shown when collapsed */}
      {collapsible && collapsed && (
        <button
          type="button"
          className="inspector-expand-btn"
          onClick={() => setCollapsed(false)}
          aria-label="Expand inspector panel"
          title="Show inspector"
        >
          ›
          <span>Inspector</span>
        </button>
      )}

      {/* Resize handle — hidden when collapsed */}
      {!hideHandle && !collapsed && (
        <ResizableEdgeHandle
          label="Resize inspector panel"
          value={activeWidth}
          min={minWidth}
          max={maxWidth}
          onChange={handleWidthChange}
          side="left"
        />
      )}

      {/* Content — hidden via CSS when collapsed */}
      <div
        className={`inspector-content${noPadding ? " inspector-no-padding" : ""}`}
        aria-hidden={collapsed}
        style={collapsed ? { visibility: "hidden", pointerEvents: "none" } : undefined}
      >
        {children}
      </div>
    </aside>
  );
}
