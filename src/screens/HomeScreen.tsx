import { useEffect, useState } from "react";
import { m } from "motion/react";
import type { LucideIcon } from "lucide-react";
import {
  Download,
  FolderCog,
  FolderOpen,
  FolderTree,
  House,
  LibraryBig,
  ScanSearch,
  ShieldAlert,
  TriangleAlert,
  Workflow,
} from "lucide-react";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { hoverLift, stagedListItem, tapPress } from "../lib/motion";
import { reviewLabel, screenHelperLine, screenLabel } from "../lib/uiLanguage";
import type {
  DetectedLibraryPaths,
  HomeOverview,
  LibrarySettings,
  Screen,
  UserView,
} from "../lib/types";

interface HomeScreenProps {
  refreshVersion: number;
  settings: LibrarySettings | null;
  onSettingsChange: (settings: LibrarySettings) => Promise<void>;
  onNavigate: (screen: Screen) => void;
  onScan: () => Promise<void>;
  isScanning: boolean;
  userView: UserView;
}

export function HomeScreen({
  refreshVersion,
  settings,
  onSettingsChange,
  onNavigate,
  onScan,
  isScanning,
  userView,
}: HomeScreenProps) {
  const {
    homePrimaryWidth,
    setHomePrimaryWidth,
    homeSecondaryWidth,
    setHomeSecondaryWidth,
  } = useUiPreferences();
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
  const metricItems =
    userView === "beginner"
      ? [
          {
            label: "Checked",
            value: overview?.totalFiles ?? 0,
            icon: FolderTree,
          },
          {
            label: "Inbox",
            value: overview?.downloadsCount ?? 0,
            icon: Download,
            onClick: () => onNavigate("downloads"),
          },
          {
            label: "Needs review",
            value: overview?.reviewCount ?? 0,
            icon: ShieldAlert,
            tone: "warn" as const,
            onClick: () => onNavigate("review"),
          },
          {
            label: "CC & Mods",
            value: overview?.modsCount ?? 0,
            icon: LibraryBig,
          },
        ]
      : [
          {
            label: "Indexed",
            value: overview?.totalFiles ?? 0,
            icon: FolderTree,
          },
          {
            label: "Mods",
            value: overview?.modsCount ?? 0,
            icon: LibraryBig,
          },
          {
            label: "Tray",
            value: overview?.trayCount ?? 0,
            icon: House,
          },
          {
            label: "Inbox",
            value: overview?.downloadsCount ?? 0,
            icon: Download,
            onClick: () => onNavigate("downloads"),
          },
          {
            label: "Review",
            value: overview?.reviewCount ?? 0,
            icon: ShieldAlert,
            tone: "warn" as const,
            onClick: () => onNavigate("review"),
          },
          {
            label: "Unsafe",
            value: overview?.unsafeCount ?? 0,
            icon: TriangleAlert,
            tone: "danger" as const,
            onClick: () => onNavigate("library"),
          },
          {
            label: userView === "power" ? "Scripts" : "Creators",
            value:
              userView === "power"
                ? overview?.scriptModsCount ?? 0
                : overview?.creatorCount ?? 0,
            icon: userView === "power" ? FolderCog : Workflow,
          },
          ...(userView === "power"
            ? [
                {
                  label: "Duplicates",
                  value: overview?.duplicatesCount ?? 0,
                  icon: Workflow,
                },
                {
                  label: "Creators",
                  value: overview?.creatorCount ?? 0,
                  icon: Workflow,
                },
              ]
            : []),
        ];
  const stateRows =
    userView === "beginner"
      ? [
          { label: "Moves", value: "Ask first" },
          {
            label: "Last check",
            value: overview?.lastScanAt
              ? new Date(overview.lastScanAt).toLocaleString()
              : "Not scanned",
          },
          { label: "Folders ready", value: `${sourceCount} / 3` },
        ]
      : [
          { label: "Mode", value: "Approval-first" },
          { label: "Moves", value: "Validator + snapshot" },
          {
            label: "Last scan",
            value: overview?.lastScanAt
              ? new Date(overview.lastScanAt).toLocaleString()
              : "Not scanned",
          },
          { label: "Roots online", value: `${sourceCount} / 3` },
          ...(userView === "power"
            ? [
                {
                  label: "Read only",
                  value: overview?.readOnlyMode ? "Yes" : "No",
                },
                {
                  label: "Bundles",
                  value: `${overview?.bundlesCount ?? 0}`,
                },
              ]
            : []),
        ];

  return (
    <section className="screen-shell">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Start here" : "Control"}</p>
          <div className="screen-title-row">
            <House size={18} strokeWidth={2} />
            <h1>Home</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("home", userView)}</p>
        </div>
        <div className="header-actions">
          {detectedPaths?.modsPath || detectedPaths?.trayPath ? (
            <button
              type="button"
              className="secondary-action"
              onClick={() => void applyDetectedPaths()}
              disabled={isSaving}
              title="Use the detected Sims 4 Mods and Tray folders"
            >
              <FolderCog size={14} strokeWidth={2} />
              {userView === "beginner" ? "Use found folders" : "Use detected"}
            </button>
          ) : null}
          <button
            type="button"
            className="primary-action"
            onClick={() => void onScan()}
            disabled={!canScan || isScanning}
            title="Run a full read-only scan"
          >
            <ScanSearch size={14} strokeWidth={2} />
            {isScanning ? "Scanning..." : userView === "beginner" ? "Scan my CC" : "Scan"}
          </button>
        </div>
      </div>

      <div className="metrics-grid">
        {metricItems.map((item, index) => (
          <MetricCard
            key={item.label}
            index={index}
            label={item.label}
            value={item.value}
            icon={item.icon}
            tone={item.tone}
            onClick={item.onClick}
          />
        ))}
      </div>

      <div className="home-layout">
        <ResizableEdgeHandle
          label="Resize left home panel"
          value={homePrimaryWidth}
          min={280}
          max={520}
          onChange={setHomePrimaryWidth}
          side="right"
          className="layout-resize-handle home-layout-handle"
        />
        <ResizableEdgeHandle
          label="Resize center home panel"
          value={homePrimaryWidth + homeSecondaryWidth}
          min={540}
          max={900}
          onChange={(nextTotal) => {
            setHomeSecondaryWidth(Math.max(240, Math.min(460, nextTotal - homePrimaryWidth)));
          }}
          side="right"
          className="layout-resize-handle home-secondary-handle"
        />
        <div className="panel-card">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Roots</p>
              <h2>{userView === "beginner" ? "Game folders" : "Library folders"}</h2>
            </div>
            <span className="ghost-chip">{sourceCount}/3 set</span>
          </div>

          <div className="folder-list">
            <FolderRow
              label="Mods"
              value={settings?.modsPath}
              onBrowse={() => void chooseFolder("modsPath")}
              busy={isSaving}
            />
            <FolderRow
              label="Tray"
              value={settings?.trayPath}
              onBrowse={() => void chooseFolder("trayPath")}
              busy={isSaving}
            />
            <FolderRow
              label="Downloads"
              value={settings?.downloadsPath}
              onBrowse={() => void chooseFolder("downloadsPath")}
              busy={isSaving}
            />
          </div>
        </div>

        <div className="panel-card">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Shortcuts</p>
              <h2>{userView === "beginner" ? "What to do next" : "Primary actions"}</h2>
            </div>
          </div>

          <div className="action-grid">
            <ActionCard
              index={0}
              icon={LibraryBig}
              label={screenLabel("library", userView)}
              title="Open the indexed file explorer"
              onClick={() => onNavigate("library")}
            />
            <ActionCard
              index={1}
              icon={Download}
              label={screenLabel("downloads", userView)}
              title="Open the downloads intake workspace"
              onClick={() => onNavigate("downloads")}
            />
            <ActionCard
              index={2}
              icon={ShieldAlert}
              label={reviewLabel(userView)}
              title="Open the review queue"
              onClick={() => onNavigate("review")}
            />
            {userView !== "beginner" ? (
              <ActionCard
                index={3}
                icon={Workflow}
                label="Organize"
                title="Open rule previews and snapshots"
                onClick={() => onNavigate("organize")}
              />
            ) : null}
            <ActionCard
              index={userView !== "beginner" ? 4 : 3}
              icon={ScanSearch}
              label={
                isScanning
                  ? "Scanning..."
                  : userView === "beginner"
                    ? "Scan my CC"
                    : "Scan now"
              }
              title="Run a full read-only scan"
              onClick={() => void onScan()}
              disabled={!canScan || isScanning}
              accent
            />
          </div>
        </div>

        <div className="panel-card">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">State</p>
              <h2>{userView === "beginner" ? "Status" : "Station"}</h2>
            </div>
          </div>

          <div className="system-ledger">
            {stateRows.map((row) => (
              <LedgerRow key={row.label} label={row.label} value={row.value} />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

function MetricCard({
  index,
  label,
  value,
  icon: Icon,
  tone,
  onClick,
}: {
  index: number;
  label: string;
  value: number;
  icon: LucideIcon;
  tone?: "warn" | "danger";
  onClick?: () => void;
}) {
  return (
    <m.button
      type="button"
      className={`metric-card ${tone ? `metric-card-${tone}` : ""} ${
        onClick ? "is-clickable" : ""
      }`}
      onClick={onClick}
      disabled={!onClick}
      title={label}
      whileHover={onClick ? hoverLift : undefined}
      whileTap={onClick ? tapPress : undefined}
      {...stagedListItem(index)}
    >
      <span className="metric-card-head">
        <Icon size={14} strokeWidth={2} />
        <span className="metric-label">{label}</span>
      </span>
      <strong className="metric-value">{value.toLocaleString()}</strong>
    </m.button>
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
    <div className="folder-row">
      <div className="folder-main">
        <div className="folder-title">{label}</div>
        <div className="folder-path">{value || "Not chosen yet"}</div>
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

function ActionCard({
  index,
  icon: Icon,
  label,
  title,
  onClick,
  disabled,
  accent,
}: {
  index: number;
  icon: LucideIcon;
  label: string;
  title: string;
  onClick: () => void;
  disabled?: boolean;
  accent?: boolean;
}) {
  return (
    <m.button
      type="button"
      className={`action-card ${accent ? "action-card-accent" : ""}`}
      onClick={onClick}
      disabled={disabled}
      title={title}
      whileHover={!disabled ? hoverLift : undefined}
      whileTap={!disabled ? tapPress : undefined}
      {...stagedListItem(index)}
    >
      <Icon size={16} strokeWidth={2} />
      <strong>{label}</strong>
    </m.button>
  );
}

function LedgerRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="ledger-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
