import { m } from "motion/react";
import { rowHover, rowPress, stagedListItem } from "../../lib/motion";
import type { LibraryFileRow, UserView } from "../../lib/types";
import {
  buildLibraryCardModel,
  type LibraryCardModel,
  usefulTrayGroupingValue,
} from "./libraryDisplay";

interface LibraryThumbnailGridProps {
  userView: UserView;
  rows: LibraryFileRow[];
  selectedId: number | null;
  onSelect: (row: LibraryFileRow) => void;
  onPrevPage: () => void;
  onNextPage: () => void;
  page: number;
  totalPages: number;
}

// ─── Fallback Category Icon ─────────────────────────────────────────────────
// Shown when no THUM preview is available. Clearly NOT a real extracted
// thumbnail — uses category color + simple symbol to indicate "no preview".
function FallbackCategoryIcon({ kind }: { kind: string }) {
  const configs: Record<string, { bg: string; symbol: React.ReactNode }> = {
    CAS: {
      bg: "rgba(236, 72, 153, 0.2)",
      symbol: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#ec4899" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z" />
          <line x1="3" y1="6" x2="21" y2="6" />
          <path d="M16 10a4 4 0 0 1-8 0" />
        </svg>
      ),
    },
    BuildBuy: {
      bg: "rgba(168, 85, 247, 0.2)",
      symbol: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#a855f7" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
      ),
    },
    ScriptMods: {
      bg: "rgba(59, 130, 246, 0.2)",
      symbol: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="16 18 22 12 16 6" />
          <polyline points="8 6 2 12 8 18" />
        </svg>
      ),
    },
    // Covers Tray, Household, Lot, Room and any tray variant
    defaultKind: {
      bg: "rgba(251, 191, 36, 0.2)",
      symbol: (
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
        </svg>
      ),
    },
  };
  const config =
    kind in configs ? configs[kind] : configs["defaultKind"];
  return (
    <div
      className="library-card-fallback-icon"
      style={{ background: config.bg }}
      title={`${kind} — no THUM preview available`}
    >
      {config.symbol}
    </div>
  );
}

/**
 * Renders the type-specific content block for a grid card.
 * One of these is always shown — no generic "no preview available" for mod types.
 * userView tunes the level of technical detail shown (e.g. hiding "No namespace detected").
 */
