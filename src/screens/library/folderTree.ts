import type { LibraryFileRow } from "../../lib/types";

/** Cached tree — build once, reuse for all getFolderContents calls within the same dataset. */
let _cachedFiles: LibraryFileRow[] | null = null;
let _cachedTree: { mods: FolderNode; tray: FolderNode } | null = null;
/** Phase 5ah: id → file lookup for O(1) file retrieval from any node. */
let _cachedIdMap: Map<number, LibraryFileRow> | null = null;

export function getCachedTree(
  files: LibraryFileRow[],
): { mods: FolderNode; tray: FolderNode } {
  if (_cachedTree && _cachedFiles === files) {
    return _cachedTree;
  }
  _cachedFiles = files;
  _cachedIdMap = new Map(files.map((f) => [f.id, f])); // Phase 5ah: build id map early
  _cachedTree = buildFolderTree(files);
  return _cachedTree;
}

export function clearCachedTree(): void {
  _cachedTree = null;
  _cachedFiles = null;
  _cachedIdMap = null;
}

/** Returns all files directly contained in a folder node (not descendants). */
function getNodeFiles(node: FolderNode): LibraryFileRow[] {
  if (!node.files) return [];
  if (_cachedIdMap === null) {
    _cachedIdMap = new Map((_cachedFiles ?? []).map((f) => [f.id, f]));
  }
  return node.files
    .map((id) => _cachedIdMap!.get(id))
    .filter((f): f is LibraryFileRow => f != null);
}

/**
 * Collects all file IDs in this node AND all descendant nodes.
 * Used by getFolderContents to return the full file list for a folder.
 */
function collectAllFileIds(node: FolderNode): number[] {
  const ids: number[] = [...(node.files ?? [])];
  for (const child of node.children) {
    ids.push(...collectAllFileIds(child));
  }
  return ids;
}

export interface FolderNode {
  name: string;
  fullPath: string;
  depth: number;
  children: FolderNode[];
  directFileCount: number;
  totalFileCount: number;
  childFolderCount: number;
  /** Phase 5ah: ids of files stored directly in this folder (not in subfolders). */
  files?: number[];
  /** Optional root-level files attached at render time for badge display. */
  rootFiles?: import("../../lib/types").LibraryFileRow[];
}

interface MutableFolderNode extends FolderNode {
  children: MutableFolderNode[];
}

const ROOT_NAMES = new Set(["Mods", "Tray"]);

/**
 * Fallback: extract the first segment from paths that have no Mods/Tray root.
 * Handles absolute Windows paths that include the full user profile path
 * (e.g. "C:\\Users\\Player\\Documents\\...\\filename.package") — the first
 * segment after drive-letter normalisation is treated as the effective root.
 */
function extractFallbackRoot(path: string): string | null {
  const normalized = path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
  const first = normalized.split("/").filter(Boolean)[0];
  return first && ROOT_NAMES.has(first) ? first : null;
}

export function getRelativePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
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
  if (rootIndex < 0) return null;
  return segments.slice(rootIndex);
}

function getFolderSegments(path: string): string[] | null {
  const rootedSegments = findRootSegments(path);
  if (!rootedSegments || rootedSegments.length === 0) return null;
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
    files: node.files ?? [],
  };
}

export function buildFolderTree(files: LibraryFileRow[]): { mods: FolderNode; tray: FolderNode } {
  const mods = createNode("Mods", "Mods", 0);
  const tray = createNode("Tray", "Tray", 0);
  const roots: Record<string, MutableFolderNode> = {
    Mods: mods,
    Tray: tray,
  };
  // Phase 5ah: Build id map for O(1) file retrieval
  const idMap = new Map<number, LibraryFileRow>();
  for (const file of files) {
    idMap.set(file.id, file);
  }
  _cachedIdMap = idMap;

  for (const file of files) {
    const folderSegments = getFolderSegments(file.path);

    // Phase 5ah-fix: If getFolderSegments returns null or [] the path has no
    // Mods/Tray root segment.  This happens with mock data that uses absolute
    // Windows paths ("C:\\Users\\...\\Mods\\sub\\file.package") where the
    // path before the root folder is included.  Fall back to the first segment
    // that matches ROOT_NAMES so these files still get placed in the tree.
    if (!folderSegments?.length) {
      const fallbackRoot = extractFallbackRoot(file.path);
      if (!fallbackRoot) continue; // truly unrecognised path — skip it
      const normalized = file.path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
      // Re-run getFolderSegments using the normalised path so the tree walk
      // starts from the recognised root segment
      const fullSegs = normalized.split("/").filter(Boolean);
      const rootIdx = fullSegs.findIndex((s) => ROOT_NAMES.has(s));
      if (rootIdx < 0) continue;
      // Patch __folderSegments so the rest of the loop works normally.
      // slice(rootIdx + 1) strips the root segment itself, leaving only
      // the path UNDER the root (depth-0 files → remaining path is just the
      // filename, depth-1 files → "subfolder/filename").
      // eslint-disable-next-line no-param-reassign
      (file as { __folderSegments?: string[] }).__folderSegments =
        fullSegs.slice(rootIdx + 1);
    }

    const workingSegments = (file as { __folderSegments?: string[] }).__folderSegments ?? folderSegments;
    if (!workingSegments?.length) continue;

    // Walk the path and assign the file to the DEEPEST (innermost) node only.
    // Depth-0 files (e.g. "Mods/foo.package" → segments=["Mods"])
    // get assigned to the root node (Mods/Tray) via roots[folderSegments[0]].
    // Files in subfolders (e.g. "Mods/sub/foo.package" → segments=["Mods","sub"])
    // go ONLY to "Mods/sub", not to Mods.
    // This ensures getFolderContents correctly separates direct vs subfolder files.
    let current = roots[workingSegments[0]];
    if (!current) continue;

    for (let index = 1; index < workingSegments.length; index += 1) {
      const segment = workingSegments[index];
      const fullPath = workingSegments.slice(0, index + 1).join("/");
      let next = current.children.find((child) => child.fullPath === fullPath);
      if (!next) {
        next = createNode(segment, fullPath, index);
        current.children.push(next);
      }
      current = next;
    }


    // Assign to the deepest node only (not ancestors)
    if (!current.files) current.files = [];
    current.files.push(file.id);
    current.directFileCount += 1;
  }

  return {
    mods: finalizeNode(mods),
    tray: finalizeNode(tray),
  };
}

