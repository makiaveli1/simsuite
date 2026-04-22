import { useState, useMemo, useEffect } from "react";
import React from "react";
import { Folder } from "lucide-react";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";
import { LibraryCollectionTable } from "./LibraryCollectionTable";
import { VirtualizedLooseFiles } from "./VirtualizedLooseFiles";
import type { FolderNode } from "./folderTree";

interface FolderContentPaneProps {
  userView: UserView;
  folderPath: string | null;
  subfolders: FolderNode[];
  files: LibraryFileRow[];
  rootFiles: LibraryFileRow[];
  tree: FolderNode;
  onNavigate: (path: string) => void;
  onSelectFile: (file: LibraryFileRow) => void;
  selectedFile: FileDetail | null;
}

// Phase 5z: paginate folder content to avoid rendering thousands of DOM nodes.
// Initial page shows up to FOLDER_PAGE_SIZE items. Users can expand.
const FOLDER_PAGE_SIZE = 100;

export function FolderContentPane({
  userView,
  folderPath,
  subfolders,
  files,
  rootFiles,
  tree,
  onNavigate,
  onSelectFile,
  selectedFile,
}: FolderContentPaneProps) {
  // Phase 5z: per-section pagination — reset to 0 when navigating to a new folder.
  const [filesExpanded, setFilesExpanded] = useState(false);
  const [rootFilesExpanded, setRootFilesExpanded] = useState(false);

  // Reset expanded state when navigating to a different folder.
  // useState initial values are only used on mount — this effect keeps them in sync.
  useEffect(() => {
    setFilesExpanded(false);
    setRootFilesExpanded(false);
  }, [folderPath]);

  const pageSize = FOLDER_PAGE_SIZE;

  const filesHasMore = files.length > pageSize;
  const displayFiles = filesExpanded ? files : files.slice(0, pageSize);

  const rootFilesHasMore = rootFiles.length > pageSize;
  const displayRootFiles = rootFilesExpanded ? rootFiles : rootFiles.slice(0, pageSize);

  const summary = [
    subfolders.length > 0 ? `${subfolders.length} subfolders` : null,
    files.length > 0 ? `${files.length} files` : null,
    rootFiles.length > 0 ? `${rootFiles.length} loose` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div className="library-folder-content-pane">
      <div className="folder-content-header">
        <div className="folder-content-title">
          <Folder size={15} strokeWidth={2} />
          <span>{folderPath ?? "Library"}</span>
        </div>
        <div className="folder-content-summary">{summary}</div>
      </div>

      {subfolders.length > 0 ? (
        <section>
          <div className="folder-content-section">Folders</div>
          <div className="folder-row-list">
            {subfolders.map((folder) => (
              <button
                key={folder.fullPath}
                type="button"
                className="folder-row"
                onClick={() => onNavigate(folder.fullPath)}
              >
                <span className="folder-row__icon" aria-hidden="true">
                  <Folder size={15} strokeWidth={2} />
                </span>
                <span className="folder-row__name">{folder.name}</span>
                <span className="folder-row__count">
                  {/* Phase 5ab: rootFiles are not part of the tree structure — count only tree files */}
                  {folder.totalFileCount} files
                </span>
              </button>
            ))}
          </div>
        </section>
      ) : null}

      {/* Root-level files stored directly in this folder with no subfolder */}
      {/* At root: only show if there are Mods depth-0 files. Tray root files are in the tree. */}
      {/* At folder level: show all root files for that folder. */}
      {rootFiles.length > 0 && (folderPath !== null || rootFiles.some((f) => getSourceRootFromPath(f.path) === "Mods")) ? (
        <section>
          {folderPath === null ? (
            <ModsLooseFilesSection
              userView={userView}
              rootFiles={rootFiles}
              selectedFile={selectedFile}
              onSelectFile={onSelectFile}
            />
          ) : (
            <>
              <div className="folder-content-section folder-content-section--loose">
                Loose files in {folderPath ?? "Mods"}
                <span className="folder-loose-files-badge">{rootFiles.length}</span>
              </div>
              <div className="folder-loose-files-hint">
                Stored directly in {folderPath} — not yet organized into subfolders
              </div>
              <LibraryCollectionTable
                userView={userView}
                rows={displayRootFiles}
                selectedId={selectedFile?.id ?? null}
                selectedIds={EMPTY_SELECTION}
                page={0}
                totalPages={1}
                onSelect={onSelectFile}
                onToggleSelect={() => undefined}
                onPrevPage={() => undefined}
                onNextPage={() => undefined}
                enableSelection={false}
                showPagination={false}
              />
              {rootFilesHasMore && (
                <button
                  type="button"
                  className="folder-load-more"
                  onClick={() => setRootFilesExpanded(true)}
                >
                  Show all {rootFiles.length} loose files
                </button>
              )}
            </>
          )}
        </section>
      ) : null}

      {files.length > 0 ? (
        <section>
          <div className="folder-content-section">
            Files in subfolders
            {filesHasMore && !filesExpanded && (
              <span className="folder-content-section-hint">
                showing {pageSize} of {files.length}
              </span>
            )}
          </div>
          <LibraryCollectionTable
            userView={userView}
            rows={displayFiles}
            selectedId={selectedFile?.id ?? null}
            selectedIds={EMPTY_SELECTION}
            page={0}
            totalPages={1}
            onSelect={onSelectFile}
            onToggleSelect={() => undefined}
            onPrevPage={() => undefined}
            onNextPage={() => undefined}
            enableSelection={false}
            showPagination={false}
          />
          {filesHasMore && !filesExpanded && (
            <button
              type="button"
              className="folder-load-more"
              onClick={() => setFilesExpanded(true)}
            >
              Show all {files.length} files
            </button>
          )}
        </section>
      ) : null}

      {subfolders.length === 0 && files.length === 0 && rootFiles.length === 0 ? (
        <div className="library-list-empty">This folder is empty.</div>
      ) : null}
    </div>
  );
}

