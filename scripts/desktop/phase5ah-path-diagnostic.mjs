/**
 * Phase 5ah — Path diagnostic: what does getFolderContents return at root level?
 */
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = '/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort';

// We need to test the actual Rust API response + the folderTree.ts logic
// Use tauri dev to get the raw data, then simulate in Node

// Try calling the API endpoint via a Playwright script that exposes the data
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(4000);

  // Inject debugging into the page context to capture state
  const result = await page.evaluate(() => {
    // Capture folderTree module state by hooking the window state
    // We can check what the folder contents useMemo sees
    const cp = document.querySelector('.library-folder-content-pane');
    return {
      contentPaneHTML: cp ? cp.innerHTML.slice(0, 1000) : 'NOT FOUND',
      folderContentHeader: cp?.querySelector('.folder-content-header .folder-content-summary')?.textContent,
      sectionCount: document.querySelectorAll('.library-folder-content-pane section').length,
      // Check what data the folderTree module has cached
      // Use window.__SIMSUITE_DEBUG if available
    };
  });

  console.log('Summary:', result.folderContentHeader);
  console.log('Sections:', result.sectionCount);

  // Now do a path simulation using the Rust API response
  // Call the Tauri command directly via fetch
  const tauriResult = await page.evaluate(async () => {
    try {
      // Try to call the list_library_files_for_tree via the API exposed in the app
      const resp = await fetch('http://127.0.0.1:1420/api/list-library-files', {
        headers: { 'Accept': 'application/json' }
      });
      return { ok: resp.ok, status: resp.status };
    } catch(e) {
      return { error: e.message };
    }
  });
  console.log('API probe:', JSON.stringify(tauriResult));

  // Instead, let's check the summary text vs what we know from the tree
  // "2 subfolders · 1 loose" means:
  // - subfolders = treeNodes for Mods/Tray = 2
  // - loose = summary loose count
  // The summary shows "1 loose" but no loose-files section appears

  // Check if rootFiles.some(f => getSourceRootFromPath(f.path) === "Mods") would be true
  // by reading the actual paths from the tree via the browser
  const treeInfo = await page.evaluate(() => {
    // The FolderContentPane should have rootFiles. We can't directly access React state,
    // but we can check the summary text which was computed from it.
    const summary = document.querySelector('.folder-content-summary')?.textContent;
    return { summary };
  });
  console.log('Tree info:', treeInfo);

  // Now simulate the path normalization locally
  const pathTest = await page.evaluate(() => {
    function getSourceRootFromPath(path) {
      const normalized = path.replace(/\\/g, "/").replace(/^[a-zA-Z]:/, "").replace(/^\/+/, "");
      const segments = normalized.split("/").filter(Boolean);
      const idx = segments.findIndex((s) => ["Mods", "Tray"].includes(s));
      return idx < 0 ? "" : segments[idx];
    }

    // Simulate paths we expect at depth-0
    const testPaths = [
      "C:\\\\Users\\\\likwi\\\\OneDrive\\\\ Sims 4\\\\Mods\\\\filename.package",
      "E:\\\\ Sims 4\\\\Mods\\\\Gameplay\\\\filename.package",
      "C:\\\\Users\\\\likwi\\\\AppData\\\\Roaming\\\\The Sims 4\\\\Mods\\\\filename.package",
    ];
    return testPaths.map(p => ({ path: p, src: getSourceRootFromPath(p) }));
  });
  console.log('Path normalization test:');
  pathTest.forEach(({ path, src }) => console.log(`  "${path}" → "${src}"`));

  await browser.close();
  console.log('\nDONE');
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });