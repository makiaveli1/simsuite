/**
 * Phase 5ah — Deep inject: add console logs to folderTree.ts getFolderContents
 * and verify output at each navigation step
 */
import { readFileSync, writeFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = '/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort';
const folderTreePath = `${projectRoot}/src/screens/library/folderTree.ts`;

async function run() {
  // Read the original file
  let original = readFileSync(folderTreePath, 'utf8');

  // Inject debugging BEFORE making changes — save original backup
  const backupPath = `${projectRoot}/src/screens/library/folderTree.ts.backup5ah`;
  writeFileSync(backupPath, original);

  console.log('Original file backed up to', backupPath);
  console.log('Original line count:', original.split('\n').length);

  // Find the getFolderContents function
  const lines = original.split('\n');
  const getFolderContentsLine = lines.findIndex(l => l.includes('export function getFolderContents'));
  console.log('getFolderContents found at line:', getFolderContentsLine + 1);

  // Inject logging before the function
  const injectBefore = `
// ── DEBUG: Phase 5ah ──
const _debugLog: string[] = [];
function _dl(msg: string) { _debugLog.push(\`[\${Date.now()}] \${msg}\`); console.log('[folderTree DEBUG]', msg); }
// ── END DEBUG ──
`;

  // Inject at the start of getFolderContents
  const modifiedLines = [...lines];
  const insertIdx = getFolderContentsLine;
  modifiedLines.splice(insertIdx, 0, injectBefore);

  // Find the "return {" block inside getFolderContents and inject logs before it
  // We need to find the specific return statements
  let inFunction = false;
  let braceCount = 0;
  let returnLine = -1;
  for (let i = getFolderContentsLine; i < modifiedLines.length; i++) {
    const l = modifiedLines[i];
    if (l.includes('export function getFolderContents')) inFunction = true;
    if (!inFunction) continue;
    if (l.includes('{')) braceCount += (l.match(/{/g) || []).length;
    if (l.includes('}')) braceCount -= (l.match(/}/g) || []).length;
    // Look for "return { subfolders" or similar
    if (braceCount > 0 && l.includes('return {')) {
      returnLine = i;
      break;
    }
  }

  console.log('First return { found at line:', returnLine + 1);

  // Build the debug log lines to inject before each return statement
  const debugReturns = [
    `  _dl(\`getFolderContents(\${folderPath ?? 'null'}, files=\${files.length}): computing…\`);`,
    `  _dl(\`  tree nodes: mods totalFileCount=\${folderTree.mods.totalFileCount}, tray totalFileCount=\${folderTree.tray.totalFileCount}\`);`,
    `  _dl(\`  rootFiles computed: \${rootFiles?.length ?? 0}, subfolders: \${subfolders?.length ?? 0}, files: \${files?.length ?? 0}\`);`,
  ];

  if (returnLine > 0) {
    modifiedLines.splice(returnLine, 0, debugReturns.join('\n'));
  }

  const modified = modifiedLines.join('\n');
  console.log('Modified line count:', modified.split('\n').length);
  console.log('Lines added:', modified.split('\n').length - lines.length);

  // Write modified file
  writeFileSync(folderTreePath, modified);
  console.log('Modified folderTree.ts written');

  // Now wait for Vite HMR to pick up the change
  console.log('\nWaiting for Vite to recompile (5s)...');
  await new Promise(r => setTimeout(r, 5000));

  // Verify the change was picked up
  const checkResponse = await fetch('http://127.0.0.1:1420/src/screens/library/folderTree.ts');
  const newContent = await checkResponse.text();
  const hasDebug = newContent.includes('_dl(');
  console.log('Vite serving modified file:', hasDebug);

  if (!hasDebug) {
    console.log('ERROR: Vite not serving modified file. Reverting...');
    writeFileSync(folderTreePath, original);
    console.log('Reverted.');
    return;
  }

  console.log('Modified file is being served. Ready for testing.');
  console.log('Now run: node scripts/desktop/phase5ah-diag.mjs');
  console.log('\nRevert with: cp src/screens/library/folderTree.ts.backup5ah src/screens/library/folderTree.ts');
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });