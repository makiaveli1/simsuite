import type { ReactNode } from "react";
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
  // New props for better control
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
  noPadding = false,
  noBorder = false,
  hideHandle = false
}: WorkbenchInspectorProps) {
  const { inspectorWidth, setInspectorWidth, density } = useUiPreferences();
  const activeWidth = width ?? inspectorWidth;
  const handleWidthChange = onWidthChange ?? setInspectorWidth;
  const densityClass = density === "compact" ? "inspector-compact" : 
                      density === "balanced" ? "inspector-balanced" : 
                      "inspector-spacious";
  const classes = [
    "workbench-inspector",
    "resizable-panel",
    densityClass,
    className,
    noBorder ? "inspector-no-border" : null,
  ]
    .filter(Boolean)
    .join(" ");

  const inlineStyle = { width: `${activeWidth}px` };

  return (
    <aside
      className={classes}
      aria-label={ariaLabel}
      style={inlineStyle}
    >
      {!hideHandle && (
        <ResizableEdgeHandle
          label="Resize inspector panel"
          value={activeWidth}
          min={minWidth}
          max={maxWidth}
          onChange={handleWidthChange}
          side="left"
        />
      )}
      <div className={`inspector-content${noPadding ? " inspector-no-padding" : ""}`}>
        {children}
      </div>
    </aside>
  );
}
