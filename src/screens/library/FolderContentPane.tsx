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
  const segments = folderPath ? folderPath.split("/").filter(Boolean) : [];
  // Root files add to the visible file count when inside a folder.
  // At root level, tree.totalFileCount already includes root files (updated in syntheticRoot).
  const totalShownFiles = files.length + (folderPath !== null ? rootFiles.length : 0);
  const summaryFileCount =
    folderPath === null ? tree.totalFileCount : totalShownFiles;
  const summary = buildFolderSummary(subfolders.length, summaryFileCount);

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

      {/* Root-level files stored directly in this folder with no subfolder (e.g. Mods\\filename.package) */}
      {rootFiles.length > 0 ? (
        <section>
          {folderPath === null ? (
            // At root: group loose files by source root (Mods / Tray)
            <>
              <ModsLooseFilesSection
                userView={userView}
                rootFiles={rootFiles}
                selectedFile={selectedFile}
                onSelectFile={onSelectFile}
              />
            </>
          ) : (
            // Inside a specific folder: single "Loose files" section
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
                rows={rootFiles}
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
            </>
          )}
        </section>
      ) : null}

      {files.length > 0 ? (
        <section>
          <div className="folder-content-section">Files</div>
          <LibraryCollectionTable
            userView={userView}
            rows={files}
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
// Shows root-level (depth-0) files grouped by source root (Mods / Tray).
// These files are stored directly in the game folder with no subfolder.

interface ModsLooseFilesSectionProps {
  userView: UserView;
  rootFiles: LibraryFileRow[];
  selectedFile: FileDetail | null;
  onSelectFile: (file: LibraryFileRow) => void;
}

function ModsLooseFilesSection({
  userView,
  rootFiles,
  selectedFile,
  onSelectFile,
}: ModsLooseFilesSectionProps) {
  // Group root files by source root (Mods / Tray)
  const groups: Record<string, LibraryFileRow[]> = { Mods: [], Tray: [] };
  for (const file of rootFiles) {
    const src = getSourceRootFromPath(file.path);
    if (src === "Mods" || src === "Tray") {
      groups[src].push(file);
    }
  }

  return (
    <>
      {Object.entries(groups).map(([sourceRoot, files]) =>
        files.length > 0 ? (
          <div key={sourceRoot} className="folder-loose-source-group">
            <div className="folder-content-section folder-content-section--loose">
              Loose files in {sourceRoot}
              <span className="folder-loose-files-badge">{files.length}</span>
            </div>
            <div className="folder-loose-files-hint">
              Stored directly in {sourceRoot} — not organized into subfolders
            </div>
            <LibraryCollectionTable
              userView={userView}
              rows={files}
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
          </div>
        ) : null,
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
