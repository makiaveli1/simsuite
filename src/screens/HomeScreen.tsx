import { useEffect, useState } from "react";
import {
  Download,
  FolderCog,
  FolderOpen,
  LibraryBig,
  ScanSearch,
  ShieldAlert,
} from "lucide-react";
import { api } from "../lib/api";
import { Workbench } from "../components/layout/Workbench";
import { WorkbenchStage } from "../components/layout/WorkbenchStage";
import { WorkbenchInspector } from "../components/layout/WorkbenchInspector";
import type {
  DetectedLibraryPaths,
  HomeOverview,
  LibrarySettings,
  Screen,
  UserView,
  WatchListFilter,
} from "../lib/types";

interface HomeScreenProps {
  refreshVersion: number;
  settings: LibrarySettings | null;
  onSettingsChange: (settings: LibrarySettings) => Promise<void>;
  onNavigate: (screen: Screen) => void;
  onNavigateWithParams: (screen: Screen, mode?: 'tracked' | 'setup' | 'review', filter?: WatchListFilter) => void;
  onScan: () => Promise<void>;
  isScanning: boolean;
  userView: UserView;
}

export function HomeScreen({
  refreshVersion,
  settings,
  onSettingsChange,
  onNavigate,
  onNavigateWithParams,
  onScan,
  isScanning,
  userView,
}: HomeScreenProps) {
  const [overview, setOverview] = useState<HomeOverview | null>(null);
  const [detectedPaths, setDetectedPaths] = useState<DetectedLibraryPaths | null>(
    null,
  );
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    void api.getHomeOverview().then(setOverview);
  }, [refreshVersion]);

  useEffect(() => {
    void api.detectDefaultLibraryPaths().then(setDetectedPaths);
  }, []);

  async function chooseFolder(kind: "modsPath" | "trayPath" | "downloadsPath") {
    const title =
      kind === "modsPath"
        ? "Choose your Sims Mods folder"
        : kind === "trayPath"
          ? "Choose your Sims Tray folder"
          : "Choose your Downloads folder";
    const picked = await api.pickFolder(title);
    if (!picked || !settings) {
      return;
    }

    setIsSaving(true);
    try {
      await onSettingsChange({ ...settings, [kind]: picked });
    } finally {
      setIsSaving(false);
    }
  }

  async function applyDetectedPaths() {
    if (!detectedPaths || !settings) {
      return;
    }

    setIsSaving(true);
    try {
      await onSettingsChange({
        modsPath: detectedPaths.modsPath ?? settings.modsPath,
        trayPath: detectedPaths.trayPath ?? settings.trayPath,
        downloadsPath: detectedPaths.downloadsPath ?? settings.downloadsPath,
      });
    } finally {
      setIsSaving(false);
    }
  }

  const canScan = Boolean(settings?.modsPath || settings?.trayPath);
  const sourceCount =
    Number(Boolean(settings?.modsPath)) +
    Number(Boolean(settings?.trayPath)) +
    Number(Boolean(settings?.downloadsPath));

  return (
    <Workbench threePanel fullHeight>
      {/* Left rail for navigation - using existing Sidebar from App shell */}
      {/* Central work area */}
      <WorkbenchStage>
        {/* Slim utility strip instead of second page header */}
        <div className="slim-strip">
          <div className="slim-strip-group">
            <span className="health-chip is-good">
              <span className="health-chip-dot"></span>
              {overview?.totalFiles?.toLocaleString() ?? 0} indexed
            </span>
            <span className="health-chip">
              {overview?.modsCount?.toLocaleString() ?? 0} mods
            </span>
            <span className="health-chip">
              {overview?.trayCount?.toLocaleString() ?? 0} tray
            </span>
          </div>
          <div className="slim-strip-group">
            <button
              type="button"
              className="primary-action"
              onClick={() => void onScan()}
              disabled={!canScan || isScanning}
              title="Run a full read-only scan"
            >
              <ScanSearch size={14} strokeWidth={2} />
              {isScanning ? "Scanning..." : "Scan"}
            </button>
          </div>
        </div>

        {/* Command Board: Two fixed columns */}
        <div className="command-board">
          {/* Left column: Do next items */}
          <div className="command-column">
            <div className="command-column-title">Do next</div>
            <button
              type="button"
              className="action-item"
              onClick={() => onNavigate("downloads")}
            >
              <Download size={14} strokeWidth={2} className="action-item-icon" />
              <span className="action-item-label">Inbox</span>
              <span className="action-item-badge">{overview?.downloadsCount ?? 0}</span>
            </button>
            <button
              type="button"
              className="action-item"
              onClick={() => onNavigate("review")}
            >
              <ShieldAlert size={14} strokeWidth={2} className="action-item-icon" />
              <span className="action-item-label">Review</span>
              <span className="action-item-badge">{overview?.reviewCount ?? 0}</span>
            </button>
            <button
              type="button"
              className="action-item"
              onClick={() => onNavigateWithParams("updates", "tracked", "exact_updates")}
            >
              <LibraryBig size={14} strokeWidth={2} className="action-item-icon" />
              <span className="action-item-label">Updates</span>
              <span className="action-item-badge">{overview?.exactUpdateItems ?? 0}</span>
            </button>
            <button
              type="button"
              className="action-item"
              onClick={() => onNavigateWithParams("updates", "setup", "all")}
            >
              <ScanSearch size={14} strokeWidth={2} className="action-item-icon" />
              <span className="action-item-label">Scan setup</span>
              <span className="action-item-badge">{overview?.watchSetupItems ?? 0}</span>
            </button>
          </div>

          {/* Right column: System health chips */}
          <div className="command-column">
            <div className="command-column-title">System health</div>
            <div className="health-chip-group">
              <span className={`health-chip ${overview?.scanNeedsRefresh ? 'is-warn' : 'is-good'}`}>
                <span className="health-chip-dot"></span>
                {overview?.scanNeedsRefresh ? 'Needs refresh' : 'Scan current'}
              </span>
              <span className={`health-chip ${(overview?.unsafeCount ?? 0) > 0 ? 'is-danger' : 'is-good'}`}>
                <span className="health-chip-dot"></span>
                {overview?.unsafeCount ?? 0} risky
              </span>
              <span className={`health-chip ${sourceCount < 3 ? 'is-warn' : 'is-good'}`}>
                <span className="health-chip-dot"></span>
                {sourceCount}/3 folders ready
              </span>
            </div>
          </div>
        </div>

        {/* Compact folder setup section */}
        <div className="folder-setup-compact">
          <div className="folder-setup-header">
            <FolderCog size={14} strokeWidth={2} />
            <span>Folders</span>
            <span className="folder-setup-status">{sourceCount}/3 set</span>
          </div>
          <div className="folder-setup-items">
            <div className="folder-item">
              <span className="folder-label">Mods</span>
              <span className="folder-path">{settings?.modsPath || "Not chosen yet"}</span>
              <button
                type="button"
                className="folder-action secondary-action"
                onClick={() => void chooseFolder("modsPath")}
                disabled={isSaving}
                title="Browse for the Mods folder"
              >
                <FolderOpen size={12} strokeWidth={2} />
                Choose
              </button>
            </div>
            <div className="folder-item">
              <span className="folder-label">Tray</span>
              <span className="folder-path">{settings?.trayPath || "Not chosen yet"}</span>
              <button
                type="button"
                className="folder-action secondary-action"
                onClick={() => void chooseFolder("trayPath")}
                disabled={isSaving}
                title="Browse for the Tray folder"
              >
                <FolderOpen size={12} strokeWidth={2} />
                Choose
              </button>
            </div>
            <div className="folder-item">
              <span className="folder-label">Downloads</span>
              <span className="folder-path">{settings?.downloadsPath || "Not chosen yet"}</span>
              <button
                type="button"
                className="folder-action secondary-action"
                onClick={() => void chooseFolder("downloadsPath")}
                disabled={isSaving}
                title="Browse for the Downloads folder"
              >
                <FolderOpen size={12} strokeWidth={2} />
                Choose
              </button>
            </div>
          </div>
        </div>
      </WorkbenchStage>

      {/* Right inspector panel */}
      <WorkbenchInspector>
        {/* Inspector content can be added here if needed */}
        <div className="inspector-placeholder">
          <p className="inspector-hint">Select an item to see details</p>
        </div>
      </WorkbenchInspector>
    </Workbench>
  );
}

function FolderRow({
  label,
  value,
  onBrowse,
  busy,
}: {
  label: string;
  value: string | null | undefined;
  onBrowse: () => void;
  busy: boolean;
}) {
  return (
    <div className="folder-row" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '0.5rem', padding: '0.25rem 0' }}>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: '0.72rem', fontWeight: 600, color: 'var(--text-soft)' }}>{label}</div>
        <div className="text-path" style={{ fontSize: '0.66rem', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{value || "Not chosen yet"}</div>
      </div>

      <button
        type="button"
        className="secondary-action"
        onClick={onBrowse}
        disabled={busy}
        title={`Browse for the ${label} folder`}
      >
        <FolderOpen size={14} strokeWidth={2} />
        Choose
      </button>
    </div>
  );
}
