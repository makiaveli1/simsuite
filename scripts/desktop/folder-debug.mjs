/**
 * Phase 5ah — Live State Diagnostic
 * Reads actual React/Redux state from the running app to diagnose folder contents
 */

import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });
    console.log('✓ App loaded\n');

    // Enter folder mode
    await page.locator('[aria-label="Folders view"]').click();
    await page.waitForTimeout(3000); // let everything fully load

    // Read React state (fiber tree)
    const state = await page.evaluate(() => {
      // Try to find the LibraryScreen component via React DevTools
      const root = document.querySelector('#root');
      if (!root) return { error: 'No #root' };

      // Walk the React fiber tree to find relevant state
      const fiber = Object.keys(root).find(k => k.startsWith('__reactFiber$') || k.startsWith('__reactContainer$'));
      if (!fiber) return { error: 'No React fiber found' };

      // For simplicity, just return DOM-based state
      const folderPane = document.querySelector('.library-folder-content-pane');
      const folderText = folderPane?.innerText?.slice(0, 800) ?? 'NOT FOUND';
      const virtualRows = document.querySelectorAll('.virtualized-row').length;
      const tableRows = document.querySelectorAll('.library-list-row').length;
      const showAllBtns = document.querySelectorAll('.folder-load-more').length;

      return {
        folderContentText: folderText,
        virtualRowCount: virtualRows,
        tableRowCount: tableRows,
        showAllButtonCount: showAllBtns,
      };
    });

    console.log('=== Folder Mode State ===');
    console.log(`Virtualized rows: ${state.virtualRowCount}`);
    console.log(`Table rows: ${state.tableRowCount}`);
    console.log(`Show-all buttons: ${state.showAllButtonCount}`);
    console.log(`\nContent pane text:\n${state.folderContentText}`);

    // Try to find and expand loose files section
    const showAllBtn = page.locator('.folder-load-more').first();
    if (await showAllBtn.count() > 0) {
      console.log('\n[ACTION] Clicking "Show all" to expand...');
      await showAllBtn.click();
      await page.waitForTimeout(2000);

      const afterExpand = await page.evaluate(() => ({
        virtualRows: document.querySelectorAll('.virtualized-row').length,
        tableRows: document.querySelectorAll('.library-list-row').length,
        folderText: document.querySelector('.library-folder-content-pane')?.innerText?.slice(0, 400) ?? 'NOT FOUND',
      }));
      console.log(`\nAfter expand — Virtual rows: ${afterExpand.virtualRows}, Table rows: ${afterExpand.tableRows}`);
      console.log(`Content:\n${afterExpand.folderText}`);

      // Try hovering first virtual row
      if (afterExpand.virtualRows > 0) {
        const firstRow = page.locator('.virtualized-row').first();
        const box = await firstRow.boundingBox();
        if (box) {
          const yBefore = box.y;
          await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
          await page.waitForTimeout(400);
          const boxAfter = await firstRow.boundingBox();
          const shift = Math.abs((boxAfter?.y ?? yBefore) - yBefore);
          console.log(`\n[HOVER] Row shift: ${shift.toFixed(1)}px ${shift > 5 ? '✗ JUMBLE' : '✓'}`);

          // Try clicking
          console.log('\n[SELECT] Clicking first row...');
          const t0 = Date.now();
          await firstRow.click();
          await page.waitForTimeout(3000);
          const dt = Date.now() - t0;

          const bodyText = await page.evaluate(() => document.body.innerText?.trim() ?? '');
          const isBlank = !bodyText || bodyText.length < 100;
          console.log(`[SELECT] Time: ${dt}ms, Blank: ${isBlank}`);
          if (isBlank) {
            console.log('✗ FAIL — Screen blank after selection');
          } else {
            const detailSheet = await page.locator('#library-detail-sheet').isVisible().catch(() => false);
            const empty = await page.locator('.library-details-empty, .detail-empty').isVisible().catch(() => false);
            console.log(`[SELECT] Detail: ${detailSheet}, Empty: ${empty}`);
            if (!detailSheet && !empty) {
              console.log('⚠ Neither detail nor empty state');
            } else {
              console.log('✓ PASS — Selection works');
            }
          }
        }
      }
    } else {
      console.log('\n⚠ No "Show all" button — content pane has no paginated content');
    }

    console.log('\n=== DIAGNOSTIC COMPLETE ===');
  } catch (err) {
    console.error('Error:', err.message);
    process.exit(1);
  } finally {
    await browser.close();
  }
}

run();
