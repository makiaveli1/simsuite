import type { ReactNode } from "react";
import { ResizableEdgeHandle } from "./ResizableEdgeHandle";
import { useUiPreferences } from "./UiPreferencesContext";

interface ResizableDetailPanelProps {
  children: ReactNode;
  className?: string;
  ariaLabel?: string;
  width?: number;
  onWidthChange?: (width: number) => void;
  minWidth?: number;
  maxWidth?: number;
}

export function ResizableDetailPanel({
  children,
  className,
  ariaLabel = "Inspector",
  width,
  onWidthChange,
  minWidth = 300,
  maxWidth = 720,
}: ResizableDetailPanelProps) {
  const { inspectorWidth, setInspectorWidth } = useUiPreferences();
  const activeWidth = width ?? inspectorWidth;
  const handleWidthChange = onWidthChange ?? setInspectorWidth;

  return (
    <aside
      className={`panel-card detail-panel resizable-panel${className ? ` ${className}` : ""}`}
      aria-label={ariaLabel}
    >
      <ResizableEdgeHandle
        label="Resize inspector panel"
        value={activeWidth}
        min={minWidth}
        max={maxWidth}
        onChange={handleWidthChange}
        side="left"
      />
      {children}
    </aside>
  );
}
