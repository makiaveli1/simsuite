import type { ReactNode } from "react";

interface WorkbenchStageProps {
  children: ReactNode;
  className?: string;
}

export function WorkbenchStage({ children, className }: WorkbenchStageProps) {
  return (
    <section className={`workbench-surface${className ? ` ${className}` : ""}`}>
      {children}
    </section>
  );
}
