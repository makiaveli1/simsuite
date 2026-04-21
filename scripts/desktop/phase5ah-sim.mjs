/**
 * Phase 5ah — Root cause analysis: simulate getFolderContents at root level
 * using mock API data to find the loose-files section bug
 */
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = '/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort';

// Read and parse the api.ts to understand what paths the mock files have
const apiContent = readFileSync(`${projectRoot}/src/lib/api.ts`, 'utf8');
const lines = apiContent.split('\n');

// Find the mockFiles array and extract some sourceLocation + filename combos
const mockLines = [];
let inMockFiles = false;
for (let i = 0; i < lines.length; i++) {
  if (lines[i].includes('mockFiles[') || lines[i].includes('mockFiles:')) {
    inMockFiles = true;
  }
  if (inMockFiles) {
    mockLines.push(`${i+1}: ${lines[i]}`);
    if (lines[i].trim() === '];' && inMockFiles) break;
  }
  if (mockLines.length > 400) break;
}

// Extract some sourceLocation + suggestedRelativePath or filename combos
console.log('=== Mock File Path Samples ===');
for (const l of mockLines.slice(0, 300)) {
  if (l.includes('sourceLocation') || l.includes('suggestedRelativePath') || l.includes('filename') || l.includes('relativePath') || l.includes('finalRelative')) {
    console.log(l);
  }
}

// Now simulate the path normalization
console.log('\n=== Path Normalization Simulation ===');

function getRelativePath(path) {
  return path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
}

const ROOT_NAMES = new Set(["Mods", "Tray"]);

function findRootSegments(path) {
  const segments = getRelativePath(path).split("/").filter(Boolean);
  const rootIndex = segments.findIndex((segment) => ROOT_NAMES.has(segment));
  if (rootIndex < 0) return null;
  return segments.slice(rootIndex);
}

function getFolderSegments(path) {
  const rootedSegments = findRootSegments(path);
  if (!rootedSegments || rootedSegments.length === 0) return null;
  return rootedSegments.slice(0, -1);
}

function getSourceRootFromPath(path) {
  const normalized = path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
  const segments = normalized.split("/").filter(Boolean);
  const idx = segments.findIndex((s) => ROOT_NAMES.has(s));
  return idx < 0 ? "" : segments[idx];
}

const testPaths = [
  // Depth-0 (direct in Mods, no subfolder)
  "Mods/mymod.package",
  "Mods/somefile.package",
  "Tray/mytray.item",
  // Depth-1 (in Gameplay subfolder)
  "Mods/Gameplay/Utility/mymod.package",
  "Mods/Gameplay/Review/mymod.package",
  // Depth-2
  "Mods/Gameplay/Utility/adeepindigo/mymod.package",
  // From the actual API data (sample suggestedRelativePaths)
  "Mods\\Gameplay\\Review\\UnknownCreator_misc.package",
  "Mods\\BuildBuy\\AHarris00\\AHarris00_CozyKitchen.package",
  "Mods\\CAS\\Miiko\\Miiko_Eyebrows.package",
];

console.log('Path                              | getFolderSegments | isRootSegment | depth');
console.log('--------------------------------|-------------------|---------------|------');
for (const p of testPaths) {
  const fs = getFolderSegments(p);
  const src = getSourceRootFromPath(p);
  const depth = fs !== null ? fs.length : -1;
  console.log(`${p.padEnd(40)} | ${JSON.stringify(fs).padEnd(18)} | ${src.padEnd(11)} | ${depth}`);
}

console.log('\n=== If mockFiles had depth-0 files (e.g. Mods/foo.package) ===');
console.log('A depth-0 Mods file: path = "Mods/foo.package"');
console.log('  getRelativePath → "Mods/foo.package"');
console.log('  findRootSegments → ["Mods", "foo.package"]');
console.log('  getFolderSegments → ["Mods"] (length=1) ✓ would be included in rootFiles');
console.log('\n=== The question: does the mock have any depth-0 files? ===');

// Check if mockFiles contains any path where the file is directly in Mods (not in a subfolder)
// A depth-0 file would have path like ".../Mods/filename.package" (no intermediate subfolder)
// But sourceLocation = "mods" is just the source root, the full path matters
// Let's check a real path from the data:
// suggestedRelativePath: "Mods\\Gameplay\\Review\\UnknownCreator_misc.package"
// After normalization: "Mods/Gameplay/Review/UnknownCreator_misc.package"
// This has segments ["Mods", "Gameplay", "Review", "UnknownCreator_misc.package"]
// getFolderSegments → ["Mods", "Gameplay", "Review"] (length=3)
// At Gameplay level, this would be counted as a subfolder file (not depth-0)

// Check if ANY mock file has a path that would be depth-0
let depth0Count = 0;
let depth1Count = 0;
let depth2PlusCount = 0;
let nullSegmentCount = 0;
let modsFiles = 0;
let trayFiles = 0;

console.log('\n=== Scanning mock file paths for depth analysis ===');
// We need to extract actual mock file data. Since we can't run the module,
// let's manually scan for the pattern.
// In the mock, each file entry has: suggestedRelativePath, sourceLocation, filename, etc.
for (const l of mockLines) {
  const m = l.match(/suggestedRelativePath:\s*"([^"]+)"/);
  if (m) {
    const relPath = m[1];
    const fs = getFolderSegments(relPath);
    if (fs === null) {
      nullSegmentCount++;
    } else if (fs.length === 1) {
      depth0Count++;
    } else if (fs.length === 2) {
      depth1Count++;
    } else {
      depth2PlusCount++;
    }
  }
  // Also track sourceLocation
  if (l.includes('sourceLocation:')) {
    if (l.includes('"mods"')) modsFiles++;
    if (l.includes('"tray"')) trayFiles++;
  }
}

console.log('Depth-0 files:', depth0Count, '(directly in Mods or Tray, no subfolder)');
console.log('Depth-1 files:', depth1Count, '(one level deep: Mods/Subfolder)');
console.log('Depth-2+ files:', depth2PlusCount);
console.log('Files with null segments:', nullSegmentCount);
console.log('Mods sourceLocation:', modsFiles);
console.log('Tray sourceLocation:', trayFiles);