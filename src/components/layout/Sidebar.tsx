import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { AnimatePresence, m } from "motion/react";
import {
  BookOpen,
  ChevronDown,
  ChevronUp,
  Copy,
  Fingerprint,
  FolderTree,
  House,
  Inbox,
  ScanSearch,
  ShieldAlert,
  ShieldCheck,
  Shapes,
  SlidersHorizontal,
  Workflow,
} from "lucide-react";
import { BrandMark } from "./BrandMark";
import { ResizableEdgeHandle } from "../ResizableEdgeHandle";
import { useUiPreferences } from "../UiPreferencesContext";
import { hoverLift, tapPress } from "../../lib/motion";
import type { ExperienceMode, Screen } from "../../lib/types";
import {
  experienceModeToLegacyView,
  getExperienceModeProfile,
} from "../../lib/experienceMode";
import { reviewLabel, screenLabel } from "../../lib/uiLanguage";

const NAV_ITEMS: Array<{
  id: Screen;
  label: string;
  icon: LucideIcon;
}> = [
  { id: "home", label: "Home", icon: House },
  { id: "downloads", label: "Inbox", icon: Inbox },
  { id: "library", label: "Library", icon: FolderTree },
  { id: "creatorAudit", label: "Creators", icon: Fingerprint },
  { id: "categoryAudit", label: "Types", icon: Shapes },
  { id: "duplicates", label: "Duplicates", icon: Copy },
  { id: "organize", label: "Organize", icon: Workflow },
  { id: "review", label: "Review", icon: ShieldAlert },
  { id: "settings", label: "Settings", icon: SlidersHorizontal },
];

interface SidebarProps {
  currentScreen: Screen;
  experienceMode: ExperienceMode;
  onNavigate: (screen: Screen) => void;
  onScan: () => void;
  isScanning: boolean;
  onOpenGuide: () => void;
}

