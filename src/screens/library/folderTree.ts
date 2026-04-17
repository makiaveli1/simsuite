import type { LibraryFileRow } from "../../lib/types";

export interface FolderNode {
  name: string;
  fullPath: string;
  depth: number;
  children: FolderNode[];
  directFileCount: number;
  totalFileCount: number;
  childFolderCount: number;
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

export function getFolderContents(
  folderPath: string,
  files: LibraryFileRow[],
): { subfolders: FolderNode[]; files: LibraryFileRow[] } {
  const folderTree = buildFolderTree(files);
  const roots = [folderTree.mods, folderTree.tray];
  const activeNode = roots
    .flatMap((root) => [root, ...collectDescendants(root)])
    .find((node) => node.fullPath === folderPath);

  const directFiles = files.filter((file) => {
    const folderSegments = getFolderSegments(file.path);
    return folderSegments?.join("/") === folderPath;
  });

  return {
    subfolders: activeNode?.children ?? [],
    files: directFiles,
  };
}

function collectDescendants(node: FolderNode): FolderNode[] {
  return node.children.flatMap((child) => [child, ...collectDescendants(child)]);
}
