/**
 * Phase 5ah — Robust folder-mode diagnostic
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  const errors = [];
  page.on('pageerror', err => errors.push(`PAGE ERROR: ${err.message}`));
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(`CONSOLE ERROR: ${msg.text()}`);
  });

  try {
    await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle', timeout: 30000 });
    console.log('✓ App loaded\n');

    // ── Basic sanity check ────────────────────────────────────────────────
    const headerExists = await page.locator('.folder-content-header').count() > 0;
    console.log(`FolderContentPane mounted: ${headerExists}`);

    // ── Navigate to folder mode ──────────────────────────────────────────
    const folderToggle = page.locator('[aria-label="Folders view"]');
    if (await folderToggle.count() > 0) {
      await folderToggle.click();
      await page.waitForTimeout(1500);
    }

    // ── Inspect folder tree ────────────────────────────────────────────────
    const treePane = page.locator('.folder-tree-pane, [class*="folder-tree"]').first();
    const treeText = await treePane.textContent().catch(() => '(not found)');
    console.log('\nTree pane text (first 300 chars):', treeText.trim().slice(0, 300));

    // ── Click Mods in content pane ───────────────────────────────────────
    const contentPane = page.locator('.library-folder-content-pane');
    const modRow = contentPane.locator('.folder-row').filter({ hasText: /^Mods$/ }).first();
    if (await modRow.count() > 0) {
      console.log('\nClicking "Mods" row in FolderContentPane...');
      await modRow.click();
      await page.waitForTimeout(1200);

      const summary = await contentPane.locator('.folder-content-summary').textContent().catch(() => '');
      const subfolders = await contentPane.locator('.folder-row').count();
      const looseBadge = await contentPane.locator('.folder-loose-files-badge').textContent().catch(() => 'none');
      const hint = await contentPane.locator('.folder-loose-files-hint').textContent().catch(() => 'none');
      const showAll = await contentPane.locator('.folder-load-more').textContent().catch(() => 'none');
      console.log(`Summary: "${summary}"`);
      console.log(`Subfolders listed: ${subfolders}`);
      console.log(`Loose badge: ${looseBadge}`);
      console.log(`Hint: "${hint}"`);
      console.log(`Show all button: "${showAll}"`);
    } else {
      console.log('\nNo "Mods" row found in content pane');
    }

    // ── Click a leaf subfolder ────────────────────────────────────────────
    const gameplayRow = contentPane.locator('.folder-row').filter({ hasText: /Gameplay/ }).first();
    if (await gameplayRow.count() > 0) {
      console.log('\nClicking "Gameplay" subfolder...');
      await gameplayRow.click();
      await page.waitForTimeout(1500);

      const header = await contentPane.locator('.folder-content-header').textContent().catch(() => '');
      const text = await contentPane.textContent().catch(() => '');
      const hasEmpty = text.includes('empty');
      const tableRows = await contentPane.locator('.library-list-row, tr').count();
      const virtualRows = await contentPane.locator('.virtualized-row').count();
      console.log(`Header: "${header?.trim()}"`);
      console.log(`Has empty state: ${hasEmpty}`);
      console.log(`Table rows: ${tableRows}, Virtual rows: ${virtualRows}`);
      console.log(`Content text (200 chars): "${text?.slice(0, 200)}"`);
    }

    // ── Check for errors ─────────────────────────────────────────────────
    if (errors.length > 0) {
      console.log(`\n⚠ ${errors.length} error(s):`);
      errors.forEach(e => console.log(`  ${e}`));
    } else {
      console.log('\n✓ No console/page errors');
    }

    console.log('\n=== DONE ===');
  } finally {
    await browser.close();
  }
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });