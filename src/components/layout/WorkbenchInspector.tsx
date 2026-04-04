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
  /** Start collapsed (only used when collapsible=true and no controlled collapsed prop) */
  defaultCollapsed?: boolean;
  /** Controlled collapse state — if provided, component is controlled */
  collapsed?: boolean;
  /** Callback when collapse state changes — required when collapsed is controlled */
  onCollapse?: (collapsed: boolean) => void;
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
  collapsed: controlledCollapsed,
  onCollapse,
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

  // Support both controlled (props) and uncontrolled (local) collapse state
  const isControlled = controlledCollapsed !== undefined;
  const [internalCollapsed, setInternalCollapsed] = useState(defaultCollapsed);
  const collapsed = isControlled ? controlledCollapsed : internalCollapsed;
  const toggleCollapsed = () => {
    const next = !collapsed;
    if (isControlled) {
      onCollapse?.(next);
    } else {
      setInternalCollapsed(next);
    }
  };

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

  // When collapsed: leave 20px visible so the expand tab can be seen
  // The expand button is absolutely positioned inside the aside
  const inlineStyle = collapsed
    ? { width: "20px", minWidth: "20px", overflow: "hidden" }
    : { '--inspector-width': `${activeWidth}px` };

  return (
    <aside
      className={classes}
      aria-label={ariaLabel}
      aria-expanded={collapsible ? !collapsed : undefined}
      style={inlineStyle}
    >
      {/* Collapse button — only shown when collapsible but children don't provide their own (non-collapsible mode) */}
      {/* When collapsible=true, LibraryDetailsPanel handles the collapse button via its headerRight prop */}
      {!collapsible && (
        <button
          type="button"
          className="inspector-collapse-btn"
          onClick={toggleCollapsed}
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
          onClick={toggleCollapsed}
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