function renderCardContent(model: LibraryCardModel, userView: UserView): React.ReactNode {
  const kind = model.kind;

  // ── CAS: show embedded item name chips + subtype ──────────────────────
  if (kind === "CAS") {
    const hasContent = model.casNames.length > 0;
    return (
      <div className="library-card-content-inner">
        {hasContent ? (
          <>
            <div className="library-card-names">
              {model.casNames.map((name) => (
                <span key={name} className="library-card-name-chip">
                  {name}
                </span>
              ))}
              {model.casNamesOverflow > 0 && (
                <span className="library-card-name-overflow">
                  +{model.casNamesOverflow}
                </span>
              )}
            </div>
            {model.subtype && (
              <div className="library-card-subtype-label">{model.subtype}</div>
            )}
          </>
        ) : model.contentSummary ? (
          <>
            <div className="library-card-resource-summary">{model.contentSummary}</div>
            {model.subtype && (
              <div className="library-card-subtype-label">{model.subtype}</div>
            )}
          </>
        ) : (
          <div className="library-card-subtype-label">
            {model.subtype ?? "CAS content"}
          </div>
        )}
      </div>
    );
  }

  // ── ScriptMods: show namespace chips + version ─────────────────────────
  if (kind === "ScriptMods") {
    const hasNamespaces = model.cardScriptNamespaces.length > 0;
    return (
      <div className="library-card-content-inner">
        {hasNamespaces ? (
          <>
            <div className="library-card-names">
              {model.cardScriptNamespaces.map((ns) => (
                <span key={ns} className="library-card-name-chip library-card-namespace-chip">
                  {ns}
                </span>
              ))}
              {model.cardScriptNamespaceOverflow > 0 && (
                <span className="library-card-name-overflow">
                  +{model.cardScriptNamespaceOverflow}
                </span>
              )}
            </div>
            {model.scriptVersionLabel && (
              <div className="library-card-version-label">{model.scriptVersionLabel}</div>
            )}
          </>
        ) : model.scriptVersionLabel ? (
          <div className="library-card-version-only">
            <span className="library-card-version-badge">{model.scriptVersionLabel}</span>
            <div className="library-card-subtype-label">Script mod</div>
          </div>
        ) : (
          <>
            <div className="library-card-subtype-label">Script mod</div>
            {userView !== "beginner" && (
              <div
                className="library-card-subtype-label"
                style={{ opacity: 0.6 }}
              >
                No namespace detected
              </div>
            )}
          </>
        )}
      </div>
    );
  }

  // ── BuildBuy: show resource summary + subtype ───────────────────────────
  if (kind === "BuildBuy") {
    return (
      <div className="library-card-content-inner">
        <div className="library-card-resource-summary">
          {model.contentSummary ?? "Build/Buy content"}
        </div>
        {model.subtype && (
          <div className="library-card-subtype-label">{model.subtype}</div>
        )}
      </div>
    );
  }

  // ── OverridesAndDefaults: show resource summary ────────────────────────
  if (kind === "OverridesAndDefaults") {
    return (
      <div className="library-card-content-inner">
        <div className="library-card-resource-summary">
          {model.contentSummary ?? "Override package"}
        </div>
      </div>
    );
  }

  // ── PosesAndAnimation: show subtype ────────────────────────────────────
  if (kind === "PosesAndAnimation") {
    return (
      <div className="library-card-content-inner">
        <div className="library-card-subtype-label">
          {model.subtype ?? "Pose / Animation"}
        </div>
      </div>
    );
  }

  // ── PresetsAndSliders: show subtype ───────────────────────────────────
  if (kind === "PresetsAndSliders") {
    return (
      <div className="library-card-content-inner">
        <div className="library-card-subtype-label">
          {model.subtype ?? "Preset / Slider"}
        </div>
      </div>
    );
  }

  // ── Tray items: show household/lot/room identity ──────────────────────
  if (
    kind === "TrayHousehold" ||
    kind === "TrayLot" ||
    kind === "TrayRoom" ||
    kind === "TrayItem" ||
    kind === "Household" ||
    kind === "Lot" ||
    kind === "Room"
  ) {
    return (
      <div className="library-card-content-inner">
        {model.trayIdentityLabel ? (
          <div className="library-card-tray-identity">{model.trayIdentityLabel}</div>
        ) : (
          <div className="library-card-subtype-label">{model.subtype ?? "Tray item"}</div>
        )}
        {model.isGrouped && model.groupedCount > 1 && (
          <div className="library-card-subtype-label">
            {model.groupedCount} files in pack
          </div>
        )}
      </div>
    );
  }

  // ── Gameplay: show content profile as primary signal ───────────────────
  if (kind === "Gameplay") {
    return (
      <div className="library-card-content-inner">
        {model.contentSummary ? (
          <div className="library-card-resource-summary">{model.contentSummary}</div>
        ) : (
          <div className="library-card-subtype-label">
            {model.subtype ?? "Gameplay mod"}
          </div>
        )}
      </div>
    );
  }

  // ── Unknown: show resource summary or fallback ─────────────────────────
  return (
    <div className="library-card-content-inner">
      {model.contentSummary ? (
        <div className="library-card-resource-summary">{model.contentSummary}</div>
      ) : model.subtype ? (
        <div className="library-card-subtype-label">{model.subtype}</div>
      ) : (
        <div className="library-card-empty-preview">
          {userView === "beginner" ? "Not recognized" : "No content detected"}
        </div>
      )}
      {userView !== "beginner" && model.contentSummary && (
        <div className="library-card-subtype-label" style={{ opacity: 0.55 }}>
          Unknown type
        </div>
      )}
    </div>
  );
}

