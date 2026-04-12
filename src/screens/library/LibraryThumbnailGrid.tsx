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

/**
 * Renders the type-specific content block for a grid card.
 * One of these is always shown — no generic "no preview available" for mod types.
 */
function renderCardContent(model: LibraryCardModel): React.ReactNode {
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
    const hasNamespaces = model.scriptNamespaces.length > 0;
    return (
      <div className="library-card-content-inner">
        {hasNamespaces ? (
          <>
            <div className="library-card-names">
              {model.scriptNamespaces.map((ns) => (
                <span key={ns} className="library-card-name-chip library-card-namespace-chip">
                  {ns}
                </span>
              ))}
              {model.scriptNamespaceOverflow > 0 && (
                <span className="library-card-name-overflow">
                  +{model.scriptNamespaceOverflow}
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
            <div
              className="library-card-subtype-label"
              style={{ opacity: 0.6 }}
            >
              No namespace detected
            </div>
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

  // ── Unknown: show resource summary or fallback ─────────────────────────
  return (
    <div className="library-card-content-inner">
      {model.contentSummary ? (
        <>
          <div className="library-card-resource-summary">{model.contentSummary}</div>
          <div className="library-card-subtype-label">Unknown type</div>
        </>
      ) : model.subtype ? (
        <>
          <div className="library-card-subtype-label">{model.subtype}</div>
          <div className="library-card-subtype-label">Unknown type</div>
        </>
      ) : (
        <div className="library-card-empty-preview">
          No content detected
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
            rows.map((row, index) => {
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
                      {model.title}
                    </div>
                    {model.identityLabel ? (
                      <div className="library-card-identity" title={model.identityLabel}>
                        {model.identityLabel}
                      </div>
                    ) : null}
                  </div>

                  {/* Content preview — type-specific */}
                  <div className="library-card-content-preview">
                    {renderCardContent(model)}
                  </div>

                  {/* Card footer: type + creator + version */}
                  <div className="library-card-footer">
                    <div className="library-card-meta">
                      <span className={`library-type-pill type-pill--${model.typeColor}`}>
                        {model.typeLabel}
                      </span>
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
            })
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
