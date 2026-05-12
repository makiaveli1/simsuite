import { useEffect, useMemo, useState } from "react";
import { m } from "motion/react";
import { Copy, RefreshCw, Search, SearchX } from "lucide-react";
import { DockSectionStack } from "../components/DockSectionStack";
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
  initialFileIds?: number[];
}

const DUPLICATES_LAYOUT_PRESETS: Array<{
  id: DuplicatesLayoutPreset;
  label: string;
  hint: string;
}> = [
  {
    id: "sweep",
    label: "Sweep",
    hint: "Leaves more width for the comparison pair list and keeps filters open.",
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
  initialFileIds,
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
  const focusedFileKey = (initialFileIds ?? [])
    .filter((id) => Number.isFinite(id))
    .join("|");
  const focusedFileIds = useMemo(
    () =>
      focusedFileKey
        ? focusedFileKey
            .split("|")
            .map((id) => Number(id))
            .filter((id) => Number.isFinite(id))
        : [],
    [focusedFileKey],
  );
  const focusedFileIdSet = useMemo(() => new Set(focusedFileIds), [focusedFileIds]);

  useEffect(() => {
    void loadDuplicates();
  }, [refreshVersion, duplicateType]);

  const focusedPair = useMemo(
    () =>
      pairs.find(
        (item) =>
          focusedFileIdSet.has(item.primaryFileId) ||
          focusedFileIdSet.has(item.secondaryFileId),
      ) ?? null,
    [focusedFileIdSet, pairs],
  );

  useEffect(() => {
    if (!pairs.length) {
      setSelectedId(null);
      return;
    }

    if (focusedPair && selectedId !== focusedPair.id) {
      setSelectedId(focusedPair.id);
      return;
    }

    if (!pairs.some((item) => item.id === selectedId)) {
      setSelectedId(pairs[0].id);
    }
  }, [focusedPair, pairs, selectedId]);

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
        item.classificationLabel,
        item.confidenceLabel,
        item.comparisonKind,
        ...(item.evidence ?? []),
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(term)),
    );
  }, [pairs, search]);

  const selected = filteredPairs.find((item) => item.id === selectedId) ?? null;
  const focusedFilename = focusedPair
    ? describeFocusedDuplicateFilename(focusedPair, focusedFileIdSet)
    : null;
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
                label={userView === "beginner" ? "What it means" : "Classification"}
                value={selected.classificationLabel}
              />
              <DetailRow
                label="Evidence strength"
                value={selected.confidenceLabel}
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
              {selected.evidence.length ? (
                <div className="tag-list">
                  {selected.evidence.map((item) => (
                    <span key={item} className="ghost-chip">
                      {item}
                    </span>
                  ))}
                </div>
              ) : null}
              {selected.cautions.length ? (
                <p className="text-muted">{selected.cautions.join(" · ")}</p>
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
    <section className="screen-shell workbench workbench-screen duplicates-screen">
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

      {focusedFileIds.length ? (
        <div className="audit-rail-note duplicates-focus-caption">
          <strong>Opened from Library.</strong>
          <p>
            {focusedPair && focusedFilename
              ? `Focused a Library comparison for ${focusedFilename}. Compare files before changing anything.`
              : "No matching duplicate comparison was found for the selected Library file. Showing the available comparisons instead."}
          </p>
        </div>
      ) : null}

      {filteredPairs.length ? (
        <div className="duplicates-workbench">
          <div className="panel-card duplicates-rail">
            <div className="panel-heading">
              <div>
                <p className="eyebrow">Pairs</p>
                <h2>{userView === "beginner" ? "File comparisons" : "Duplicate intelligence"}</h2>
              </div>
              <button
                type="button"
                className="workspace-toggle"
                onClick={() =>
                  setDuplicatesFiltersCollapsed(!duplicatesFiltersCollapsed)
                }
              >
                {duplicatesFiltersCollapsed ? "Show filters" : "Hide filters"}
              </button>
            </div>

            <div className="summary-matrix duplicates-summary-strip">
              <SummaryStat
                label={userView === "beginner" ? "Checks" : "Comparisons"}
                value={overview?.totalPairs ?? 0}
                tone="neutral"
              />
              <SummaryStat
                label="Duplicates"
                value={overview?.exactPairs ?? 0}
                tone="good"
              />
              <SummaryStat
                label="Name matches"
                value={overview?.filenamePairs ?? 0}
                tone="neutral"
              />
              <SummaryStat
                label="Version reviews"
                value={overview?.versionPairs ?? 0}
                tone="low"
              />
            </div>

            {!duplicatesFiltersCollapsed ? (
              <div className="duplicates-rail-stack">
                <div className="audit-rail-note duplicates-rail-note">
                  <strong>Compare the context, not just the name.</strong>
                  <p>
                    Start with the focused pair in the middle, check where both copies
                    live, then use the right panel only when you need deeper proof.
                  </p>
                </div>

                <div className="duplicates-filter-grid">
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
                      <option value="">All comparisons</option>
                      <option value="exact">Duplicates</option>
                      <option value="filename">Name matches</option>
                      <option value="version">Version reviews</option>
                    </select>
                  </label>
                </div>

                {userView !== "beginner" ? (
                  <div className="duplicates-layout-presets">
                    {DUPLICATES_LAYOUT_PRESETS.map((preset) => (
                      <button
                        key={preset.id}
                        type="button"
                        className={`workspace-toggle ${
                          duplicatesLayoutPreset === preset.id ? "is-active" : ""
                        }`}
                        onClick={() => applyDuplicatesLayoutPreset(preset.id)}
                        title={preset.hint}
                      >
                        {preset.label}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : (
              <p className="duplicates-rail-collapsed-copy">
                Open the filters when you want to narrow the list or switch the balance
                between the middle stage and the inspector.
              </p>
            )}
          </div>

          <div className="duplicates-stage">
            <ResizableEdgeHandle
              label="Resize duplicates list height"
              value={duplicatesQueueHeight}
              min={260}
              max={860}
              onChange={setDuplicatesQueueHeight}
              side="bottom"
              className="layout-resize-handle duplicates-queue-height-handle duplicates-stage-handle"
            />
            <div className="panel-card duplicates-focus-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">
                    {userView === "beginner" ? "Selected match" : "Focus pair"}
                  </p>
                  <h2>
                    {selected
                      ? userView === "beginner"
                        ? selected.isDuplicate
                          ? "Compare both copies"
                          : "Review this comparison"
                        : "Compare this pair"
                      : userView === "beginner"
                        ? "Pick a match to compare"
                        : "Pick a pair to compare"}
                  </h2>
                </div>
                {selected ? (
                  <div className="header-actions">
                    <span
                      className={`confidence-badge ${duplicateToneClass(
                        selected,
                      )}`}
                    >
                      {selected.classificationLabel}
                    </span>
                    <span className="ghost-chip">{selected.confidenceLabel}</span>
                  </div>
                ) : null}
              </div>

              {selected ? (
                <div className="duplicates-focus-content">
                  <p className="duplicates-focus-caption">
                    {duplicateStageHeadline(selected, userView)}
                  </p>

                  <div className="duplicates-compare-grid">
                    <DuplicateFileCard
                      label={
                        selected.isDuplicate && userView === "beginner"
                          ? "First copy"
                          : "Primary file"
                      }
                      filename={selected.primaryFilename}
                      creator={selected.primaryCreator}
                      path={selected.primaryPath}
                      size={selected.primarySize}
                      modifiedAt={selected.primaryModifiedAt}
                      userView={userView}
                    />
                    <DuplicateFileCard
                      label={
                        selected.isDuplicate && userView === "beginner"
                          ? "Second copy"
                          : "Secondary file"
                      }
                      filename={selected.secondaryFilename}
                      creator={selected.secondaryCreator}
                      path={selected.secondaryPath}
                      size={selected.secondarySize}
                      modifiedAt={selected.secondaryModifiedAt}
                      userView={userView}
                    />
                  </div>

                  <div className="duplicates-stage-note">
                    <strong>{duplicateStageSupportHeading(selected)}</strong>
                    <span>{duplicateStageSupportBody(selected, userView)}</span>
                  </div>
                </div>
              ) : (
                <StatePanel
                  eyebrow={userView === "beginner" ? "Same Mod Twice?" : "Duplicates"}
                  title={userView === "beginner" ? "Select a match" : "Select a pair"}
                  body={
                    userView === "beginner"
                      ? "Choose one comparison below to review both files side by side."
                      : "Pick a comparison pair from the queue below to bring it into the center stage."
                  }
                  icon={SearchX}
                  compact
                />
              )}
            </div>

            <div className="panel-card queue-panel duplicates-queue-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">
                    {userView === "beginner" ? "Browse matches" : "Pair queue"}
                  </p>
                  <h2>
                    {userView === "beginner"
                      ? "Work through the overlap list"
                      : "Pairs waiting in this view"}
                  </h2>
                </div>
                <span className="ghost-chip">{filteredPairs.length} shown</span>
              </div>

              <p className="duplicates-queue-caption">
                {userView === "beginner"
                  ? "Use the focused pair above, then move down this list when you are ready for the next check."
                  : "Keep the current pair in view above while you move through the rest of the overlap queue."}
              </p>

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
                      <span className="ghost-chip">{pair.classificationLabel}</span>
                      <span className="ghost-chip">{pair.confidenceLabel}</span>
                    </div>
                  </m.button>
                ))}
              </div>
            </div>
          </div>

          <ResizableDetailPanel
            ariaLabel="Duplicate details"
            className="duplicates-inspector-shell"
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
                  <span className="ghost-chip">{selected.classificationLabel}</span>
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
                    ? "Choose one comparison from the queue to compare both file paths and see how SimSuite matched them."
                    : "Select a comparison pair to inspect the path comparison, detection method, and exact hash details when available."
                }
                icon={SearchX}
                meta={["Compare-only for now", "No cleanup action here"]}
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
              ? "Try clearing the search or switching the match type if you want to look for broader comparisons."
              : "Clear the search or broaden the comparison type filter to see more of the indexed overlap set."
          }
          icon={Copy}
          tone="info"
          meta={["Duplicates, name matches, and version reviews available"]}
        />
      )}
    </section>
  );
}

function duplicateToneClass(pair: DuplicatePair) {
  if (pair.isDuplicate) {
    return "good";
  }

  switch (pair.comparisonKind.toLowerCase()) {
    case "exact_file":
    case "exact_package":
    case "exact_script":
      return "good";
    case "name_match_review":
      return "medium";
    default:
      return "low";
  }
}

function duplicateStageHeadline(pair: DuplicatePair, userView: UserView) {
  switch (pair.comparisonKind.toLowerCase()) {
    case "exact_file":
    case "exact_package":
    case "exact_script":
      return userView === "beginner"
        ? "SimSuite matched the contents. Compare folders and backups before changing anything."
        : "This pair has same-content evidence, so path and folder context should carry most of the review.";
    case "name_match_review":
      return userView === "beginner"
        ? "These names match, but SimSuite did not find same-content duplicate proof."
        : "This name match is for review only because the underlying file contents differ or are not proven identical.";
    case "version_review":
      return userView === "beginner"
        ? "Version clues suggest these may be different releases of the same mod."
        : "This looks like a version variant, so size, folder context, and naming clues matter most.";
    default:
      return userView === "beginner"
        ? "SimSuite has limited information here, so compare the files manually before changing anything."
        : "This pair needs manual comparison because the duplicate evidence is limited.";
  }
}

function duplicateStageSupportHeading(pair: DuplicatePair) {
  if (pair.evidence.length) {
    return pair.evidence.slice(0, 2).join(" · ");
  }

  if (pair.primaryCreator && pair.primaryCreator === pair.secondaryCreator) {
    return `Both copies currently point to ${pair.primaryCreator}.`;
  }

  return Math.abs(pair.primarySize - pair.secondarySize) === 0
    ? "Both files are the same size."
    : "The file sizes do not match exactly.";
}

function duplicateStageSupportBody(pair: DuplicatePair, userView: UserView) {
  const sizeDelta = Math.abs(pair.primarySize - pair.secondarySize);
  const sizeLine =
    sizeDelta === 0
      ? pair.isDuplicate
        ? "The file sizes match, so folder path and backups are the next context to check."
        : "Matching size alone is not duplicate proof."
      : `There is a ${sizeDelta.toLocaleString()} byte gap between them, which can help separate the newer or edited copy.`;

  return userView === "beginner"
    ? `${sizeLine} The deeper evidence stays on the right if you want the full comparison. ${pair.cautions[0] ?? ""}`.trim()
    : `${sizeLine} Keep the middle stage for quick comparison and the right inspector for the evidence trail. ${pair.cautions[0] ?? ""}`.trim();
}

function formatDuplicateDate(value: string | null) {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return parsed.toLocaleDateString("en-IE", {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

function describeFocusedDuplicateFilename(
  pair: DuplicatePair,
  focusedFileIds: Set<number>,
) {
  if (focusedFileIds.has(pair.primaryFileId)) {
    return pair.primaryFilename;
  }
  if (focusedFileIds.has(pair.secondaryFileId)) {
    return pair.secondaryFilename;
  }
  return pair.primaryFilename;
}

function DuplicateFileCard({
  label,
  filename,
  creator,
  path,
  size,
  modifiedAt,
  userView,
}: {
  label: string;
  filename: string;
  creator: string | null;
  path: string;
  size: number;
  modifiedAt: string | null;
  userView: UserView;
}) {
  const modifiedLabel = formatDuplicateDate(modifiedAt);

  return (
    <div className="duplicates-file-card">
      <div className="duplicates-file-topline">
        <span className="section-label">{label}</span>
        <span className="ghost-chip">{size.toLocaleString()} bytes</span>
      </div>
      <strong>{filename}</strong>
      <p className="duplicates-file-caption">
        {creator ?? unknownCreatorLabel(userView)}
      </p>
      <div className="path-card">{path}</div>
      {modifiedLabel ? (
        <div className="duplicates-file-meta">
          <span>Seen on {modifiedLabel}</span>
        </div>
      ) : null}
    </div>
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