export function LibraryThumbnailGrid({
  userView,
  rows,
  selectedId,
  onSelect,
  onPrevPage,
  onNextPage,
  page,
  totalPages,
}: LibraryThumbnailGridProps) {
  return (
    <>
      <div className="library-grid-scroll library-list-shell">
        <div className="library-grid">
          {rows.length ? (
            (() => {
              // Phase 5: deduplicate tray packs — collapse all files sharing a bundleName
              // to a single pack-head card. Also drop ghost cards (Unknown kind + empty title)
              // that the backend returns as variant/metadata entries with no useful content.
              const seenBundleNames = new Set<string>();
              const filteredRows = rows.filter((row) => {
                const model = buildLibraryCardModel(row, userView);
                // Drop ghost cards: Unknown kind with no displayable content
                if (model.kind === "Unknown" && !model.displayTitle) return false;
                // Collapse tray duplicates: only keep the first file in each bundle
                if (model.isGrouped && model.bundleName) {
                  if (seenBundleNames.has(model.bundleName)) return false;
                  seenBundleNames.add(model.bundleName);
                }
                return true;
              });

              return filteredRows.map((row, index) => {
                const model = buildLibraryCardModel(row, userView);
                const isSelected = selectedId === row.id;

              return (
                <m.div
                  key={row.id}
                  data-kind={model.kind}
                  data-bundle-name={model.bundleName ?? undefined}
                  className={[
                    "library-card",
                    `library-card--${model.typeColor}`,
                    isSelected ? "is-selected" : "",
                    model.isTray ? "is-tray" : "",
                    model.hasIssues ? "has-issues" : "",
                    model.isMisplaced ? "is-misplaced" : "",
                    ((model.row.groupedFileCount ?? 0) > 1) ? "is-grouped" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => onSelect(row)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      onSelect(row);
                    }
                  }}
                  whileHover={rowHover}
                  whileTap={rowPress}
                  {...stagedListItem(index)}
                >
                  {/* Card header: tiny type dot + status badges */}
                  <div className="library-card-header">
                    <span className={`library-type-dot library-type-dot--${model.typeColor}`} aria-hidden="true" />
                    <div className="library-card-header-right">
                      {model.hasIssues && (
                        <span
                          className="library-issues-badge"
                          title="Has safety notes or parser warnings"
                        >
                          ⚑
                        </span>
                      )}
                      {model.hasDuplicate && (
                        <span className="library-duplicate-badge">Duplicate</span>
                      )}
                      {model.isGrouped && model.groupedCount > 1 && (
                        <span
                          className="library-card-pack-badge"
                          title={`Part of a ${model.groupedCount}-file pack`}
                        >
                          📦 {model.groupedCount} files
                        </span>
                      )}
                      {model.isMisplaced && (
                        <span
                          className="library-card-misplaced-badge"
                          title="This tray item is in the Mods folder — it needs review"
                        >
                          ⚠ misplaced
                        </span>
                      )}
                      <span className={`library-health-pill is-${model.watchStatusTone}`}>
                        {model.watchStatusLabel}
                      </span>
                    </div>
                  </div>

                  <div className="library-card-title-block">
                    <div className="library-card-title" title={model.title}>
                      {model.displayTitle}
                    </div>
                    <div className="library-card-title-meta">
                      <span className={`library-type-pill type-pill--${model.typeColor}`}>
                        {model.typeLabel}
                      </span>
                    </div>
                    {model.identityLabel ? (
                      <div className="library-card-identity" title={model.identityLabel}>
                        {model.identityLabel}
                      </div>
                    ) : null}
                  </div>

                  {/* Optional thumbnail preview (THUM 0x3C1AF1F2) */}
                  {model.thumbnailPreview ? (
                    <div className="library-card-thumbnail-zone">
                      <img
                        src={`data:image/png;base64,${model.thumbnailPreview}`}
                        alt={`Preview for ${model.displayTitle}`}
                        className="library-card-thumbnail-img"
                      />
                    </div>
                  ) : (
                    <div className="library-card-thumbnail-zone library-card-thumbnail-zone--fallback">
                      <FallbackCategoryIcon kind={model.kind} />
                    </div>
                  )}

                  {/* Content preview — type-specific */}
                  <div className="library-card-content-preview">
                    {renderCardContent(model, userView)}
                  </div>

                  {/* Card footer: creator + version */}
                  <div className="library-card-footer">
                    <div className="library-card-meta">
                      <span className="library-card-creator">{model.creatorLabel}</span>
                      {model.versionLabel && (
                        <span className="library-card-version">{model.versionLabel}</span>
                      )}
                    </div>
                    {model.isGrouped && (() => {
                      const safeBundleName = usefulTrayGroupingValue({
                        bundleName: model.bundleName ?? null,
                        insights: model.row.insights,
                      });

                      if (safeBundleName) {
                        return (
                          <div className="library-card-pack-label">
                            Pack: {safeBundleName}
                          </div>
                        );
                      }

                      if (model.groupedCount > 1) {
                        return (
                          <div className="library-card-pack-label">
                            {model.groupedCount} grouped files
                          </div>
                        );
                      }

                      return null;
                    })()}
                    {!model.isMisplaced && model.isTray && (
                      <div className="library-card-tray-badge">tray · disabled</div>
                    )}
                  </div>

                  {/* Confidence bar at card bottom edge */}
                  <div
                    className={`library-card-confidence-bar confidence-bar--${model.confidenceLevel}`}
                    aria-label={`${model.confidenceLevel} confidence`}
                  />
                </m.div>
              );
              });
            })()
          ) : (
            <div className="library-grid-empty">
              {userView === "beginner"
                ? "Nothing matches these filters right now."
                : "No indexed files match the current filters."}
            </div>
          )}
        </div>
      </div>

      <div className="table-footer">
        <button
          type="button"
          className="secondary-action"
          onClick={onPrevPage}
          disabled={page === 0}
        >
          Previous
        </button>
        <div className="table-page-label">
          Page {Math.min(page + 1, totalPages)} of {totalPages}
        </div>
        <button
          type="button"
          className="secondary-action"
          onClick={onNextPage}
          disabled={page + 1 >= totalPages}
        >
          Next
        </button>
      </div>

      <div className="table-pagination-bar" />
    </>
  );
}
