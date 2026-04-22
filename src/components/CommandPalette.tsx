import { useEffect, useRef, useState, useCallback } from "react";
import { api } from "../lib/api";
import type {
  DownloadsInboxItem,
  LibraryFileRow,
  LibraryWatchListItem,
} from "../lib/types";

// ─── Types ───────────────────────────────────────────────────────────────────

export interface CommandPaletteResult {
  id: string;
  type: "download" | "library" | "update";
  name: string;
  sub: string;
  laneOrSource: string;
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigate: (screen: "downloads" | "library" | "updates", id?: number) => void;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function isLibraryRow(item: unknown): item is LibraryFileRow {
  return (
    typeof item === "object" &&
    item !== null &&
    "filename" in item
  );
}

function isWatchItem(item: unknown): item is LibraryWatchListItem {
  return (
    typeof item === "object" &&
    item !== null &&
    "watchResult" in item
  );
}

function normalizeDownload(item: DownloadsInboxItem): CommandPaletteResult {
  return {
    id: `dl-${item.id}`,
    type: "download",
    name: item.displayName,
    sub: item.creatorName ?? "",
    laneOrSource: item.status,
  };
}

function normalizeLibrary(item: LibraryFileRow): CommandPaletteResult {
  return {
    id: `lib-${item.id}`,
    type: "library",
    name: item.filename,
    sub: item.subtype ?? item.kind ?? "",
    laneOrSource: item.creator ?? "",
  };
}

function normalizeUpdate(item: LibraryWatchListItem): CommandPaletteResult {
  return {
    id: `upd-${item.fileId}`,
    type: "update",
    name: item.filename,
    sub: item.creator ?? "",
    laneOrSource: item.watchResult.latestVersion ?? item.installedVersion ?? "",
  };
}

// ─── Component ───────────────────────────────────────────────────────────────

export function CommandPalette({ isOpen, onClose, onNavigate }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [downloads, setDownloads] = useState<CommandPaletteResult[]>([]);
  const [library, setLibrary] = useState<CommandPaletteResult[]>([]);
  const [updates, setUpdates] = useState<CommandPaletteResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  // Build flat list for keyboard nav
  const allResults: CommandPaletteResult[] = [...downloads, ...library, ...updates];

  // Reset when opened
  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setDownloads([]);
      setLibrary([]);
      setUpdates([]);
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 20);
    }
  }, [isOpen]);

  // Search with debounce
  useEffect(() => {
    if (!query.trim()) {
      setDownloads([]);
      setLibrary([]);
      setUpdates([]);
      setSelectedIndex(0);
      return;
    }

    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      const q = query.trim();

      const [dlRaw, libRaw, updRaw] = await Promise.all([
        api.getDownloadsInbox({ search: q, limit: 5 }).catch(() => null),
        api.listLibraryFiles({ search: q, limit: 5, includePreviews: false }).catch(() => null),
        api.listLibraryWatchItems("all", 30).catch(() => null),
      ]);

      // Downloads
      const dlItems: DownloadsInboxItem[] = dlRaw?.items ?? [];
      setDownloads(dlItems.slice(0, 5).map(normalizeDownload));

      // Library
      const libItems: LibraryFileRow[] = libRaw?.items ?? [];
      setLibrary(libItems.slice(0, 5).map(normalizeLibrary));

      // Updates – no text-search API exists, so filter client-side
      const updItems: LibraryWatchListItem[] = updRaw?.items ?? [];
      const filteredUpdates = updItems
        .filter((item) => {
          const hay = `${item.filename} ${item.creator ?? ""}`.toLowerCase();
          return hay.includes(q.toLowerCase());
        })
        .slice(0, 5)
        .map(normalizeUpdate);
      setUpdates(filteredUpdates);

      setSelectedIndex(0);
    }, 150);

    return () => clearTimeout(debounceRef.current);
  }, [query]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, allResults.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const item = allResults[selectedIndex];
        if (item) {
          const id =
            item.type === "download"
              ? parseInt(item.id.replace("dl-", ""), 10)
              : item.type === "library"
              ? parseInt(item.id.replace("lib-", ""), 10)
              : parseInt(item.id.replace("upd-", ""), 10);
          onNavigate(item.type as "downloads" | "library" | "updates", id);
        }
      } else if (e.key === "Escape") {
        onClose();
      }
    },
    [allResults, selectedIndex, onNavigate, onClose],
  );

  if (!isOpen) return null;

  // Compute which flat index each section starts at
  const downloadsStart = 0;
  const libraryStart = downloads.length;
  const updatesStart = libraryStart + library.length;

  const flatIndexOf = (sectionStart: number, localIdx: number) =>
    sectionStart + localIdx;

  return (
    <div
      className="command-palette-backdrop"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="command-palette" onKeyDown={handleKeyDown}>
        {/* Search input */}
        <div className="command-palette-input-wrap">
          <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            style={{ opacity: 0.5, flexShrink: 0 }}
          >
            <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" strokeWidth="1.5" />
            <path d="M10 10L13.5 13.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          <input
            ref={inputRef}
            className="command-palette-input"
            placeholder="Search downloads, library, updates…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoComplete="off"
            spellCheck={false}
          />
          {query && (
            <button
              onClick={() => setQuery("")}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                color: "var(--text-dim)",
                padding: "2px 4px",
                fontSize: "12px",
                lineHeight: 1,
              }}
              title="Clear"
            >
              ✕
            </button>
          )}
        </div>

        {/* Results */}
        <div className="command-palette-results">
          {/* Downloads section */}
          {downloads.length > 0 && (
            <div className="command-palette-section">
              <div className="command-palette-section-title">Downloads</div>
              {downloads.map((item, i) => (
                <div
                  key={item.id}
                  className={`command-palette-row ${
                    selectedIndex === flatIndexOf(downloadsStart, i)
                      ? "is-selected"
                      : ""
                  }`}
                  onClick={() =>
                    onNavigate("downloads", parseInt(item.id.replace("dl-", ""), 10))
                  }
                  onMouseEnter={() => setSelectedIndex(flatIndexOf(downloadsStart, i))}
                >
                  <div className="command-palette-row-icon">
                    <DownloadIcon />
                  </div>
                  <div className="command-palette-row-name">{item.name}</div>
                  {item.sub && (
                    <div className="command-palette-row-sub">{item.sub}</div>
                  )}
                  <div className="command-palette-row-sub">{item.laneOrSource}</div>
                </div>
              ))}
            </div>
          )}

          {/* Library section */}
          {library.length > 0 && (
            <div className="command-palette-section">
              <div className="command-palette-section-title">Library</div>
              {library.map((item, i) => (
                <div
                  key={item.id}
                  className={`command-palette-row ${
                    selectedIndex === flatIndexOf(libraryStart, i)
                      ? "is-selected"
                      : ""
                  }`}
                  onClick={() =>
                    onNavigate("library", parseInt(item.id.replace("lib-", ""), 10))
                  }
                  onMouseEnter={() => setSelectedIndex(flatIndexOf(libraryStart, i))}
                >
                  <div className="command-palette-row-icon">
                    <LibraryIcon />
                  </div>
                  <div className="command-palette-row-name">{item.name}</div>
                  {item.sub && (
                    <div className="command-palette-row-sub">{item.sub}</div>
                  )}
                  {item.laneOrSource && (
                    <div className="command-palette-row-sub">{item.laneOrSource}</div>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Updates section */}
          {updates.length > 0 && (
            <div className="command-palette-section">
              <div className="command-palette-section-title">Updates</div>
              {updates.map((item, i) => (
                <div
                  key={item.id}
                  className={`command-palette-row ${
                    selectedIndex === flatIndexOf(updatesStart, i)
                      ? "is-selected"
                      : ""
                  }`}
                  onClick={() =>
                    onNavigate("updates", parseInt(item.id.replace("upd-", ""), 10))
                  }
                  onMouseEnter={() => setSelectedIndex(flatIndexOf(updatesStart, i))}
                >
                  <div className="command-palette-row-icon">
                    <UpdateIcon />
                  </div>
                  <div className="command-palette-row-name">{item.name}</div>
                  {item.sub && (
                    <div className="command-palette-row-sub">{item.sub}</div>
                  )}
                  {item.laneOrSource && (
                    <div className="command-palette-row-sub">
                      {item.laneOrSource}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Empty state */}
          {query.trim() &&
            downloads.length === 0 &&
            library.length === 0 &&
            updates.length === 0 && (
              <div
                style={{
                  padding: "24px 16px",
                  textAlign: "center",
                  color: "var(--text-dim)",
                  fontSize: "13px",
                }}
              >
                No results for{" "}
                <strong style={{ color: "var(--text)" }}>"{query}"</strong>
              </div>
            )}
        </div>

        {/* Keyboard hints */}
        <div className="command-palette-hint">
          <span className="command-palette-hint-item">
            <kbd className="command-palette-hint-key">↑↓</kbd> navigate
          </span>
          <span className="command-palette-hint-item">
            <kbd className="command-palette-hint-key">↵</kbd> open
          </span>
          <span className="command-palette-hint-item">
            <kbd className="command-palette-hint-key">esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  );
}

// ─── Tiny icons (inline SVG) ─────────────────────────────────────────────────

function DownloadIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M7 1v8M4 6l3 3 3-3M2 11h10"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function LibraryIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect
        x="1.5"
        y="1.5"
        width="11"
        height="11"
        rx="2"
        stroke="currentColor"
        strokeWidth="1.4"
      />
      <path d="M4.5 5h5M4.5 7.5h5M4.5 10h3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function UpdateIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M2 7c0-2.76 2.24-5 5-5s5 2.24 5 5-2.24 5-5 5"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <path
        d="M7 4v3l2 1.5"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
