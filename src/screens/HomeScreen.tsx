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
  const reviewActionLabel = userView === "beginner" ? "Needs review" : "Review";
  const setupActionLabel = userView === "beginner" ? "Set pages" : "Track setup";
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
      label: reviewActionLabel,
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
      label: setupActionLabel,
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
      note: "Clear page matches with a newer version waiting.",
      onClick: () => onNavigateWithParams("updates", "tracked", "exact_updates"),
    },
    {
      label: "Possible updates",
      value: String(overview?.possibleUpdateItems ?? 0),
      note: "Changed pages that still need one more careful look.",
      onClick: () => onNavigateWithParams("updates", "tracked", "possible_updates"),
    },
    {
      label: "Unclear watch results",
      value: String(overview?.unknownWatchItems ?? 0),
      note: "Saved pages that still do not give a clean answer.",
      onClick: () => onNavigateWithParams("updates", "tracked", "unclear"),
    },
    {
      label: userView === "beginner" ? "Pages to save" : "Need source setup",
      value: String(overview?.watchSetupItems ?? 0),
      note: "Installed files that still need one saved page first.",
      onClick: () => onNavigateWithParams("updates", "setup", "all"),
    },
    {
      label: userView === "beginner" ? "Needs follow-up" : "Watch review",
      value: String(overview?.watchReviewItems ?? 0),
      note: "Reminder-only or provider-backed pages that stay cautious.",
      onClick: () => onNavigateWithParams("updates", "review", "all"),
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
  const visibleSystemRows =
    userView === "power"
      ? systemRows
      : userView === "standard"
        ? systemRows.slice(0, 5)
        : systemRows.slice(0, 4);
  const healthSnapshot = [
    {
      label: userView === "beginner" ? "Scripts" : "Script mods",
      value: (overview?.scriptModsCount ?? 0).toLocaleString(),
    },
    {
      label: "Duplicates",
      value: (overview?.duplicatesCount ?? 0).toLocaleString(),
    },
    {
      label: "Creators",
      value: (overview?.creatorCount ?? 0).toLocaleString(),
    },
    {
      label: userView === "power" ? "Bundles" : "Update watch",
      value:
        userView === "power"
          ? (overview?.bundlesCount ?? 0).toLocaleString()
          : (overview?.exactUpdateItems ?? 0).toLocaleString(),
    },
  ];
  const primaryActionMatches =
    busiestAction && primaryAction.title === busiestAction.label;

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
          <section className="panel-card home-priority-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Today board</p>
                <h2>{userView === "beginner" ? "Best next move" : "Keep the day moving"}</h2>
              </div>
              <span className="ghost-chip">
                {nextActions.reduce((total, item) => total + item.count, 0)} open
              </span>
            </div>

            <div className="home-priority-card">
              <div className="home-priority-copy">
                <span className="section-label">Best next move</span>
                <strong>{primaryAction.title}</strong>
                <p>{primaryAction.body}</p>
              </div>
              <button
                type="button"
                className="primary-action"
                onClick={primaryAction.onClick}
                disabled={primaryAction.disabled}
              >
                {primaryAction.cta}
              </button>
            </div>

            <div className="home-command-list">
              {nextActions.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className={`action-item home-action-item ${
                    primaryActionMatches && primaryAction.title === item.label
                      ? "is-active"
                      : ""
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
                <h2>{userView === "beginner" ? "What SimSuite knows" : "Truth before action"}</h2>
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
            <div className="summary-matrix home-health-summary">
              {healthSnapshot.map((item) => (
                <HomeStatTile key={item.label} label={item.label} value={item.value} />
              ))}
            </div>
            <p className="home-panel-note">
              {overview?.scanNeedsRefresh
                ? "A fresh scan should happen before you trust older library facts."
                : "The scan truth, watch lanes, and safety signals are all reading from the same current pass."}
            </p>
          </section>

          <section className="panel-card home-watch-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Tracked pages</p>
                <h2>{userView === "beginner" ? "Update follow-up" : "Watch lanes"}</h2>
              </div>
              <span className="ghost-chip">
                {(overview?.exactUpdateItems ?? 0) +
                  (overview?.possibleUpdateItems ?? 0) +
                  (overview?.unknownWatchItems ?? 0) +
                  (overview?.watchSetupItems ?? 0) +
                  (overview?.watchReviewItems ?? 0)}{" "}
                open
              </span>
            </div>

            <div className="home-watch-list">
              {watchRows.map((row) => (
                <button
                  key={row.label}
                  type="button"
                  className="home-watch-row"
                  onClick={row.onClick}
                >
                  <span className="home-watch-copy">
                    <span className="action-item-label">{row.label}</span>
                    <span className="home-action-note">{row.note}</span>
                  </span>
                  <span className="action-item-badge">{row.value}</span>
                </button>
              ))}
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
            <p className="home-panel-note">
              {sourceCount < 3
                ? "Finish the missing folder links first so scans, downloads watching, and tray checks all land in the right place."
                : "These roots are ready, so scans and follow-up work can stay inside one steady desktop flow."}
            </p>
          </section>
        </div>
      </WorkbenchStage>

      <WorkbenchInspector className="home-inspector-shell" ariaLabel="Home details">
        <div className="home-inspector">
          <div className="detail-header">
            <div>
              <p className="eyebrow">Today</p>
              <h2>{userView === "beginner" ? "What needs you first" : "Command board"}</h2>
            </div>
            <span className="ghost-chip">Home</span>
          </div>

          <div className="home-focus-card">
            <div className="home-focus-copy">
              <span className="section-label">Best next move</span>
              <strong>{primaryAction.title}</strong>
              <p>{primaryAction.body}</p>
            </div>
            <button
              type="button"
              className="primary-action"
              onClick={primaryAction.onClick}
              disabled={primaryAction.disabled}
            >
              {primaryAction.cta}
            </button>
          </div>

          <div className="summary-matrix home-inspector-grid">
            {nextActions.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`summary-stat home-inspector-link ${
                  primaryActionMatches && primaryAction.title === item.label
                    ? "is-active"
                    : ""
                }`}
                onClick={item.onClick}
              >
                <span>{item.label}</span>
                <strong>{item.count}</strong>
              </button>
            ))}
          </div>

          <div className="detail-block">
            <div className="section-label">System truth</div>
            <div className="detail-list">
              {visibleSystemRows.map((row) => (
                <HomeDetailRow key={row.label} label={row.label} value={row.value} />
              ))}
            </div>
          </div>

          <div className="detail-block">
            <div className="section-label">Watch pressure</div>
            <div className="detail-list">
              {watchRows.slice(0, 4).map((row) => (
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

function HomeStatTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="summary-stat home-stat-tile">
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
