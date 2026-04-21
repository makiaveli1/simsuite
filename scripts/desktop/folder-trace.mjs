/**
 * Phase 5ah — Detailed Folder Navigation Diagnostic
 * Traces exactly what folderContents contains at each navigation step
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
    await page.waitForTimeout(4000); // wait for tree + content to fully load

    // Inject debug hook into the window to capture folderContents
    // We do this by reading DOM state and tree structure
    const initialState = await page.evaluate(() => {
      const layout = document.querySelector('.library-folders-layout');
      const contentPane = document.querySelector('.library-folder-content-pane');
      const treePane = document.querySelector('.library-folder-tree-pane');
      return {
        layoutExists: !!layout,
        contentPaneHeader: contentPane?.querySelector('.folder-content-title span')?.textContent ?? 'NOT FOUND',
        contentPaneText: contentPane?.innerText?.slice(0, 300) ?? 'NOT FOUND',
        treePaneText: treePane?.innerText?.slice(0, 300) ?? 'NOT FOUND',
        virtualRowCount: document.querySelectorAll('.virtualized-row').length,
        looseFilesBadge: document.querySelectorAll('.folder-loose-files-badge').length,
        folderLoadMore: document.querySelectorAll('.folder-load-more').length,
      };
    });
    console.log('=== Initial State (root level) ===');
    console.log(`Content pane header: "${initialState.contentPaneHeader}"`);
    console.log(`Content pane text: "${initialState.contentPaneText}"`);
    console.log(`Tree pane text: "${initialState.treePaneText}"`);
    console.log(`Virtual rows: ${initialState.virtualRowCount}`);
    console.log(`Loose-files badges: ${initialState.looseFilesBadge}`);
    console.log(`Folder "load more" buttons: ${initialState.folderLoadMore}`);

    // Now find and click the first subfolder in the CONTENT pane (not tree pane)
    // The content pane shows subfolders as .folder-row buttons
    const contentPaneRows = page.locator('.library-folder-content-pane .folder-row');
    const rowCount = await contentPaneRows.count();
    console.log(`\n=== Content pane has ${rowCount} folder rows ===`);

    if (rowCount > 0) {
      // Click the first content pane folder row
      const firstContentRow = contentPaneRows.first();
      const rowText = await firstContentRow.textContent();
      console.log(`Clicking content pane row: "${rowText?.trim()}"`);
      await firstContentRow.click();
      await page.waitForTimeout(3000);

      const afterClick = await page.evaluate(() => {
        const contentPane = document.querySelector('.library-folder-content-pane');
        return {
          header: contentPane?.querySelector('.folder-content-title span')?.textContent ?? 'NOT FOUND',
          text: contentPane?.innerText?.slice(0, 500) ?? 'NOT FOUND',
          virtualRows: document.querySelectorAll('.virtualized-row').length,
          tableRows: document.querySelectorAll('.library-folder-content-pane .library-list-row').length,
          looseBadges: document.querySelectorAll('.folder-loose-files-badge').length,
          loadMore: document.querySelectorAll('.folder-load-more').length,
        };
      });
      console.log(`\nAfter clicking "${rowText?.trim()}":`);
      console.log(`Content pane header: "${afterClick.header}"`);
      console.log(`Content pane text: "${afterClick.text}"`);
      console.log(`Virtual rows: ${afterClick.virtualRows}`);
      console.log(`Table rows: ${afterClick.tableRows}`);
      console.log(`Loose-files badges: ${afterClick.looseBadges}`);

      // If still no rows, try clicking deeper
      const subRowCount = await page.locator('.library-folder-content-pane .folder-row').count();
      if (subRowCount > 0) {
        const firstSubRow = page.locator('.library-folder-content-pane .folder-row').first();
        const subRowText = await firstSubRow.textContent();
        console.log(`\nClicking deeper: "${subRowText?.trim()}"`);
        await firstSubRow.click();
        await page.waitForTimeout(3000);

        const afterDeepClick = await page.evaluate(() => ({
          header: document.querySelector('.library-folder-content-pane .folder-content-title span')?.textContent ?? 'NOT FOUND',
          text: document.querySelector('.library-folder-content-pane')?.innerText?.slice(0, 500) ?? 'NOT FOUND',
          virtualRows: document.querySelectorAll('.virtualized-row').length,
          tableRows: document.querySelectorAll('.library-folder-content-pane .library-list-row').length,
          looseBadges: document.querySelectorAll('.folder-loose-files-badge').length,
        }));
        console.log(`After deep click "${subRowText?.trim()}":`);
        console.log(`Header: "${afterDeepClick.header}"`);
        console.log(`Text: "${afterDeepClick.text}"`);
        console.log(`Virtual rows: ${afterDeepClick.virtualRows}`);
        console.log(`Table rows: ${afterDeepClick.tableRows}`);
        console.log(`Loose-files badges: ${afterDeepClick.looseBadges}`);
      }

      // Try selection test on whatever rows are visible
      const anyRows = await page.locator('.virtualized-row, .library-folder-content-pane .library-list-row').count();
      if (anyRows > 0) {
        console.log('\n[SELECT] Attempting selection test...');
        const row = page.locator('.virtualized-row, .library-folder-content-pane .library-list-row').first();
        const box = await row.boundingBox();
        if (box) {
          await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
          await page.waitForTimeout(400);
          const boxAfter = await row.boundingBox();
          const shift = Math.abs((boxAfter?.y ?? box.y) - box.y);
          console.log(`[HOVER] Shift: ${shift.toFixed(1)}px ${shift > 5 ? '✗' : '✓'}`);

          await row.click();
          await page.waitForTimeout(3000);
          const bodyText = await page.evaluate(() => document.body.innerText?.trim() ?? '');
          const isBlank = !bodyText || bodyText.length < 100;
          console.log(`[SELECT] Blank: ${isBlank} ${isBlank ? '✗ FAIL' : '✓ PASS'}`);
          if (!isBlank) {
            const detail = await page.locator('#library-detail-sheet').isVisible().catch(() => false);
            const empty = await page.locator('.library-details-empty').isVisible().catch(() => false);
            console.log(`[SELECT] Detail: ${detail}, Empty: ${empty}`);
          }
        }
      } else {
        console.log('\n⚠ No clickable rows found for selection test');
      }
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
