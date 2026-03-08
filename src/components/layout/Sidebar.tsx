import type { LucideIcon } from "lucide-react";
import {
  BookOpen,
  Copy,
  Fingerprint,
  FolderTree,
  House,
  ScanSearch,
  ShieldAlert,
  ShieldCheck,
  Shapes,
  Workflow,
} from "lucide-react";
import { BrandMark } from "./BrandMark";
import { ResizableEdgeHandle } from "../ResizableEdgeHandle";
import { useUiPreferences } from "../UiPreferencesContext";
import type { Screen, UserView } from "../../lib/types";

const NAV_ITEMS: Array<{
  id: Screen;
  label: string;
  icon: LucideIcon;
}> = [
  { id: "home", label: "Home", icon: House },
  { id: "library", label: "Library", icon: FolderTree },
  { id: "creatorAudit", label: "Creators", icon: Fingerprint },
  { id: "categoryAudit", label: "Categories", icon: Shapes },
  { id: "duplicates", label: "Duplicates", icon: Copy },
  { id: "organize", label: "Organize", icon: Workflow },
  { id: "review", label: "Review", icon: ShieldAlert },
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
      <button
        type="button"
        className="brand-panel"
        onClick={() => onNavigate("home")}
        title="Go to Home"
      >
        <div className="brand-mark" aria-hidden="true">
          <BrandMark />
        </div>
        <span className="brand-wordmark">SimSuite</span>
      </button>

      <div className="rail-actions">
        <button
          type="button"
          className={`rail-action rail-action-primary ${isScanning ? "is-busy" : ""}`}
          onClick={onScan}
          disabled={isScanning}
          title="Run a full library scan"
        >
          <ScanSearch size={16} strokeWidth={2} />
          <span>
            {isScanning ? "Scanning..." : userView === "beginner" ? "Scan CC" : "Scan"}
          </span>
        </button>

        <button
          type="button"
          className="rail-action"
          onClick={onOpenGuide}
          title="Open the field guide"
        >
          <BookOpen size={16} strokeWidth={2} />
          <span>Guide</span>
        </button>
      </div>

      <nav className="nav-stack" aria-label="Primary">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`rail-nav ${currentScreen === item.id ? "is-active" : ""}`}
            onClick={() => onNavigate(item.id)}
            title={VIEW_LABELS[item.id][userView] ?? item.label}
          >
            <item.icon size={18} strokeWidth={2} />
            <span>{VIEW_LABELS[item.id][userView] ?? item.label}</span>
          </button>
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
