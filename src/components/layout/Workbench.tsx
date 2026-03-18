import type { ReactNode } from "react";
import { useUiPreferences } from "../UiPreferencesContext";

interface WorkbenchProps {
  children: ReactNode;
  className?: string;
  threePanel?: boolean;
  // New props for better control
  noPadding?: boolean;
  fullHeight?: boolean;
}

export function Workbench({ 
  children, 
  className, 
  threePanel = false,
  noPadding = false,
  fullHeight = false
}: WorkbenchProps) {
  const layoutClass = threePanel ? "workbench-three-panel" : "workbench-two-panel";
  const { density } = useUiPreferences();
  const densityClass = density === "compact" ? "workbench-compact" : 
                      density === "balanced" ? "workbench-balanced" : 
                      "workbench-spacious";
  const classes = [
    "workbench",
    layoutClass,
    densityClass,
    className,
    noPadding ? "workbench-no-padding" : null,
    fullHeight ? "workbench-full-height" : null,
  ]
    .filter(Boolean)
    .join(" ");
  
  return (
    <div 
      className={classes}
      style={{ 
        height: fullHeight ? "100%" : undefined,
        padding: noPadding ? "0" : undefined 
      }}
    >
      {children}
    </div>
  );
}
