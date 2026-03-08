import type { ReactNode } from "react";
import { ArrowDown, ArrowUp, ChevronDown, RotateCcw } from "lucide-react";
import { useUiPreferences } from "./UiPreferencesContext";

export interface DockSectionDefinition<T extends string = string> {
  id: T;
  label: string;
  hint?: string;
  badge?: string | null;
  defaultCollapsed?: boolean;
  children: ReactNode;
}

interface DockSectionStackProps<T extends string = string> {
  layoutId: string;
  sections: Array<DockSectionDefinition<T>>;
  intro?: string;
  resetLabel?: string;
  className?: string;
}

export function DockSectionStack<T extends string = string>({
  layoutId,
  sections,
  intro,
  resetLabel = "Reset panel order",
  className,
}: DockSectionStackProps<T>) {
  const {
    getDockSectionLayout,
    setDockSectionOrder,
    setDockSectionCollapsed,
    resetDockSectionLayout,
  } = useUiPreferences();

  const sectionIds = sections.map((section) => section.id);
  const defaults = Object.fromEntries(
    sections.map((section) => [section.id, Boolean(section.defaultCollapsed)]),
  );
  const layout = getDockSectionLayout(layoutId, sectionIds, defaults);
  const orderedSections = layout.order
    .map((id) => sections.find((section) => section.id === id))
    .filter((section): section is DockSectionDefinition<T> => Boolean(section));

  function moveSection(sectionId: T, direction: -1 | 1) {
    const index = layout.order.indexOf(sectionId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= layout.order.length) {
      return;
    }

    const nextOrder = [...layout.order];
    [nextOrder[index], nextOrder[target]] = [nextOrder[target], nextOrder[index]];
    setDockSectionOrder(layoutId, nextOrder);
  }

  return (
    <div className={`dock-stack${className ? ` ${className}` : ""}`}>
      {intro ? (
        <div className="dock-stack-toolbar">
          <p className="dock-stack-copy">{intro}</p>
          <button
            type="button"
            className="dock-stack-reset"
            onClick={() => resetDockSectionLayout(layoutId)}
            title={resetLabel}
          >
            <RotateCcw size={13} strokeWidth={2} />
            Reset
          </button>
        </div>
      ) : null}

      {orderedSections.map((section, index) => {
        const collapsed = layout.collapsed[section.id] ?? false;

        return (
          <section
            key={section.id}
            className={`dock-section${collapsed ? " is-collapsed" : ""}`}
          >
            <div className="dock-section-header">
              <button
                type="button"
                className="dock-section-toggle"
                onClick={() =>
                  setDockSectionCollapsed(layoutId, section.id, !collapsed)
                }
                aria-expanded={!collapsed}
                title={collapsed ? `Open ${section.label}` : `Collapse ${section.label}`}
              >
                <ChevronDown
                  size={14}
                  strokeWidth={2}
                  className="dock-section-chevron"
                />
                <span className="dock-section-copy">
                  <strong>{section.label}</strong>
                  {section.hint ? <span>{section.hint}</span> : null}
                </span>
              </button>

              <div className="dock-section-tools">
                {section.badge ? (
                  <span className="ghost-chip dock-section-badge">{section.badge}</span>
                ) : null}
                <button
                  type="button"
                  className="dock-tool-button"
                  onClick={() => moveSection(section.id, -1)}
                  disabled={index === 0}
                  title={`Move ${section.label} up`}
                >
                  <ArrowUp size={13} strokeWidth={2} />
                </button>
                <button
                  type="button"
                  className="dock-tool-button"
                  onClick={() => moveSection(section.id, 1)}
                  disabled={index === orderedSections.length - 1}
                  title={`Move ${section.label} down`}
                >
                  <ArrowDown size={13} strokeWidth={2} />
                </button>
              </div>
            </div>

            {!collapsed ? (
              <div className="dock-section-body-shell">
                <div className="dock-section-body">{section.children}</div>
              </div>
            ) : null}
          </section>
        );
      })}
    </div>
  );
}
