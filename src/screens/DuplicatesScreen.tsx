import { useEffect, useMemo, useState } from "react";
import { m } from "motion/react";
import { Copy, RefreshCw, Search, SearchX } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
import { LayoutPresetBar } from "../components/LayoutPresetBar";
import { ResizableEdgeHandle } from "../components/ResizableEdgeHandle";
import { ResizableDetailPanel } from "../components/ResizableDetailPanel";
import { StatePanel } from "../components/StatePanel";
import { useUiPreferences } from "../components/UiPreferencesContext";
import { api } from "../lib/api";
import { rowHover, rowPress, stagedListItem } from "../lib/motion";
import { screenHelperLine, unknownCreatorLabel } from "../lib/uiLanguage";
import type {
  DuplicatesLayoutPreset,
  DuplicateOverview,
  DuplicatePair,
  Screen,
  UserView,
} from "../lib/types";

interface DuplicatesScreenProps {
  refreshVersion: number;
  onNavigate: (screen: Screen) => void;
  userView: UserView;
}

const DUPLICATES_LAYOUT_PRESETS: Array<{
  id: DuplicatesLayoutPreset;
  label: string;
  hint: string;
}> = [
  {
    id: "sweep",
    label: "Sweep",
    hint: "Leaves more width for the duplicate pair list and keeps filters open.",
  },
  {
    id: "balanced",
    label: "Balanced",
    hint: "Balanced comparison layout with filters visible.",
  },
  {
    id: "compare",
    label: "Compare",
    hint: "Widens the right panel and hides filters for side-by-side path checks.",
  },
];

