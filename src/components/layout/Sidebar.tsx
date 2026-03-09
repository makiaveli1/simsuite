import type { LucideIcon } from "lucide-react";
import { AnimatePresence, m } from "motion/react";
import {
  BookOpen,
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
import type { Screen, UserView } from "../../lib/types";

const NAV_ITEMS: Array<{
  id: Screen;
  label: string;
  icon: LucideIcon;
}> = [
  { id: "home", label: "Home", icon: House },
  { id: "downloads", label: "Inbox", icon: Inbox },
  { id: "library", label: "Library", icon: FolderTree },
  { id: "creatorAudit", label: "Creators", icon: Fingerprint },
  { id: "categoryAudit", label: "Categories", icon: Shapes },
  { id: "duplicates", label: "Duplicates", icon: Copy },
  { id: "organize", label: "Organize", icon: Workflow },
  { id: "review", label: "Review", icon: ShieldAlert },
  { id: "settings", label: "Settings", icon: SlidersHorizontal },
];

interface SidebarProps {
  currentScreen: Screen;
  userView: UserView;
  onNavigate: (screen: Screen) => void;
  onScan: () => void;
  isScanning: boolean;
  onOpenGuide: () => void;
}

const VIEW_LABELS: Record<Screen, Partial<Record<UserView, string>>> = {
  home: {},
  downloads: {
    beginner: "New files",
  },
  library: {
    beginner: "My CC",
  },
  creatorAudit: {
    beginner: "Creator names",
  },
  categoryAudit: {
    beginner: "Mod types",
  },
  duplicates: {
    beginner: "Same file?",
  },
  organize: {
    beginner: "Tidy up",
  },
  review: {
    beginner: "Needs help",
  },
  settings: {},
};

export function Sidebar({
  currentScreen,
  userView,
  onNavigate,
  onScan,
  isScanning,
  onOpenGuide,
}: SidebarProps) {
  const { sidebarWidth, setSidebarWidth } = useUiPreferences();

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
            {isScanning ? "Scanning..." : userView === "beginner" ? "Scan CC" : "Scan"}
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
        {NAV_ITEMS.map((item) => (
          <m.button
            key={item.id}
            type="button"
            className={`rail-nav ${currentScreen === item.id ? "is-active" : ""}`}
            onClick={() => onNavigate(item.id)}
            title={VIEW_LABELS[item.id][userView] ?? item.label}
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
            <span>{VIEW_LABELS[item.id][userView] ?? item.label}</span>
          </m.button>
        ))}
      </nav>

      <div className="sidebar-footer">
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
