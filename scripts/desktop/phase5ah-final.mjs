/**
 * Phase 5ah — Final Verification Test
 * Tests all three issues in the running app
 */

import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';
const VIEWPORT = { width: 1440, height: 900 };

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: VIEWPORT });

  const errors = [];
  page.on('pageerror', err => errors.push(`PAGE ERROR: ${err.message}`));
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(`CONSOLE ERROR: ${msg.text()}`);
  });

  try {
    await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });
    console.log('✓ App loaded\n');

    // ─── ISSUE 1: Folder mode speed ──────────────────────────────
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    await foldersBtn.waitFor({ state: 'visible', timeout: 10000 });
    const t0 = Date.now();
    await foldersBtn.click();
    await page.waitForTimeout(500); // let first render complete
    const folderEntryMs = Date.now() - t0;

    const layoutVisible = await page.locator('.library-folders-layout').isVisible().catch(() => false);
    const treePaneVisible = await page.locator('.library-folder-tree-pane').isVisible().catch(() => false);
    const contentPaneVisible = await page.locator('.library-folder-content-pane').isVisible().catch(() => false);
    console.log(`[1] Folder mode entry: ${folderEntryMs}ms (target: <500ms)`);
    console.log(`    Layout: ${layoutVisible}, Tree: ${treePaneVisible}, Content: ${contentPaneVisible}`);
    console.log(`    ${folderEntryMs < 500 ? '✓ PASS' : '✗ SLOW'} — Folder mode felt speed`);

    // ─── ISSUE 2: Loose-files hover stability ────────────────────
    // At root level, check if loose-files section renders
    await page.waitForTimeout(2000); // let content hydrate

    const summaryText = await page.locator('.folder-content-summary').textContent().catch(() => '');
    console.log(`\n[2] Content pane summary: "${summaryText}"`);

    // Check for loose-files section (either virtualized rows or table rows)
    await page.waitForTimeout(500);
    const virtualRows = page.locator('.virtualized-row');
    const virtualRowCount = await virtualRows.count();
    const tableRows = page.locator('.library-folder-content-pane .library-list-row');
    const tableRowCount = await tableRows.count();
    console.log(`[2] Loose-files rows: virtual=${virtualRowCount}, table=${tableRowCount}`);

    // Also check for "Show all" button which expands the section
    const showAllBtn = page.locator('.folder-load-more');
    const showAllCount = await showAllBtn.count();
    console.log(`[2] Expand buttons: ${showAllCount}`);

    let hoverStable = null;
    const totalRows = virtualRowCount + tableRowCount;
    if (totalRows > 0) {
      const rows = virtualRowCount > 0 ? await virtualRows.all() : await tableRows.all();
      hoverStable = true;
      for (let i = 0; i < Math.min(5, rows.length); i++) {
        const box = await rows[i].boundingBox();
        if (!box || box.height === 0) continue;
        const yBefore = box.y;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.waitForTimeout(400);
        const boxAfter = await rows[i].boundingBox();
        const shift = Math.abs((boxAfter?.y ?? yBefore) - yBefore);
        if (shift > 5) {
          console.log(`[2]   Row ${i}: shift=${shift.toFixed(1)}px ✗ HOVER JUMBLE`);
          hoverStable = false;
        }
      }
      if (hoverStable) console.log(`[2] ✓ PASS — No hover jumble detected`);
    } else {
      console.log(`[2] ⚠ Could not test hover — no rows (content: "${summaryText}")`);
    }

    // ─── ISSUE 3: Selection blank screen ───────────────────────────
    console.log('\n[3] Testing mod selection (blank screen check)...');

    // Find any clickable rows (could be virtualized or table)
    let rowsToClick = virtualRowCount > 0
      ? await virtualRows.all()
      : await tableRows.all();

    // If no rows at root, try expanding "Show all" or navigating to a folder
    if (rowsToClick.length === 0 && showAllCount > 0) {
      console.log('[3] No rows at root — clicking "Show all" to expand...');
      await showAllBtn.first().click();
      await page.waitForTimeout(2000);
      rowsToClick = [...(await virtualRows.all()), ...(await tableRows.all())];
    }

    // If still no rows, try navigating to Mods folder and finding rows there
    if (rowsToClick.length === 0) {
      console.log('[3] No rows — navigating to Mods folder...');
      const modsRow = page.locator('.library-folder-content-pane .folder-row').filter({ hasText: /Mods/ }).first();
      if (await modsRow.count() > 0) {
        await modsRow.click();
        await page.waitForTimeout(2000);
        rowsToClick = [...(await virtualRows.all()), ...(await tableRows.all())];
        if (rowsToClick.length === 0) {
          // Try deeper
          const subRow = page.locator('.library-folder-content-pane .folder-row').first();
          if (await subRow.count() > 0) {
            await subRow.click();
            await page.waitForTimeout(2000);
            rowsToClick = [...(await virtualRows.all()), ...(await tableRows.all())];
          }
        }
      }
    }

    if (rowsToClick.length > 0) {
      const firstRow = rowsToClick[0];
      const tSelect = Date.now();
      await firstRow.click();
      await page.waitForTimeout(3000); // wait for getFileDetail
      const selectMs = Date.now() - tSelect;

      const bodyText = await page.evaluate(() => document.body.innerText?.trim() ?? '');
      const isBlank = !bodyText || bodyText.length < 100;
      console.log(`[3] Selection took ${selectMs}ms, body length: ${bodyText.length}`);

      if (isBlank) {
        console.log('[3] ✗ FAIL — Screen went blank after selection');
      } else {
        const detailSheet = await page.locator('#library-detail-sheet').isVisible().catch(() => false);
        const emptyState = await page.locator('.library-details-empty, .detail-empty').isVisible().catch(() => false);
        console.log(`[3] Detail sheet: ${detailSheet}, Empty: ${emptyState}`);
        if (!detailSheet && !emptyState) {
          console.log('[3] ⚠ Neither detail nor empty — possible partial render');
        } else {
          console.log('[3] ✓ PASS — Selection works without blank screen');
        }
      }
    } else {
      console.log('[3] ⚠ Could not test selection — no rows found in any location');
      // At minimum, verify the app didn't crash
      const bodyText = await page.evaluate(() => document.body.innerText?.trim() ?? '');
      const isBlank = !bodyText || bodyText.length < 50;
      console.log(`[3] App still running: ${!isBlank ? '✓' : '✗'}`);
    }

    // ─── Regression checks ─────────────────────────────────────────
    console.log('\n[REGRESS] List mode...');
    await page.locator('[aria-label="List view"]').click();
    await page.waitForTimeout(1500);
    const listRows = await page.locator('.library-table tbody tr').count();
    console.log(`[REGRESS] List mode: ${listRows} rows ${listRows > 0 ? '✓' : '⚠ (may be empty in dev data)'}`);

    console.log('[REGRESS] Grid mode...');
    await page.locator('[aria-label="Grid view"]').click();
    await page.waitForTimeout(1500);
    const cards = await page.locator('.library-card, [class*="library-card"]').count();
    console.log(`[REGRESS] Grid mode: ${cards} cards ${cards > 0 ? '✓' : '✗'}`);

    // ─── Error report ─────────────────────────────────────────────
    if (errors.length > 0) {
      console.log(`\n⚠ ${errors.length} error(s) during test:`);
      errors.forEach(e => console.log(`  ${e}`));
    } else {
      console.log('\n✓ No console/page errors during test');
    }

    console.log('\n=== VERIFICATION COMPLETE ===');
  } catch (err) {
    console.error('✗ Test failed:', err.message);
    process.exit(1);
  } finally {
    await browser.close();
  }
}

run();
