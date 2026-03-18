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

  async function chooseFirstMissingFolder() {
    if (!settings) {
      return;
    }

    if (!settings.modsPath) {
      await chooseFolder("modsPath");
      return;
    }

    if (!settings.trayPath) {
      await chooseFolder("trayPath");
      return;
    }

    if (!settings.downloadsPath) {
      await chooseFolder("downloadsPath");
    }
  }

  const canScan = Boolean(settings?.modsPath || settings?.trayPath);
  const sourceCount =
    Number(Boolean(settings?.modsPath)) +
    Number(Boolean(settings?.trayPath)) +
    Number(Boolean(settings?.downloadsPath));
  const hasDetectedPathSuggestion = Boolean(
    detectedPaths &&
      ((!settings?.modsPath && detectedPaths.modsPath) ||
        (!settings?.trayPath && detectedPaths.trayPath) ||
        (!settings?.downloadsPath && detectedPaths.downloadsPath)),
  );
  const nextActions = [
    {
      id: "inbox",
      label: "Inbox",
      description: "Fresh downloads and staged batches waiting for a safe hand-off.",
      count: overview?.downloadsCount ?? 0,
      icon: <Download size={14} strokeWidth={2} className="action-item-icon" />,
      onClick: () => onNavigate("downloads"),
    },
    {
      id: "review",
      label: "Review",
      description: "Files that still need a human check before SimSuite can move them.",
      count: overview?.reviewCount ?? 0,
      icon: <ShieldAlert size={14} strokeWidth={2} className="action-item-icon" />,
      onClick: () => onNavigate("review"),
    },
    {
      id: "updates",
      label: "Updates",
      description: "Tracked pages that already look like confirmed update matches.",
      count: overview?.exactUpdateItems ?? 0,
      icon: <LibraryBig size={14} strokeWidth={2} className="action-item-icon" />,
      onClick: () =>
        onNavigateWithParams("updates", "tracked", "exact_updates"),
    },
    {
      id: "setup",
      label: "Track setup",
      description: "Library items that still need a page saved before SimSuite can watch them.",
      count: overview?.watchSetupItems ?? 0,
      icon: <ScanSearch size={14} strokeWidth={2} className="action-item-icon" />,
      onClick: () => onNavigateWithParams("updates", "setup", "all"),
    },
  ];
  const busiestAction = nextActions.find((item) => item.count > 0) ?? null;

  const primaryAction =
    sourceCount < 3
      ? {
          title: "Finish folder setup",
          body:
            "Pick the last missing folders so scans, inbox watching, and tray checks all have the right places to read from.",
          cta:
            hasDetectedPathSuggestion && !isSaving
              ? "Use detected folders"
              : "Choose next folder",
          disabled: isSaving,
          onClick: () =>
            hasDetectedPathSuggestion && !isSaving
              ? void applyDetectedPaths()
              : void chooseFirstMissingFolder(),
        }
      : (busiestAction
          ? {
              title: busiestAction.label,
              body: busiestAction.description,
              cta: `Open ${busiestAction.label}`,
              disabled: false,
              onClick: busiestAction.onClick,
            }
          : overview?.scanNeedsRefresh
            ? {
                title: "Refresh library facts",
                body:
                  "The stored library picture is older than the current scan rules, so a fresh scan is the safest next move.",
                cta: isScanning ? "Scanning..." : "Run scan",
                disabled: isScanning || !canScan,
                onClick: () => void onScan(),
              }
            : {
                title: "Everything looks steady",
                body:
                  "There is no hot queue right now, so this is a good time to skim the library or tune your tracked pages.",
                cta: "Browse library",
                disabled: false,
                onClick: () => onNavigate("library"),
              });

  const watchRows = [
    {
      label: "Confirmed updates",
      value: String(overview?.exactUpdateItems ?? 0),
    },
    {
      label: "Possible updates",
      value: String(overview?.possibleUpdateItems ?? 0),
    },
    {
      label: "Unclear watch results",
      value: String(overview?.unknownWatchItems ?? 0),
    },
    {
      label: "Need source setup",
      value: String(overview?.watchSetupItems ?? 0),
    },
  ];
  const systemRows = [
    {
      label: "Last scan",
      value: formatTimestamp(overview?.lastScanAt),
    },
    {
      label: "Library facts",
      value: overview?.scanNeedsRefresh ? "Need refresh" : "Current",
    },
    {
      label: "Safety mode",
      value: overview?.readOnlyMode ? "Read-only on" : "Read-only off",
    },
    {
      label: "Duplicates found",
      value: (overview?.duplicatesCount ?? 0).toLocaleString(),
    },
    {
      label: "Creators seen",
      value: (overview?.creatorCount ?? 0).toLocaleString(),
    },
    {
      label: "Bundles seen",
      value: (overview?.bundlesCount ?? 0).toLocaleString(),
    },
  ];

  return (
    <Workbench threePanel fullHeight className="home-workbench">
      <WorkbenchStage className="home-stage">
        <div className="slim-strip home-slim-strip">
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

        <div className="home-stage-grid">
          <section className="panel-card home-command-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Do next</p>
                <h2>Keep the day moving</h2>
              </div>
              <span className="ghost-chip">
                {nextActions.reduce((total, item) => total + item.count, 0)} open
              </span>
            </div>
            <div className="home-command-list">
              {nextActions.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className={`action-item home-action-item ${
                    primaryAction.title === item.label ? "is-active" : ""
                  }`}
                  onClick={item.onClick}
                >
                  {item.icon}
                  <span className="home-action-copy">
                    <span className="action-item-label">{item.label}</span>
                    <span className="home-action-note">{item.description}</span>
                  </span>
                  <span className="action-item-badge">{item.count}</span>
                </button>
              ))}
            </div>
          </section>

          <section className="panel-card home-health-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">System health</p>
                <h2>Truth before action</h2>
              </div>
            </div>
            <div className="health-chip-group home-health-chips">
              <span
                className={`health-chip ${
                  overview?.scanNeedsRefresh ? "is-warn" : "is-good"
                }`}
              >
                <span className="health-chip-dot"></span>
                {overview?.scanNeedsRefresh ? "Scan needs refresh" : "Scan current"}
              </span>
              <span
                className={`health-chip ${
                  (overview?.unsafeCount ?? 0) > 0 ? "is-danger" : "is-good"
                }`}
              >
                <span className="health-chip-dot"></span>
                {overview?.unsafeCount ?? 0} risky
              </span>
              <span
                className={`health-chip ${
                  sourceCount < 3 ? "is-warn" : "is-good"
                }`}
              >
                <span className="health-chip-dot"></span>
                {sourceCount}/3 folders ready
              </span>
            </div>
            <div className="detail-list home-health-ledger">
              <HomeDetailRow
                label="Possible updates"
                value={(overview?.possibleUpdateItems ?? 0).toLocaleString()}
              />
              <HomeDetailRow
                label="Unclear watch results"
                value={(overview?.unknownWatchItems ?? 0).toLocaleString()}
              />
              <HomeDetailRow
                label="Script mods"
                value={(overview?.scriptModsCount ?? 0).toLocaleString()}
              />
              <HomeDetailRow
                label="Duplicates"
                value={(overview?.duplicatesCount ?? 0).toLocaleString()}
              />
            </div>
          </section>

          <section className="panel-card home-folders-panel">
            <div className="panel-heading">
              <div className="home-folders-heading">
                <p className="eyebrow">Folders</p>
                <h2>Library roots</h2>
              </div>
              <div className="home-folders-toolbar">
                <span className="ghost-chip">{sourceCount}/3 set</span>
                {hasDetectedPathSuggestion ? (
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={() => void applyDetectedPaths()}
                    disabled={isSaving}
                  >
                    <FolderCog size={14} strokeWidth={2} />
                    Use detected folders
                  </button>
                ) : null}
              </div>
            </div>
            <div className="folder-setup-items home-folder-grid">
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
                <span className="folder-path">
                  {settings?.downloadsPath || "Not chosen yet"}
                </span>
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
          </section>
        </div>
      </WorkbenchStage>

      <WorkbenchInspector className="home-inspector-shell" ariaLabel="Home details">
        <div className="home-inspector">
          <div className="detail-header">
            <div>
              <p className="eyebrow">Control room</p>
              <h2>{primaryAction.title}</h2>
            </div>
            <span className="ghost-chip">Home</span>
          </div>

          <div className="home-focus-card">
            <p>{primaryAction.body}</p>
            <button
              type="button"
              className="primary-action"
              onClick={primaryAction.onClick}
              disabled={primaryAction.disabled}
            >
              {primaryAction.cta}
            </button>
          </div>

          <div className="detail-block">
            <div className="section-label">Update watch</div>
            <div className="detail-list">
              {watchRows.map((row) => (
                <HomeDetailRow key={row.label} label={row.label} value={row.value} />
              ))}
            </div>
          </div>

          <div className="detail-block">
            <div className="section-label">System truth</div>
            <div className="detail-list">
              {systemRows.map((row) => (
                <HomeDetailRow key={row.label} label={row.label} value={row.value} />
              ))}
            </div>
          </div>
        </div>
      </WorkbenchInspector>
    </Workbench>
  );
}

function HomeDetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatTimestamp(value: string | null | undefined) {
  if (!value) {
    return "Not scanned yet";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
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
