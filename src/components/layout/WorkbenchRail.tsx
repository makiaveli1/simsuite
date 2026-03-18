import type { ReactNode } from "react";
import { ResizableEdgeHandle } from "../ResizableEdgeHandle";
import { useUiPreferences } from "../UiPreferencesContext";

interface WorkbenchRailProps {
  children: ReactNode;
  className?: string;
  ariaLabel?: string;
  width?: number;
  onWidthChange?: (width: number) => void;
  minWidth?: number;
  maxWidth?: number;
  resizable?: boolean;
  // New props for better control
  noPadding?: boolean;
  noBorder?: boolean;
  hideHandle?: boolean;
}

export function WorkbenchRail({
  children,
  className,
  ariaLabel = "Sidebar Rail",
  width,
  onWidthChange,
  minWidth = 200,
  maxWidth = 400,
  resizable = false,
  noPadding = false,
  noBorder = false,
  hideHandle = false
}: WorkbenchRailProps) {
  const { sidebarWidth, setSidebarWidth, density } = useUiPreferences();
  const activeWidth = width ?? sidebarWidth;
  const handleWidthChange = onWidthChange ?? setSidebarWidth;
  const densityClass = density === "compact" ? "rail-compact" : 
                      density === "balanced" ? "rail-balanced" : 
                      "rail-spacious";
  const classes = [
    "workbench-rail",
    densityClass,
    className,
    resizable ? "resizable-panel" : null,
    noBorder ? "rail-no-border" : null,
  ]
    .filter(Boolean)
    .join(" ");

  const inlineStyle = resizable && width ? { width: `${width}px` } : undefined;

  return (
    <aside
      className={classes}
      aria-label={ariaLabel}
      style={inlineStyle}
    >
      {!hideHandle && resizable && onWidthChange && width !== undefined && (
        <ResizableEdgeHandle
          label="Resize rail panel"
          value={activeWidth}
          min={minWidth}
          max={maxWidth}
          onChange={handleWidthChange}
          side="right"
        />
      )}
      <div className={`rail-content${noPadding ? " rail-no-padding" : ""}`}>
        {children}
      </div>
    </aside>
  );
}
