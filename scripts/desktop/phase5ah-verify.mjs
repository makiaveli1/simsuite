/**
 * Phase 5ah — Folder mode verification
 * Navigate to #/library for the actual library screen
 */
import { chromium } from 'playwright';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', err => errors.push(`PAGE: ${err.message}`));
  page.on('console', msg => { if (msg.type() === 'error') errors.push(`CONSOLE: ${msg.text()}`); });

  // Navigate to library screen directly
  await page.goto('http://127.0.0.1:1420/#/library', { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  const bodyText = await page.evaluate(() => document.body.innerText);
  console.log('Initial body (200 chars):', bodyText.slice(0, 200));

  // Verify we're on the library
  const viewToggleVisible = await page.locator('.library-view-toggle').count() > 0;
  console.log('Library view toggle visible:', viewToggleVisible);

  if (!viewToggleVisible) {
    console.log('✗ Not on library screen — check URL or routing');
    const ariaLabels = await page.evaluate(() => Array.from(document.querySelectorAll('[aria-label]')).map(e => e.getAttribute('aria-label')));
    console.log('Aria labels:', ariaLabels.slice(0, 10));
    await browser.close();
    process.exit(1);
  }

  // Enter folder mode
  const t0 = Date.now();
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(2500);
  const folderEntryMs = Date.now() - t0;

  // Check content pane
  const contentPane = page.locator('.library-folder-content-pane');
  const contentText = await contentPane.textContent().catch(() => '');
  const summary = await contentPane.locator('.folder-content-summary').textContent().catch(() => '');
  const subfolders = await contentPane.locator('.folder-row').count();
  console.log(`\n[1] Folder mode entry: ${folderEntryMs}ms`);
  console.log(`[1] Summary: "${summary}"`);
  console.log(`[1] Subfolders visible: ${subfolders}`);
  console.log(`[1] Content text: "${contentText.slice(0, 200)}"`);
  console.log(`[1] ${folderEntryMs < 3000 ? '✓' : '⚠'} Entry speed OK`);

  // Navigate Mods → Gameplay
  const t1 = Date.now();
  const modBtn = page.locator('.folder-row').filter({ hasText: /^Mods$/ }).first();
  if (await modBtn.count() > 0) {
    await modBtn.click();
    await page.waitForTimeout(800);
    const subRows = await page.locator('.folder-row').count();
    console.log(`\n[1b] Mods → ${subRows} subfolders`);
    const t2 = Date.now();
    const gameplay = page.locator('.folder-row').filter({ hasText: /Gameplay/ }).first();
    if (await gameplay.count() > 0) {
      await gameplay.click();
      await page.waitForTimeout(800);
      const header = await page.locator('.folder-content-header span').textContent().catch(() => '');
      console.log(`[1b] Deep nav → "${header}" (${Date.now()-t2}ms)`);
    }
    console.log(`[1b] Nav total: ${Date.now()-t1}ms`);
  } else {
    console.log('\n[1b] ⚠ No "Mods" folder-row');
  }

  // Click a row to test blank screen
  console.log('\n[2] Blank screen test...');
  await page.locator('.folder-content-header').first().click().catch(() => {});
  await page.waitForTimeout(800);
  await page.locator('.folder-row').filter({ hasText: /Gameplay|Repository|Options/ }).first().click().catch(() => {});
  await page.waitForTimeout(1500);
  const tableRows = await page.locator('.library-folder-content-pane .library-list-row').count();
  console.log(`[2] Table rows visible: ${tableRows}`);

  if (tableRows === 0) {
    // Navigate to Gameplay which has 156 rows
    await page.locator('.folder-row').filter({ hasText: /Gameplay/ }).first().click().catch(() => {});
    await page.waitForTimeout(1500);
    const newRows = await page.locator('.library-folder-content-pane .library-list-row').count();
    console.log(`[2] After navigation: ${newRows} table rows`);
  }

  const finalRowCount = await page.locator('.library-folder-content-pane .library-list-row').count();
  if (finalRowCount > 0) {
    const firstRow = page.locator('.library-folder-content-pane .library-list-row').first();
    const box = await firstRow.boundingBox().catch(() => null);
    await firstRow.click();
    await page.waitForTimeout(3000);
    const bodyLen = await page.evaluate(() => document.body.innerText.trim().length);
    const detailVisible = await page.locator('.library-inspector-panel, #library-detail-sheet').isVisible().catch(() => false);
    console.log(`[2] Body: ${bodyLen} chars, Detail: ${detailVisible}`);
    console.log(`[2] ${bodyLen < 150 && !detailVisible ? '✗ BLANK SCREEN DETECTED' : '✓ No blank screen'}`);
  } else {
    console.log('[2] ⚠ No rows to click in folder content');
  }

  // Hover stability
  console.log('\n[3] Hover stability...');
  await page.locator('.folder-content-header').first().click().catch(() => {});
  await page.waitForTimeout(500);
  await page.locator('.folder-row').filter({ hasText: /Mods/ }).first().click().catch(() => {});
  await page.waitForTimeout(500);

  const showAll = page.locator('.folder-load-more');
  if (await showAll.count() > 0) {
    await showAll.click();
    await page.waitForTimeout(1500);
  }

  const virtualRows = page.locator('.virtualized-row');
  const vCount = await virtualRows.count();
  if (vCount > 0) {
    let jitter = false;
    for (let i = 0; i < Math.min(4, vCount); i++) {
      const box = await virtualRows.nth(i).boundingBox().catch(() => null);
      if (!box || box.height === 0) continue;
      const y1 = box.y;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForTimeout(600);
      const box2 = await virtualRows.nth(i).boundingBox().catch(() => null);
      if (Math.abs((box2?.y ?? y1) - y1) > 4) jitter = true;
    }
    console.log(`[3] Virtualized hover: ${jitter ? '⚠ JITTER' : '✓ Stable'}`);
  } else {
    const tRows = await page.locator('.library-folder-content-pane .library-list-row').count();
    if (tRows > 0) {
      const box = await page.locator('.library-folder-content-pane .library-list-row').first().boundingBox().catch(() => null);
      if (box) {
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.waitForTimeout(500);
        const box2 = await page.locator('.library-folder-content-pane .library-list-row').first().boundingBox().catch(() => null);
        const shift = Math.abs((box2?.y ?? box.y) - box.y);
        console.log(`[3] Table row hover: ${shift.toFixed(1)}px ${shift < 5 ? '✓' : '⚠'}`);
      }
    } else {
      const badge = await page.evaluate(() => document.querySelector('.folder-loose-files-badge')?.textContent ?? 'none');
      console.log(`[3] ⚠ No hover target rows. Loose badge: "${badge}"`);
    }
  }

  // Regressions
  console.log('\n[REGRESS] Grid...');
  await page.locator('[aria-label="Grid view"]').click().catch(() => {});
  await page.waitForTimeout(1500);
  const cards = await page.locator('[class*="library-card"]').count();
  console.log(`[REGRESS] Grid: ${cards} cards ${cards > 0 ? '✓' : '⚠'}`);
  await page.locator('[aria-label="List view"]').click().catch(() => {});
  await page.waitForTimeout(1500);
  const listRows = await page.locator('.library-table tbody tr').count();
  console.log(`[REGRESS] List: ${listRows} rows ${listRows > 0 ? '✓' : '⚠'}`);

  if (errors.length > 0) {
    console.log(`\n⚠ ${errors.length} error(s):`);
    errors.slice(0, 5).forEach(e => console.log(`  ${e}`));
  } else {
    console.log('\n✓ No errors');
  }

  console.log('\n=== VERIFICATION COMPLETE ===');
  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });