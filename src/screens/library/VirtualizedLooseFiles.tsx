import { useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, ChevronUp } from "lucide-react";
import { m } from "motion/react";
import { rowHover, rowPress } from "../../lib/motion";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";
import { LibraryCollectionTable } from "./LibraryCollectionTable";
import { buildLibraryRowModel, type LibraryRowModel } from "./libraryDisplay";

// Phase 5aa: CSS row height for virtualizer. Must match .library-list-row height in globals.css.
const ROW_HEIGHT = 88;

// Threshold: below this count, render a plain LibraryCollectionTable (no virtualization overhead).
const PLAIN_RENDER_THRESHOLD = 200;

// Threshold: virtualize once expanded list exceeds this count.
// Below this, just render the full non-virtualized table.
const VIRTUALIZE_THRESHOLD = 300;

interface VirtualizedLooseFilesProps {
  userView: UserView;
  allFiles: LibraryFileRow[];
  selectedFile: FileDetail | null;
  onSelectFile: (file: LibraryFileRow) => void;
}

export function VirtualizedLooseFiles({
  userView,
  allFiles,
  selectedFile,
  onSelectFile,
}: VirtualizedLooseFilesProps) {
  const [expanded, setExpanded] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const count = allFiles.length;
  const selectedId = selectedFile?.id ?? null;

  // ── Plain render: small enough list ──────────────────────────────────────
  // Render via LibraryCollectionTable — no virtualization overhead.
  if (count <= PLAIN_RENDER_THRESHOLD) {
    return (
      <LibraryCollectionTable
        userView={userView}
        rows={allFiles}
        selectedId={selectedId}
        selectedIds={new Set()}
        page={0}
        totalPages={1}
        onSelect={onSelectFile}
        onToggleSelect={() => undefined}
        onPrevPage={() => undefined}
        onNextPage={() => undefined}
        enableSelection={false}
        showPagination={false}
      />
    );
  }

  // ── Virtualized render: large list, user expanded it ─────────────────────
  const visibleCount = expanded ? count : VIRTUALIZE_THRESHOLD;

  const virtualizer = useVirtualizer({
    count: visibleCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalSize = virtualizer.getTotalSize();

  // Pre-compute row models in a single pass
  const modelCache = new Map<number, LibraryRowModel>();
  for (const file of allFiles) {
    modelCache.set(file.id, buildLibraryRowModel(file, userView));
  }

  return (
    <div className="virtualized-loose-files">
      {/* Scroll position indicator — sticky pill */}
      <div className="folder-scroll-counter" aria-live="polite" aria-atomic="true">
        <span className="folder-scroll-counter__range">
          {virtualItems.length > 0
            ? `${virtualItems[0].start + 1}–${virtualItems[virtualItems.length - 1].end + 1}`
            : "0"}
        </span>
        <span className="folder-scroll-counter__sep"> of </span>
        <span className="folder-scroll-counter__total">{count.toLocaleString()}</span>
        <span className="folder-scroll-counter__label"> files</span>
      </div>

      {/* Table column header */}
      <div className="library-list-header" role="row">
        <div className="library-list-col library-list-col--type" aria-label="Type" />
        <div className="library-list-col library-list-col--thumb" aria-label="Preview" />
        <div className="library-list-col library-list-col--select" aria-label="Select" />
        <div className="library-list-col library-list-col--name">
          {userView === "beginner" ? "File" : "Mod or file"}
        </div>
        <div className="library-list-col library-list-col--status">Status</div>
        <div className="library-list-col library-list-col--facts">
          {userView === "power" ? "Clues" : "At a glance"}
        </div>
      </div>

      {/* Virtualized scroll container */}
      <div
        ref={scrollRef}
        className="virtualized-loose-files__scroll-container"
        style={{ height: Math.min(visibleCount * ROW_HEIGHT, 600), overflow: "auto" }}
      >
        <div
          className="virtualized-loose-files__inner"
          style={{ height: totalSize, position: "relative" }}
        >
          {virtualItems.map((virtualRow) => {
            const file = allFiles[virtualRow.index];
            const model = modelCache.get(file.id)!;
            return (
              <VirtualRow
                key={file.id}
                file={file}
                model={model}
                isSelected={selectedId === file.id}
                onSelect={onSelectFile}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  height: ROW_HEIGHT,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              />
            );
          })}
        </div>
      </div>

      {/* Expand / collapse */}
      <div className="virtualized-loose-files__controls">
        {expanded ? (
          <button
            type="button"
            className="folder-load-more"
            onClick={() => setExpanded(false)}
          >
            <ChevronUp size={14} strokeWidth={2} />
            Show less
          </button>
        ) : (
          <button
            type="button"
            className="folder-load-more"
            onClick={() => setExpanded(true)}
          >
            <ChevronDown size={14} strokeWidth={2} />
            Show all {count.toLocaleString()} files
          </button>
        )}
      </div>
    </div>
  );
}

// ── VirtualRow ────────────────────────────────────────────────────────────────
// Mirrors LibraryCollectionTable row rendering without the table wrapper overhead.
// Must stay visually identical to a .library-list-row in the main table.
interface VirtualRowProps {
  file: LibraryFileRow;
  model: LibraryRowModel;
  isSelected: boolean;
  onSelect: (file: LibraryFileRow) => void;
  style: React.CSSProperties;
}

function VirtualRow({ file, model, isSelected, onSelect, style }: VirtualRowProps) {
  return (
    <m.div
      className={[
        "library-list-row",
        "virtualized-row",
        isSelected ? "is-selected" : "",
        model.isTray ? "is-tray" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      style={style}
      onClick={() => onSelect(file)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect(file);
        }
      }}
      whileHover={rowHover}
      whileTap={rowPress}
    >
      <div className="library-list-col library-list-col--type library-type-accent-col">
        {model.typeColor ? (
          <div className={`type-accent type-accent--${model.typeColor}`} aria-label={model.typeLabel} />
        ) : null}
      </div>
      <div className="library-list-col library-list-col--thumb">
        {model.thumbnailPreview ? (
          <>
            <img
              src={`data:image/png;base64,${model.thumbnailPreview}`}
              alt=""
              className="library-row-thumb-img"
            />
            {model.previewSource && model.previewSource !== "fallback" ? (
              <span
                className={`library-row-thumb-source library-row-thumb-source--${model.previewSource}`}
                title={`Source: ${model.previewSource}`}
              >
                {model.previewSource === "cache"
                  ? "CH"
                  : model.previewSource === "embedded"
                  ? "EM"
                  : model.previewSource === "external"
                  ? "EX"
                  : "—"}
              </span>
            ) : null}
          </>
        ) : (
          <div
            className={`library-row-thumb-fallback library-row-thumb-fallback--${model.typeColor}`}
            title={model.typeLabel}
            aria-label={model.typeLabel}
          />
        )}
      </div>
      <div className="library-list-col library-list-col--select" />
      <div className="library-list-col library-list-col--name library-name-cell">
        <div className="library-row-title" title={model.title}>
          {model.displayTitle}
        </div>
        <div className="library-row-meta">
          <span className={`library-type-pill type-pill--${model.typeColor}`}>
            {model.typeLabel}
          </span>
          {model.isTray && file.bundleName ? (
            <span className="library-row-bundle">{file.bundleName}</span>
          ) : null}
        </div>
      </div>
      <div className="library-list-col library-list-col--status library-status-cell">
        <div className="library-status-pills">
          <span className={`library-health-pill is-${model.watchStatusTone}`}>
            {model.watchStatusLabel}
          </span>
          {model.healthTone && (
            <span className={`library-health-pill is-${model.healthTone}`}>
              {model.healthLabel}
            </span>
          )}
          {model.duplicateLabel && (
            <span className={`library-health-pill is-${model.duplicateTone}`}>
              {model.duplicateLabel}
            </span>
          )}
        </div>
      </div>
      <div className="library-list-col library-list-col--facts library-facts-cell">
        <div className="library-row-facts">
          {model.supportingFacts.map((fact) => (
            <span key={fact} className="library-row-fact">
              {fact}
            </span>
          ))}
        </div>
        {model.colorSwatches.length > 0 && (
          <div className="library-row-swatches" aria-label="Color hints">
            {model.colorSwatches.map((hex, i) => (
              <div
                key={i}
                className="library-row-swatch"
                style={{ background: hex }}
                title={hex}
              />
            ))}
          </div>
        )}
      </div>
    </m.div>
  );
}