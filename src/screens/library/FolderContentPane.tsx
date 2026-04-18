import { useState } from "react";
import { Folder } from "lucide-react";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";
import { LibraryCollectionTable } from "./LibraryCollectionTable";
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
  const [prevFolderPath, setPrevFolderPath] = useState<string | null>(null);
  if (folderPath !== prevFolderPath) {
    setPrevFolderPath(folderPath);
    setFilesExpanded(false);
    setRootFilesExpanded(false);
  }

  const segments = folderPath ? folderPath.split("/").filter(Boolean) : [];
  const totalShownFiles = files.length + (folderPath !== null ? rootFiles.length : 0);
  const summaryFileCount =
    folderPath === null ? tree.totalFileCount : totalShownFiles;
  const summary = buildFolderSummary(subfolders.length, summaryFileCount);

  const displayFiles = filesExpanded ? files : files.slice(0, FOLDER_PAGE_SIZE);
  const displayRootFiles = rootFilesExpanded
    ? rootFiles
    : rootFiles.slice(0, FOLDER_PAGE_SIZE);
  const filesHasMore = files.length > FOLDER_PAGE_SIZE;
  const rootFilesHasMore = rootFiles.length > FOLDER_PAGE_SIZE;

  return (
    <div className="library-folder-content-pane">
      <div className="folder-content-header">
        <div className="folder-breadcrumb" aria-label="Folder path">
          {segments.length > 0 ? (
            segments.map((segment, index) => {
              const path = segments.slice(0, index + 1).join("/");
              const isCurrent = index === segments.length - 1;
              return (
                <span key={path} className="folder-breadcrumb-item">
                  <button
                    type="button"
                    className={`folder-breadcrumb-link${isCurrent ? " is-current" : ""}`}
                    onClick={() => onNavigate(path)}
                    aria-current={isCurrent ? "page" : undefined}
                  >
                    {segment}
                  </button>
                  {!isCurrent ? <span className="folder-breadcrumb-sep">/</span> : null}
                </span>
              );
            })
          ) : (
            <span className="folder-breadcrumb-root">All folders</span>
          )}
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
                  {folder.totalFileCount + (folder.rootFiles?.length ?? 0)} files
                </span>
              </button>
            ))}
          </div>
        </section>
      ) : null}

      {/* Root-level files stored directly in this folder with no subfolder */}
      {rootFiles.length > 0 ? (
        <section>
          {folderPath === null ? (
            <>
              <ModsLooseFilesSection
                userView={userView}
                rootFiles={rootFiles}
                selectedFile={selectedFile}
                onSelectFile={onSelectFile}
                displayRootFiles={displayRootFiles}
                rootFilesExpanded={rootFilesExpanded}
                rootFilesHasMore={rootFilesHasMore}
                onToggleRootFilesExpanded={() =>
                  setRootFilesExpanded((v) => !v)
                }
              />
            </>
          ) : (
            <>
              <div className="folder-content-section folder-content-section--loose">
                Loose files
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
            Files
            {filesHasMore && !filesExpanded && (
              <span className="folder-content-section-hint">
                showing {FOLDER_PAGE_SIZE} of {files.length}
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
          {filesHasMore && (
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
  displayRootFiles: LibraryFileRow[];
  rootFilesExpanded: boolean;
  rootFilesHasMore: boolean;
  onToggleRootFilesExpanded: () => void;
}

function ModsLooseFilesSection({
  userView,
  rootFiles,
  selectedFile,
  onSelectFile,
  displayRootFiles,
  rootFilesExpanded,
  rootFilesHasMore,
  onToggleRootFilesExpanded,
}: ModsLooseFilesSectionProps) {
  const groups: Record<string, LibraryFileRow[]> = { Mods: [], Tray: [] };
  for (const file of rootFiles) {
    const src = getSourceRootFromPath(file.path);
    if (src === "Mods" || src === "Tray") {
      groups[src].push(file);
    }
  }

  return (
    <>
      {Object.entries(groups).map(([sourceRoot, allGroupFiles]) =>
        allGroupFiles.length > 0 ? (
          <div key={sourceRoot} className="folder-loose-source-group">
            <div className="folder-content-section folder-content-section--loose">
              Loose files in {sourceRoot}
              <span className="folder-loose-files-badge">{allGroupFiles.length}</span>
            </div>
            <div className="folder-loose-files-hint">
              Stored directly in {sourceRoot} — not organized into subfolders
            </div>
            {/* For loose files section at root: paginate per source group */}
            <LooseFilesGroupTable
              userView={userView}
              allFiles={allGroupFiles}
              selectedFile={selectedFile}
              onSelectFile={onSelectFile}
            />
          </div>
        ) : null,
      )}
    </>
  );
}

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
  const [expanded, setExpanded] = useState(false);
  const displayFiles = expanded ? allFiles : allFiles.slice(0, FOLDER_PAGE_SIZE);
  const hasMore = allFiles.length > FOLDER_PAGE_SIZE;

  return (
    <>
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
      {hasMore && (
        <button
          type="button"
          className="folder-load-more"
          onClick={() => setExpanded(true)}
        >
          Show all {allFiles.length} loose files
        </button>
      )}
    </>
  );
}

function buildFolderSummary(subfolderCount: number, fileCount: number) {
  if (subfolderCount > 0) {
    return `${subfolderCount} ${subfolderCount === 1 ? "subfolder" : "subfolders"}, ${fileCount} ${fileCount === 1 ? "file" : "files"}`;
  }
  return `${fileCount} ${fileCount === 1 ? "file" : "files"}`;
}
