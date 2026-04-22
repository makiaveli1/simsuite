import { useEffect, useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { ChevronDown, ChevronRight, Folder, FolderOpen } from "lucide-react";
import type { FolderNode } from "./folderTree";
import type { LibraryFileRow } from "../../lib/types";
import { computeTreeClue, type FolderTreeClue } from "./libraryDisplay";

interface FolderTreePaneProps {
  tree: FolderNode;
  activePath: string | null;
  onNavigate: (path: string) => void;
}

export function FolderTreePane({ tree, activePath, onNavigate }: FolderTreePaneProps) {
  const defaultExpanded = useMemo(
    () => new Set<string>(tree.name === "Mods" ? [tree.fullPath] : []),
    [tree.fullPath, tree.name],
  );
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(defaultExpanded);

  useEffect(() => {
    setExpandedPaths(defaultExpanded);
  }, [defaultExpanded]);

  useEffect(() => {
    if (!activePath || !(activePath === tree.fullPath || activePath.startsWith(`${tree.fullPath}/`))) {
      return;
    }

    setExpandedPaths((current) => {
      const next = new Set(current);
      const segments = activePath.split("/");
      for (let index = 0; index < segments.length; index += 1) {
        next.add(segments.slice(0, index + 1).join("/"));
      }
      return next;
    });
  }, [activePath, tree.fullPath]);

  return renderNode(tree, activePath, expandedPaths, setExpandedPaths, onNavigate, tree.rootFiles ?? [], tree);
}

function renderNode(
  node: FolderNode,
  activePath: string | null,
  expandedPaths: Set<string>,
  setExpandedPaths: Dispatch<SetStateAction<Set<string>>>,
  onNavigate: (path: string) => void,
  allFiles: LibraryFileRow[],
  treeRoots: FolderNode,
) {
  const hasChildren = node.children.length > 0;
  const isExpanded = expandedPaths.has(node.fullPath);
  const isActive = activePath === node.fullPath;

  return (
    <div key={node.fullPath}>
      <button
        type="button"
        className={`folder-tree-row${isActive ? " folder-tree-row--active is-active" : ""}`}
        onClick={() => onNavigate(node.fullPath)}
        style={{ paddingLeft: `${0.65 + node.depth}rem` }}
      >
        <span className="folder-tree-row__chevron-wrap">
          {hasChildren ? (
            <span
              className="folder-tree-row__chevron"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                setExpandedPaths((current) => {
                  const next = new Set(current);
                  if (next.has(node.fullPath)) {
                    next.delete(node.fullPath);
                  } else {
                    next.add(node.fullPath);
                  }
                  return next;
                });
              }}
              aria-hidden="true"
            >
              {isExpanded ? <ChevronDown size={14} strokeWidth={2} /> : <ChevronRight size={14} strokeWidth={2} />}
            </span>
          ) : (
            <span className="folder-tree-row__chevron folder-tree-row__chevron--empty" aria-hidden="true" />
          )}
        </span>
        <span className="folder-tree-row__icon" aria-hidden="true">
          {isActive ? <FolderOpen size={14} strokeWidth={2} /> : <Folder size={14} strokeWidth={2} />}
        </span>
        <span className="folder-tree-row__label">{node.name}</span>
        <span className="folder-count-badge">
          {node.totalFileCount + (node.rootFiles?.length ?? 0)}
        </span>
        {/* Phase 5ao: tree node clue — dominant kind + issue dot */}
        {(() => {
          const clue = computeTreeClue(allFiles, { fullPath: node.fullPath, sourceLocation: node.fullPath.startsWith("Mods") ? "mods" : node.fullPath.startsWith("Tray") ? "tray" : "mods" });
          if (clue.issueState === "none" && !clue.dominantKind) return null;
          return (
            <span className="folder-tree-node-clue">
              {clue.dominantKind ? (
                <span
                  className={`tree-node-dominant-kind type-pill--${clue.dominantKind.replace(/([A-Z])/g, "-$1").toLowerCase().replace(/^-/, "")}`}
                  title={`Mostly ${clue.dominantKind} (${Math.round(clue.dominantKindShare ?? 0)}%)`}
                >
                  {clue.dominantKind.replace(/([A-Z])/g, " $1").trim()}
                </span>
              ) : null}
              {clue.issueState !== "none" && (
                <span
                  className={`tree-node-issue-dot tree-node-issue-dot--${clue.issueState === "mixed" ? "mixed" : clue.issueState === "warning" ? "warning" : "duplicate"}`}
                  title={
                    clue.issueState === "mixed"
                      ? `${clue.warningCount} warning${clue.warningCount !== 1 ? "s" : ""} · ${clue.duplicateCount} duplicate${clue.duplicateCount !== 1 ? "s" : ""}`
                      : clue.issueState === "warning"
                        ? `${clue.warningCount} warning${clue.warningCount !== 1 ? "s" : ""}`
                        : `${clue.duplicateCount} duplicate${clue.duplicateCount !== 1 ? "s" : ""}`
                  }
                />
              )}
            </span>
          );
        })()}
      </button>

      {hasChildren && isExpanded ? (
        <div className="folder-tree-children">
          {node.children.map((child) =>
            renderNode(child, activePath, expandedPaths, setExpandedPaths, onNavigate, allFiles, treeRoots),
          )}
        </div>
      ) : null}
    </div>
  );
}
