import { m } from "motion/react";
import { cardHoverShadow, cardPress, stagedListItem } from "../../lib/motion";
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
                  whileHover={cardHoverShadow}
                  whileTap={cardPress}
                  {...stagedListItem(index)}
                >
                  {/* ── Resting card: thumbnail dominant, minimal chrome ─────────────────────── */}
                  {/* Type-color accent bar at top of hero — the only always-visible type signal */}
                  <div className={`library-card-hero library-card-hero--${model.typeColor}`}>
                    {/* Full-bleed thumbnail or fallback */}
                    {model.thumbnailPreview ? (
                      <div className="library-card-thumb-wrap">
                        <img
                          src={`data:image/png;base64,${model.thumbnailPreview}`}
                          alt={`Preview for ${model.displayTitle}`}
                          className="library-card-thumbnail-img"
                        />
                        {/* Source badge: top-left corner of thumbnail — visible at rest */}
                        {model.previewSource && model.previewSource !== 'fallback' ? (
                          <span
                            className={`library-thumb-source-badge library-thumb-source-badge--${model.previewSource}`}
                            title={`Preview: ${model.previewSource}`}
                          >
                            {model.previewSource === 'embedded' ? 'M' : model.previewSource === 'cache' ? 'C' : 'E'}
                          </span>
                        ) : null}
                        {/* Source dot: bottom-left (quiet indicator) */}
                        {model.previewSource && model.previewSource !== 'fallback' ? (
                          <div className="library-thumb-source-dot" title={`Preview: ${model.previewSource}`}>
                            <span className={`source-dot source-dot--${model.previewSource} is-active`} />
                          </div>
                        ) : null}
                      </div>
                    ) : (
                      <div className={`library-card-thumbnail-zone--fallback library-card-hero--${model.typeColor}`}>
                        <FallbackCategoryIcon kind={model.kind} />
                        <div className="library-thumb-source-dot" title="No preview available">
                          <span className="source-dot" />
                        </div>
                      </div>
                    )}

                    {/* Color swatch strip — shown only on fallback cards */}
                    {model.colorSwatches.length > 0 && !model.thumbnailPreview && (
                      <div className="library-card-swatch-strip" aria-label="Color hints from file metadata">
                        {model.colorSwatches.slice(0, 6).map((hex, i) => (
                          <div key={i} className="library-card-swatch" style={{ background: hex }} title={hex} />
                        ))}
                      </div>
                    )}

                    {/* Resting state: tiny type pill at hero bottom — thumbnail leads */}
                    <div className="library-card-title-block">
                      <span className={`library-type-pill type-pill--${model.typeColor}`}>
                        {model.typeLabel}
                      </span>
                    </div>

                    {/* ── Reveal panel: all useful info on hover/select/focus ─────────────── */}
                    <div className="library-card-info-reveal">
                      <div className="library-card-reveal-title" title={model.title}>
                        {model.displayTitle}
                      </div>
                      {model.identityLabel && (
                        <div className="library-card-reveal-identity">{model.identityLabel}</div>
                      )}
                      <div className="library-card-reveal-meta">
                        <span className={`library-type-pill type-pill--${model.typeColor}`}>
                          {model.typeLabel}
                        </span>
                        {model.previewSource && model.previewSource !== 'fallback' && (
                          <span className={`library-reveal-source library-reveal-source--${model.previewSource}`}>
                            {model.previewSource === 'embedded' ? 'EM' : model.previewSource === 'cache' ? 'CH' : 'EX'}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Status badges — only shown when selected (not cluttering resting state) */}
                  {isSelected && (model.hasIssues || model.hasDuplicate || model.isMisplaced || model.watchStatusLabel !== 'Not tracked') && (
                    <div className="library-card-status-badges">
                      {model.hasIssues && <span className="library-issues-badge" title="Has safety notes">⚑</span>}
                      {model.hasDuplicate && <span className="library-duplicate-badge">Duplicate</span>}
                      {model.isMisplaced && <span className="library-card-misplaced-badge">⚠ misplaced</span>}
                      {model.watchStatusLabel !== 'Not tracked' && (
                        <span className={`library-health-pill is-${model.watchStatusTone}`}>{model.watchStatusLabel}</span>
                      )}
                    </div>
                  )}

                  {/* Confidence bar — quiet, bottom edge */}
                  <div className={`library-card-confidence-bar confidence-bar--${model.confidenceLevel}`} aria-label={`${model.confidenceLevel} confidence`} />
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