// Returns root-level (depth-0) files for a given source root.
export function getRootFiles(
  rootName: string,
  files: LibraryFileRow[],
): LibraryFileRow[] {
  return files.filter((file) => {
    const folderSegments = getFolderSegments(file.path);
    return (
      folderSegments !== null &&
      folderSegments.length === 1 &&
      folderSegments[0] === rootName
    );
  });
}

/**
 * Phase 5ah: Returns all files in a folder node and all its descendant folders.
 * Uses the pre-built `files` arrays on each node for O(1) retrieval per node.
 * Recurses down the folder tree — O(depth) per lookup, not O(n).
 */
function getAllFilesForNode(node: FolderNode): LibraryFileRow[] {
  if (_cachedIdMap === null && _cachedFiles !== null) {
    _cachedIdMap = new Map(_cachedFiles.map((f) => [f.id, f]));
  }
  if (!_cachedIdMap) return [];
  const allIds = collectAllFileIds(node);
  return allIds
    .map((id) => _cachedIdMap!.get(id))
    .filter((f): f is LibraryFileRow => f != null);
}

function findFolderNodeByPath(nodes: FolderNode[], folderPath: string): FolderNode | undefined {
  for (const node of nodes) {
    if (node.fullPath === folderPath) {
      return node;
    }
    const match = findFolderNodeByPath(node.children, folderPath);
    if (match) {
      return match;
    }
  }
  return undefined;
}

export function getFolderContents(
  folderPath: string | null,
  files: LibraryFileRow[],
  cachedTree?: { mods: FolderNode; tray: FolderNode },
): { subfolders: FolderNode[]; files: LibraryFileRow[]; rootFiles: LibraryFileRow[] } {
  // Reject stale Rust preloader tree (no files) — loadTreeRows builds a proper tree with files
  const folderTree = (cachedTree && cachedTree.mods.files !== undefined) ? cachedTree : getCachedTree(files);
  const roots: FolderNode[] = [folderTree.mods, folderTree.tray];

  // Phase 5aj: removed debug console.log spam — these fired on every navigation,
  // adding string allocation and DevTools overhead to the hot path.
  // Root level: Mods and Tray as subfolders
  if (folderPath === null) {
    const rootFiles = getAllFilesForNode(folderTree.mods).concat(getAllFilesForNode(folderTree.tray));
    const subfolders = roots.filter((r) => r.totalFileCount > 0);
    return { subfolders, files: [], rootFiles };
  }

  const activeNode = findFolderNodeByPath(roots, folderPath);

  if (!activeNode) {
    return { subfolders: [], files: [], rootFiles: [] };
  }

  // Phase 5ah: Use pre-computed file ids on the node — O(depth) not O(n)
  const allFiles = getAllFilesForNode(activeNode!);

  // Phase 5ab dedupe: if at a root node (Mods/Tray), separate direct from subfolder files
  const rootFiles = allFiles.filter((f) => {
    const folderSegments = getFolderSegments(f.path);
    return folderSegments !== null && folderSegments.length === 1 && folderSegments[0] === activeNode!.name;
  });
  const rootFileIds = new Set(rootFiles.map((f) => f.id));
  const subfolderFiles = allFiles.filter((f) => !rootFileIds.has(f.id));

  return {
    subfolders: activeNode.children,
    files: subfolderFiles,
    rootFiles,
  };
}