export function DuplicatesScreen({
  refreshVersion,
  onNavigate,
  userView,
}: DuplicatesScreenProps) {
  const {
    duplicatesDetailWidth,
    duplicatesQueueHeight,
    setDuplicatesDetailWidth,
    setDuplicatesQueueHeight,
    duplicatesFiltersCollapsed,
    setDuplicatesFiltersCollapsed,
    duplicatesLayoutPreset,
    applyDuplicatesLayoutPreset,
  } = useUiPreferences();
  const [overview, setOverview] = useState<DuplicateOverview | null>(null);
  const [pairs, setPairs] = useState<DuplicatePair[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [duplicateType, setDuplicateType] = useState("");
  const [search, setSearch] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    void loadDuplicates();
  }, [refreshVersion, duplicateType]);

  useEffect(() => {
    if (!pairs.length) {
      setSelectedId(null);
      return;
    }

    if (!pairs.some((item) => item.id === selectedId)) {
      setSelectedId(pairs[0].id);
    }
  }, [pairs, selectedId]);

  async function loadDuplicates() {
    setIsLoading(true);
    try {
      const [nextOverview, nextPairs] = await Promise.all([
        api.getDuplicateOverview(),
        api.listDuplicatePairs(duplicateType || undefined, 160),
      ]);
      setOverview(nextOverview);
      setPairs(nextPairs);
    } finally {
      setIsLoading(false);
    }
  }

  const filteredPairs = useMemo(() => {
    const term = search.trim().toLowerCase();
    if (!term) {
      return pairs;
    }

    return pairs.filter((item) =>
      [
        item.primaryFilename,
        item.secondaryFilename,
        item.primaryCreator,
        item.secondaryCreator,
        item.primaryPath,
        item.secondaryPath,
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(term)),
    );
  }, [pairs, search]);

  const selected = filteredPairs.find((item) => item.id === selectedId) ?? null;
  const duplicateInspectorSections = selected
    ? [
        {
          id: "summary",
          label:
            userView === "beginner" ? "Why these look alike" : "Pair summary",
          hint:
            userView === "beginner"
              ? "Shows how SimSuite matched these two files."
              : "Detection method, creators, and size comparison.",
          children: (
            <div className="detail-list">
              <DetailRow
                label={userView === "beginner" ? "How it matched" : "Method"}
                value={selected.detectionMethod}
              />
              <DetailRow
                label="Left creator"
                value={selected.primaryCreator ?? unknownCreatorLabel(userView)}
              />
              <DetailRow
                label="Right creator"
                value={selected.secondaryCreator ?? unknownCreatorLabel(userView)}
              />
              {userView !== "beginner" ? (
                <DetailRow
                  label="Size delta"
                  value={`${Math.abs(
                    selected.primarySize - selected.secondarySize,
                  ).toLocaleString()} bytes`}
                />
              ) : null}
            </div>
          ),
        },
        {
          id: "paths",
          label: userView === "beginner" ? "Both file locations" : "Path comparison",
          hint:
            userView === "beginner"
              ? "Check where each copy lives before doing cleanup later."
              : "Side-by-side path comparison for duplicate review.",
          children: (
            <>
              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Left file" : "Primary file"}
                </div>
                <div className="path-card">{selected.primaryPath}</div>
              </div>

              <div className="detail-block">
                <div className="section-label">
                  {userView === "beginner" ? "Right file" : "Secondary file"}
                </div>
                <div className="path-card">{selected.secondaryPath}</div>
              </div>
            </>
          ),
        },
        ...(userView === "power"
          ? [
              {
                id: "hashes",
                label: "Hashes",
                hint: "Exact identity values for deeper comparison.",
                defaultCollapsed: true,
                children: (
                  <div className="path-card">
                    {selected.primaryHash ?? "No hash"} /{" "}
                    {selected.secondaryHash ?? "No hash"}
                  </div>
                ),
              },
            ]
          : []),
      ]
    : [];

  return (
    <section className="screen-shell workbench">
      <div className="screen-header-row">
        <div className="screen-heading">
          <p className="eyebrow">{userView === "beginner" ? "Lookalikes" : "Analysis"}</p>
          <div className="screen-title-row">
            <Copy size={18} strokeWidth={2} />
            <h1>{userView === "beginner" ? "Same Mod Twice?" : "Duplicates"}</h1>
          </div>
          <p className="workspace-toolbar-copy">{screenHelperLine("duplicates", userView)}</p>
        </div>
        <div className="header-actions">
          <button
            type="button"
            className="secondary-action"
            onClick={() => void loadDuplicates()}
            disabled={isLoading}
          >
            <RefreshCw size={14} strokeWidth={2} />
            {isLoading ? "Refreshing..." : "Refresh"}
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => onNavigate("library")}
          >
            <Search size={14} strokeWidth={2} />
            Library
          </button>
        </div>
      </div>

      <div className="summary-matrix">
        <SummaryStat
          label={userView === "beginner" ? "Matches" : "Pairs"}
          value={overview?.totalPairs ?? 0}
          tone="neutral"
        />
        <SummaryStat label="Exact" value={overview?.exactPairs ?? 0} tone="good" />
        <SummaryStat label="Filename" value={overview?.filenamePairs ?? 0} tone="neutral" />
        <SummaryStat label="Version" value={overview?.versionPairs ?? 0} tone="low" />
      </div>

      <LayoutPresetBar
        title={userView === "beginner" ? "Quick view" : "Duplicates layout"}
        summary={
          userView === "beginner"
            ? "Keep the match list and the compare panel easy to read while you look through repeats."
            : userView === "power"
              ? "Saved layouts for broad sweeps or deeper path comparison."
              : "Saved layouts for broad sweeps or deeper path comparison."
        }
        presets={userView === "beginner" ? [] : DUPLICATES_LAYOUT_PRESETS}
        activePreset={duplicatesLayoutPreset}
        onApplyPreset={(preset) =>
          applyDuplicatesLayoutPreset(preset as DuplicatesLayoutPreset)
        }
        filterToggle={{
          collapsed: duplicatesFiltersCollapsed,
          onToggle: () =>
            setDuplicatesFiltersCollapsed(!duplicatesFiltersCollapsed),
          hiddenLabel: "Show filters",
          shownLabel: "Hide filters",
        }}
      />

      {!duplicatesFiltersCollapsed ? (
        <div className="panel-card filter-panel">
          <div className="filter-grid">
            <label className="field">
              <span>Search</span>
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Filename or creator"
              />
            </label>

            <label className="field">
              <span>{userView === "beginner" ? "Match type" : "Type"}</span>
              <select
                value={duplicateType}
                onChange={(event) => setDuplicateType(event.target.value)}
              >
                <option value="">All</option>
                <option value="exact">Exact</option>
                <option value="filename">Filename</option>
                <option value="version">Version</option>
              </select>
            </label>
          </div>
        </div>
      ) : null}

      {filteredPairs.length ? (
        <div className="review-layout duplicates-layout">
          <div className="panel-card queue-panel duplicates-queue-panel">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Pairs</p>
                <h2>{userView === "beginner" ? "Possible repeats" : "Detected duplicates"}</h2>
              </div>
              <span className="ghost-chip">{filteredPairs.length} shown</span>
            </div>

            <div className="vertical-dock queue-dock">
              <div className="queue-list duplicates-queue-list">
                {filteredPairs.map((pair, index) => (
                  <m.button
                    key={pair.id}
                    type="button"
                    className={`queue-row ${selectedId === pair.id ? "is-selected" : ""}`}
                    onClick={() => setSelectedId(pair.id)}
                    title={`${pair.primaryFilename} and ${pair.secondaryFilename}`}
                    whileHover={rowHover}
                    whileTap={rowPress}
                    {...stagedListItem(index)}
                  >
                    <div className="queue-main">
                      <strong>{pair.primaryFilename}</strong>
                      <span>
                        {pair.secondaryFilename}
                        {userView === "power"
                          ? ` · ${pair.primarySize.toLocaleString()} / ${pair.secondarySize.toLocaleString()} bytes`
                          : ""}
                      </span>
                    </div>
                    <div className="queue-meta">
                      <span className="ghost-chip">{pair.duplicateType}</span>
                      <span className="ghost-chip">{pair.detectionMethod}</span>
                    </div>
                  </m.button>
                ))}
              </div>
              <ResizableEdgeHandle
                label="Resize duplicates list height"
                value={duplicatesQueueHeight}
                min={260}
                max={860}
                onChange={setDuplicatesQueueHeight}
                side="bottom"
                className="dock-resize-handle duplicates-queue-height-handle"
              />
            </div>
          </div>

          <ResizableDetailPanel
            ariaLabel="Duplicate details"
            width={duplicatesDetailWidth}
            onWidthChange={setDuplicatesDetailWidth}
            minWidth={320}
            maxWidth={780}
          >
            {selected ? (
              <>
                <div className="detail-header">
                  <div>
                    <p className="eyebrow">
                      {userView === "beginner" ? "Selected match" : "Selected pair"}
                    </p>
                    <h2>{selected.primaryFilename}</h2>
                  </div>
                  <span className="ghost-chip">{selected.duplicateType}</span>
                </div>

                <DockSectionStack
                  layoutId="duplicatesInspector"
                  sections={duplicateInspectorSections}
                  intro={
                    userView === "beginner"
                      ? "Open only the comparison panels you need and move them into the order you like."
                      : "Collapse or reorder duplicate sections to fit broad sweeps or deep comparisons."
                  }
                />
              </>
            ) : (
              <StatePanel
                eyebrow={userView === "beginner" ? "Same Mod Twice?" : "Duplicates"}
                title={userView === "beginner" ? "Select a match" : "Select a pair"}
                body={
                  userView === "beginner"
                    ? "Choose one possible repeat from the left to compare both file paths and see how SimSuite matched them."
                    : "Select a duplicate pair to inspect the path comparison, detection method, and exact hash details when available."
                }
                icon={SearchX}
                meta={["Read-only for now", "Cleanup actions come later"]}
              />
            )}
          </ResizableDetailPanel>
        </div>
      ) : (
        <StatePanel
          eyebrow={userView === "beginner" ? "Same Mod Twice?" : "Duplicates"}
          title={
            userView === "beginner"
              ? "No repeated files match this filter"
              : "No pairs match the current filter"
          }
          body={
            userView === "beginner"
              ? "Try clearing the search or switching the match type if you want to look for broader possible repeats."
              : "Clear the search or broaden the duplicate type filter to see more of the indexed overlap set."
          }
          icon={Copy}
          tone="info"
          meta={["Exact, filename, and version views available"]}
        />
      )}
    </section>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SummaryStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: number;
  tone: "good" | "neutral" | "low";
}) {
  return (
    <div className={`summary-stat summary-stat-${tone}`}>
      <span>{label}</span>
      <strong>{value.toLocaleString()}</strong>
    </div>
  );
}
