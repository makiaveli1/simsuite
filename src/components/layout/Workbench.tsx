import type { HTMLAttributes, ReactNode } from "react";
import { useUiPreferences } from "../UiPreferencesContext";

interface WorkbenchProps extends HTMLAttributes<HTMLDivElement> {
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
  fullHeight = false,
  style,
  ...rest
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
      {...rest}
      className={classes}
      style={{ 
        ...style,
        height: fullHeight ? "100%" : style?.height,
        padding: noPadding ? "0" : style?.padding,
      }}
    >
      {children}
    </div>
  );
}
