import type { LucideIcon } from "lucide-react";
import { Info } from "lucide-react";
import { m } from "motion/react";
import type { ReactNode } from "react";

interface StatePanelProps {
  eyebrow: string;
  title: string;
  body: ReactNode;
  icon?: LucideIcon;
  tone?: "neutral" | "good" | "warn" | "danger" | "info";
  compact?: boolean;
  badge?: string;
  actions?: ReactNode;
  meta?: string[];
}

export function StatePanel({
  eyebrow,
  title,
  body,
  icon: Icon = Info,
  tone = "neutral",
  compact = false,
  badge,
  actions,
  meta,
}: StatePanelProps) {
  return (
    <m.div
      className={`state-panel state-panel-${tone} ${compact ? "state-panel-compact" : ""}`}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -6 }}
      transition={{ duration: 0.2 }}
    >
      <div className="state-panel-head">
        <div className="state-panel-badge-wrap">
          <span className="state-panel-icon" aria-hidden="true">
            <Icon size={18} strokeWidth={2} />
          </span>
          <div className="state-panel-copy">
            <p className="eyebrow">{eyebrow}</p>
            <h2>{title}</h2>
          </div>
        </div>
        {badge ? <span className="ghost-chip">{badge}</span> : null}
      </div>

      <p className="state-panel-body">{body}</p>

      {meta?.length ? (
        <div className="state-panel-meta">
          {meta.map((item) => (
            <span key={item} className="ghost-chip">
              {item}
            </span>
          ))}
        </div>
      ) : null}

      {actions ? <div className="state-panel-actions">{actions}</div> : null}
    </m.div>
  );
}
