import type { LibraryFileRow } from "../../lib/types";

/** Cached tree — build once, reuse for all getFolderContents calls within the same dataset. */
let _cachedFiles: LibraryFileRow[] | null = null;
let _cachedTree: { mods: FolderNode; tray: FolderNode } | null = null;

/**
 * Returns a cached tree for the given file list.
 * Rebuilds only when `files` reference changes.
 * This avoids a full O(n) `buildFolderTree` traversal on every folder click.
 */
export function getCachedTree(
  files: LibraryFileRow[],
): { mods: FolderNode; tray: FolderNode } {
  if (_cachedTree && _cachedFiles === files) {
    return _cachedTree;
  }
  _cachedFiles = files;
  _cachedTree = buildFolderTree(files);
  return _cachedTree;
}

/** Clears the cached tree — call when filters or data change. */
export function clearCachedTree(): void {
  _cachedTree = null;
  _cachedFiles = null;
}

export interface FolderNode {
  name: string;
  fullPath: string;
  depth: number;
  children: FolderNode[];
  directFileCount: number;
  totalFileCount: number;
  childFolderCount: number;
  /** Root-level (depth-0) files directly in this folder, not in any subfolder. */
  rootFiles?: LibraryFileRow[];
}

interface MutableFolderNode extends FolderNode {
  children: MutableFolderNode[];
}

const ROOT_NAMES = new Set(["Mods", "Tray"]);

export function getRelativePath(path: string): string {
  return path
    .replace(/\\/g, "/")
    .replace(/^[a-zA-Z]:/, "")
    .replace(/^\/+/, "");
}

function createNode(name: string, fullPath: string, depth: number): MutableFolderNode {
  return {
    name,
    fullPath,
    depth,
    children: [],
    directFileCount: 0,
    totalFileCount: 0,
    childFolderCount: 0,
  };
}

function findRootSegments(path: string): string[] | null {
  const segments = getRelativePath(path).split("/").filter(Boolean);
  const rootIndex = segments.findIndex((segment) => ROOT_NAMES.has(segment));
  if (rootIndex < 0) {
    return null;
  }
  return segments.slice(rootIndex);
}

function getFolderSegments(path: string): string[] | null {
  const rootedSegments = findRootSegments(path);
  if (!rootedSegments || rootedSegments.length === 0) {
    return null;
  }
  return rootedSegments.slice(0, -1);
}

function finalizeNode(node: MutableFolderNode): FolderNode {
  const children = node.children
    .map(finalizeNode)
    .filter((child) => child.totalFileCount > 0)
    .sort((left, right) => left.name.localeCompare(right.name, undefined, { sensitivity: "base" }));

  const totalFileCount = node.directFileCount + children.reduce((sum, child) => sum + child.totalFileCount, 0);

  return {
    ...node,
    children,
    totalFileCount,
    childFolderCount: children.length,
  };
}

export function buildFolderTree(files: LibraryFileRow[]): { mods: FolderNode; tray: FolderNode } {
  const mods = createNode("Mods", "Mods", 0);
  const tray = createNode("Tray", "Tray", 0);
  const roots: Record<string, MutableFolderNode> = {
    Mods: mods,
    Tray: tray,
  };

  for (const file of files) {
    const folderSegments = getFolderSegments(file.path);
    if (!folderSegments?.length) {
      continue;
    }

    let current = roots[folderSegments[0]];
    if (!current) {
      continue;
    }

    for (let index = 1; index < folderSegments.length; index += 1) {
      const segment = folderSegments[index];
      const fullPath = folderSegments.slice(0, index + 1).join("/");
      let next = current.children.find((child) => child.fullPath === fullPath);
      if (!next) {
        next = createNode(segment, fullPath, index);
        current.children.push(next);
      }
      current = next;
    }

    current.directFileCount += 1;
  }

  return {
    mods: finalizeNode(mods),
    tray: finalizeNode(tray),
  };
}

// Returns root-level (depth=0) files for a given source root.
// Root files are those stored directly in the root folder (e.g. Mods\) with no subfolder.
// These are NOT part of the folder tree structure and must be surfaced separately.
export function getRootFiles(
  rootName: string, // "Mods" | "Tray"
  files: LibraryFileRow[],
): LibraryFileRow[] {
  return files.filter((file) => {
    const folderSegments = getFolderSegments(file.path);
    // root-level files have folderSegments.length === 1 (only the root itself, no subfolders)
    // OR folderSegments is null/empty (file directly in root, no subfolder path)
    return (
      folderSegments !== null &&
      folderSegments.length === 1 &&
      folderSegments[0] === rootName
    );
  });
}

export function getFolderContents(
  folderPath: string,
  files: LibraryFileRow[],
  cachedTree?: { mods: FolderNode; tray: FolderNode },
): { subfolders: FolderNode[]; files: LibraryFileRow[]; rootFiles: LibraryFileRow[] } {
  const folderTree = cachedTree ?? getCachedTree(files);
  const roots = [folderTree.mods, folderTree.tray];
  const activeNode = roots
    .flatMap((root) => [root, ...collectDescendants(root)])
    .find((node) => node.fullPath === folderPath);

  const directFiles = files.filter((file) => {
    const folderSegments = getFolderSegments(file.path);
    return folderSegments?.join("/") === folderPath;
  });

  // Root files: depth-0 files stored directly in this folder with no subfolder.
  // E.g. the 9,777 files stored directly in "Mods" with no subfolder path.
  const rootFiles: LibraryFileRow[] =
    activeNode != null
      ? getRootFiles(activeNode.name, files)
      : [];

  // Phase 5ab dedupe: when at a root node (Mods/Tray), directFiles and rootFiles
  // both contain the same depth-0 files. Deduplicate so each file appears once.
  const rootFileIds = new Set(rootFiles.map((f) => f.id));
  const dedupedFiles = directFiles.filter((f) => !rootFileIds.has(f.id));

  return {
    subfolders: activeNode?.children ?? [],
    files: dedupedFiles,
    rootFiles,
  };
}

function collectDescendants(node: FolderNode): FolderNode[] {
  return node.children.flatMap((child) => [child, ...collectDescendants(child)]);
}
