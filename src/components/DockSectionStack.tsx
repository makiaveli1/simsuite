import type { ReactNode } from "react";
import { AnimatePresence, m } from "motion/react";
import { ArrowDown, ArrowUp, ChevronDown, RotateCcw } from "lucide-react";
import { hoverLift, panelSpring, tapPress } from "../lib/motion";
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
          <m.button
            type="button"
            className="dock-stack-reset"
            onClick={() => resetDockSectionLayout(layoutId)}
            title={resetLabel}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            <RotateCcw size={13} strokeWidth={2} />
            Reset
          </m.button>
        </div>
      ) : null}

      {orderedSections.map((section, index) => {
        const collapsed = layout.collapsed[section.id] ?? false;

        return (
          <m.section
            key={section.id}
            className={`dock-section${collapsed ? " is-collapsed" : ""}`}
            layout
            transition={panelSpring}
          >
            <div className="dock-section-header">
              <m.button
                type="button"
                className="dock-section-toggle"
                onClick={() =>
                  setDockSectionCollapsed(layoutId, section.id, !collapsed)
                }
                aria-expanded={!collapsed}
                title={collapsed ? `Open ${section.label}` : `Collapse ${section.label}`}
                whileHover={hoverLift}
                whileTap={tapPress}
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
              </m.button>

              <div className="dock-section-tools">
                {section.badge ? (
                  <span className="ghost-chip dock-section-badge">{section.badge}</span>
                ) : null}
                <m.button
                  type="button"
                  className="dock-tool-button"
                  onClick={() => moveSection(section.id, -1)}
                  disabled={index === 0}
                  title={`Move ${section.label} up`}
                  whileHover={index === 0 ? undefined : hoverLift}
                  whileTap={index === 0 ? undefined : tapPress}
                >
                  <ArrowUp size={13} strokeWidth={2} />
                </m.button>
                <m.button
                  type="button"
                  className="dock-tool-button"
                  onClick={() => moveSection(section.id, 1)}
                  disabled={index === orderedSections.length - 1}
                  title={`Move ${section.label} down`}
                  whileHover={
                    index === orderedSections.length - 1 ? undefined : hoverLift
                  }
                  whileTap={
                    index === orderedSections.length - 1 ? undefined : tapPress
                  }
                >
                  <ArrowDown size={13} strokeWidth={2} />
                </m.button>
              </div>
            </div>

            <AnimatePresence initial={false}>
              {!collapsed ? (
                <m.div
                  key="body"
                  className="dock-section-body-shell"
                  initial={{ height: 0, opacity: 0 }}
                  animate={{ height: "auto", opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={panelSpring}
                >
                  <div className="dock-section-body">{section.children}</div>
                </m.div>
              ) : null}
            </AnimatePresence>
          </m.section>
        );
      })}
    </div>
  );
}
