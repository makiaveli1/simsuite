import { useEffect, useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { ChevronDown, ChevronRight, Folder, FolderOpen } from "lucide-react";
import type { FolderNode } from "./folderTree";

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

  return renderNode(tree, activePath, expandedPaths, setExpandedPaths, onNavigate);
}

function renderNode(
  node: FolderNode,
  activePath: string | null,
  expandedPaths: Set<string>,
  setExpandedPaths: Dispatch<SetStateAction<Set<string>>>,
  onNavigate: (path: string) => void,
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
      </button>

      {hasChildren && isExpanded ? (
        <div className="folder-tree-children">
          {node.children.map((child) =>
            renderNode(child, activePath, expandedPaths, setExpandedPaths, onNavigate),
          )}
        </div>
      ) : null}
    </div>
  );
}
