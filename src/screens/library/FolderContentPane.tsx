import { Folder } from "lucide-react";
import type { FileDetail, LibraryFileRow, UserView } from "../../lib/types";
import { LibraryCollectionTable } from "./LibraryCollectionTable";
import type { FolderNode } from "./folderTree";

interface FolderContentPaneProps {
  userView: UserView;
  folderPath: string | null;
  subfolders: FolderNode[];
  files: LibraryFileRow[];
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
  tree,
  onNavigate,
  onSelectFile,
  selectedFile,
}: FolderContentPaneProps) {
  const segments = folderPath ? folderPath.split("/").filter(Boolean) : [];
  const summary = buildFolderSummary(subfolders.length, folderPath === null ? tree.totalFileCount : files.length);

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
                <span className="folder-row__count">{folder.totalFileCount} files</span>
              </button>
            ))}
          </div>
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

      {subfolders.length === 0 && files.length === 0 ? (
        <div className="library-list-empty">This folder is empty.</div>
      ) : null}
    </div>
  );
}

const EMPTY_SELECTION = new Set<number>();

function buildFolderSummary(subfolderCount: number, fileCount: number) {
  if (subfolderCount > 0) {
    return `${subfolderCount} ${subfolderCount === 1 ? "subfolder" : "subfolders"}, ${fileCount} ${fileCount === 1 ? "file" : "files"}`;
  }

  return `${fileCount} ${fileCount === 1 ? "file" : "files"}`;
}