export function Sidebar({
  currentScreen,
  experienceMode,
  onNavigate,
  onScan,
  isScanning,
  onOpenGuide,
}: SidebarProps) {
  const { sidebarWidth, setSidebarWidth } = useUiPreferences();
  const userView = experienceModeToLegacyView(experienceMode);
  const modeProfile = getExperienceModeProfile(experienceMode);
  const navById = new Map(NAV_ITEMS.map((item) => [item.id, item]));
  const primaryItems = modeProfile.primaryScreens
    .map((id) => navById.get(id))
    .filter((item): item is (typeof NAV_ITEMS)[number] => Boolean(item));
  const toolItems = modeProfile.toolScreens
    .map((id) => navById.get(id))
    .filter((item): item is (typeof NAV_ITEMS)[number] => Boolean(item));
  const isToolScreenActive = toolItems.some((item) => item.id === currentScreen);
  const [showToolsDrawer, setShowToolsDrawer] = useState(isToolScreenActive);

  useEffect(() => {
    if (experienceMode !== "casual") {
      setShowToolsDrawer(false);
      return;
    }

    if (isToolScreenActive) {
      setShowToolsDrawer(true);
    }
  }, [experienceMode, isToolScreenActive]);

  return (
    <aside className="sidebar-shell">
      <ResizableEdgeHandle
        label="Resize navigation rail"
        value={sidebarWidth}
        min={84}
        max={180}
        onChange={setSidebarWidth}
        side="right"
        className="sidebar-resize-handle"
      />
      <m.button
        type="button"
        className="brand-panel"
        onClick={() => onNavigate("home")}
        title="Go to Home"
        whileHover={hoverLift}
        whileTap={tapPress}
      >
        <div className="brand-mark" aria-hidden="true">
          <BrandMark />
        </div>
        <span className="brand-wordmark">SimSuite</span>
      </m.button>

      <div className="rail-actions">
        <m.button
          type="button"
          className={`rail-action rail-action-primary ${isScanning ? "is-busy" : ""}`}
          onClick={onScan}
          disabled={isScanning}
          title="Run a full library scan"
          whileHover={isScanning ? undefined : hoverLift}
          whileTap={isScanning ? undefined : tapPress}
        >
          <ScanSearch size={16} strokeWidth={2} />
          <span>
            {isScanning
              ? "Scanning..."
              : experienceMode === "casual"
                ? "Scan my CC"
                : experienceMode === "creator"
                  ? "Run scan"
                  : "Scan"}
          </span>
        </m.button>

        <m.button
          type="button"
          className="rail-action"
          onClick={onOpenGuide}
          title="Open the field guide"
          whileHover={hoverLift}
          whileTap={tapPress}
        >
          <BookOpen size={16} strokeWidth={2} />
          <span>Guide</span>
        </m.button>
      </div>

      <nav className="nav-stack" aria-label="Primary">
        {primaryItems.map((item) => (
          <m.button
            key={item.id}
            type="button"
            className={`rail-nav ${currentScreen === item.id ? "is-active" : ""}`}
            onClick={() => onNavigate(item.id)}
            title={item.id === "review" ? reviewLabel(userView) : screenLabel(item.id, userView)}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            <AnimatePresence>
              {currentScreen === item.id ? (
                <m.span
                  layoutId="sidebar-active-panel"
                  className="rail-nav-active-indicator"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                    transition={{ duration: 0.18 }}
                />
              ) : null}
            </AnimatePresence>
            <item.icon size={18} strokeWidth={2} />
            <span>
              {item.id === "review"
                ? reviewLabel(userView)
                : screenLabel(item.id, userView)}
            </span>
          </m.button>
        ))}
      </nav>

      {toolItems.length > 0 && experienceMode === "casual" ? (
        <div className={`sidebar-tools-compact${showToolsDrawer ? " is-open" : ""}`}>
          <m.button
            type="button"
            className={`rail-nav rail-nav-tools-toggle ${isToolScreenActive ? "is-active" : ""}`}
            onClick={() => setShowToolsDrawer((current) => !current)}
            title={showToolsDrawer ? "Hide extra tools" : "Show extra tools"}
            whileHover={hoverLift}
            whileTap={tapPress}
          >
            <Workflow size={18} strokeWidth={2} />
            <span>More</span>
            <span className="rail-nav-tools-chevron" aria-hidden="true">
              {showToolsDrawer ? (
                <ChevronUp size={12} strokeWidth={2} />
              ) : (
                <ChevronDown size={12} strokeWidth={2} />
              )}
            </span>
          </m.button>

          <AnimatePresence initial={false}>
            {showToolsDrawer ? (
              <m.div
                className="sidebar-tools-drawer"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                transition={{ duration: 0.18 }}
              >
                <span className="sidebar-group-label">More tools</span>
                <nav className="nav-stack nav-stack-secondary" aria-label="More tools">
                  {toolItems.map((item) => (
                    <m.button
                      key={item.id}
                      type="button"
                      className={`rail-nav rail-nav-secondary ${currentScreen === item.id ? "is-active" : ""}`}
                      onClick={() => onNavigate(item.id)}
                      title={screenLabel(item.id, userView)}
                      whileHover={hoverLift}
                      whileTap={tapPress}
                    >
                      <AnimatePresence>
                        {currentScreen === item.id ? (
                          <m.span
                            layoutId="sidebar-active-panel"
                            className="rail-nav-active-indicator"
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            transition={{ duration: 0.18 }}
                          />
                        ) : null}
                      </AnimatePresence>
                      <item.icon size={18} strokeWidth={2} />
                      <span>{screenLabel(item.id, userView)}</span>
                    </m.button>
                  ))}
                </nav>
              </m.div>
            ) : null}
          </AnimatePresence>
        </div>
      ) : toolItems.length > 0 ? (
        <div className="sidebar-nav-group">
          <span className="sidebar-group-label">Tools</span>
          <nav className="nav-stack nav-stack-secondary" aria-label="Tools">
            {toolItems.map((item) => (
              <m.button
                key={item.id}
                type="button"
                className={`rail-nav rail-nav-secondary ${currentScreen === item.id ? "is-active" : ""}`}
                onClick={() => onNavigate(item.id)}
                title={
                  item.id === "review"
                    ? reviewLabel(userView)
                    : screenLabel(item.id, userView)
                }
                whileHover={hoverLift}
                whileTap={tapPress}
              >
                <AnimatePresence>
                  {currentScreen === item.id ? (
                    <m.span
                      layoutId="sidebar-active-panel"
                      className="rail-nav-active-indicator"
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.18 }}
                    />
                  ) : null}
                </AnimatePresence>
                <item.icon size={18} strokeWidth={2} />
                <span>{screenLabel(item.id, userView)}</span>
              </m.button>
            ))}
          </nav>
        </div>
      ) : null}

      <div className="sidebar-footer">
        <div className="sidebar-mode-note">
          <span className="section-label">{modeProfile.label}</span>
          <p>{modeProfile.workspaceSummary}</p>
        </div>
        <div className="sidebar-footer-row">
          <ShieldCheck size={14} strokeWidth={2} />
          <span>Validated moves</span>
        </div>
        <div className="sidebar-footer-row muted">
          <span>Undo always via snapshots</span>
        </div>
      </div>
    </aside>
  );
}