const EMPTY_SELECTION = new Set<number>();

// ── Helpers ──────────────────────────────────────────────────────────────────

const SOURCE_ROOT_NAMES = new Set(["Mods", "Tray"]);

function getSourceRootFromPath(path: string): string {
  const normalized = path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
  const segments = normalized.split("/").filter(Boolean);
  const idx = segments.findIndex((s) => SOURCE_ROOT_NAMES.has(s));
  return idx < 0 ? "" : segments[idx];
}

// ── ModsLooseFilesSection ─────────────────────────────────────────────────────

interface ModsLooseFilesSectionProps {
  userView: UserView;
  rootFiles: LibraryFileRow[];
  selectedFile: FileDetail | null;
  onSelectFile: (file: LibraryFileRow) => void;
}

// Phase 5ag: Only Mods root-level files are surfaced as a loose-files section.
// Tray root files are NOT surfaced separately — Tray is represented in the folder tree
// and Tray root files are accessible there. This avoids redundant duplication at root level.
const ModsLooseFilesSection = React.memo(function ModsLooseFilesSection({
  userView,
  rootFiles,
  selectedFile,
  onSelectFile,
}: ModsLooseFilesSectionProps) {
  // Filter to Mods only — Tray root files are shown via the folder tree, not here
  const modsFiles = useMemo(
    () => rootFiles.filter((f) => getSourceRootFromPath(f.path) === "Mods"),
    [rootFiles]
  );
  if (!modsFiles.length) return null;

  return (
    <div className="folder-loose-source-group">
      <div className="folder-content-section folder-content-section--loose">
        Loose files in Mods
        <span className="folder-loose-files-badge">{modsFiles.length}</span>
      </div>
      <div className="folder-loose-files-hint">
        Stored directly in Mods — not organized into subfolders
      </div>
      <LooseFilesGroupTable
        userView={userView}
        allFiles={modsFiles}
        selectedFile={selectedFile}
        onSelectFile={onSelectFile}
      />
    </div>
  );
});

// ── LooseFilesGroupTable ──────────────────────────────────────────────────────

interface LooseFilesGroupTableProps {
  userView: UserView;
  allFiles: LibraryFileRow[];
  selectedFile: FileDetail | null;
  onSelectFile: (file: LibraryFileRow) => void;
}

function LooseFilesGroupTable({
  userView,
  allFiles,
  selectedFile,
  onSelectFile,
}: LooseFilesGroupTableProps) {
  return (
    <VirtualizedLooseFiles
      userView={userView}
      allFiles={allFiles}
      selectedFile={selectedFile}
      onSelectFile={onSelectFile}
    />
  );
}
