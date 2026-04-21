/**
 * Phase 5ah — Folder Mode Verification Test
 * Tests: folder mode entry speed, loose-files hover stability, mod selection no blank
 * 
 * Run: node scripts/desktop/folder-mode-verify.mjs
 * (Requires dev server: pnpm dev)
 */

import { chromium } from 'playwright';

const DEV_URL = 'http://127.0.0.1:1420/#library';
const VIEWPORT = { width: 1440, height: 900 };

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: VIEWPORT });

  try {
    // ─── Setup ─────────────────────────────────────────────
    await page.goto(DEV_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });
    console.log('✓ App loaded');

    // ─── Step 1 — Enter folder mode ─────────────────────────
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    await foldersBtn.waitFor({ state: 'visible', timeout: 10000 });
    const t0 = Date.now();
    await foldersBtn.click();
    await page.waitForTimeout(500); // let animations settle
    const dt1 = Date.now() - t0;
    console.log(`✓ Entered folder mode in ${dt1}ms`);

    // ─── Step 2 — Verify folder tree loaded ────────────────
    const treePane = page.locator('.library-folder-tree-pane, .folder-tree-root');
    await treePane.waitFor({ state: 'visible', timeout: 10000 });
    const treeItems = await page.locator('.folder-tree-row, [data-folder]').count();
    console.log(`✓ Folder tree visible — ${treeItems} folder rows`);

    // ─── Step 3 — Navigate to a leaf subfolder (has actual files, not just more subfolders) ─────────
    // Click Mods root to expand it, then click a leaf node (like TMEX or Unsorted) that has files
    const modsRow = page.locator('.folder-tree-row').filter({ hasText: /^Mods$/ }).first();
    await modsRow.waitFor({ state: 'visible', timeout: 8000 });
    await modsRow.click();
    await page.waitForTimeout(500); // expand Mods tree

    // Get all visible subfolder rows to find one with actual files
    const subfolderRows = page.locator('.folder-tree-children .folder-tree-row');
    const subfolderRowCount = await subfolderRows.count();
    console.log(`  Found ${subfolderRowCount} expanded subfolder rows`);

    // Pick a likely leaf node — Unsorted often has loose files
    let targetRow = page.locator('.folder-tree-row').filter({ hasText: 'Unsorted' }).first();
    if (await targetRow.count() === 0) {
      targetRow = subfolderRows.first(); // fallback to first expanded subfolder
    }

    const targetName = await targetRow.textContent().catch(() => 'unknown');
    await targetRow.click();
    await page.waitForTimeout(2000); // allow folderContents to recompute + render
    console.log(`✓ Navigated into "${targetName?.trim()}"`);

    // Content pane should now show files (not just subfolders)
    const contentPane = page.locator('.library-folder-content-pane, .folder-content-pane');
    const contentPaneVisible = await contentPane.isVisible().catch(() => false);
    console.log(`✓ Content pane visible: ${contentPaneVisible}`);

    // Debug: snapshot of content pane
    const contentPaneHtml = await page.evaluate(() => {
      const pane = document.querySelector('.library-folder-content-pane, .folder-content-pane');
      if (!pane) return 'NOT FOUND';
      return JSON.stringify({
        innerText: pane.innerText?.slice(0, 800),
        childCount: pane.children.length,
        rowCount: pane.querySelectorAll('.library-list-row').length,
        trCount: pane.querySelectorAll('tr').length,
        virtualRowCount: pane.querySelectorAll('.virtualized-row').length,
        classList: [...pane.classList],
      });
    });
    console.log(`  Content pane snapshot: ${contentPaneHtml}`);

    // ─── Step 4 — Loose-files hover stability ──────────────
    await page.waitForTimeout(500); // let virtualizer settle
    // In folder mode, content can be table rows (.library-list-row) or virtualized
    const looseFileRows = page.locator('.library-list-row, .virtualized-row');
    const rowCount = await looseFileRows.count();
    console.log(`✓ Found ${rowCount} file rows in content area`);

    if (rowCount > 0) {
      // Hover over first 3 visible rows and check for crash/jumble
      const visibleRows = await looseFileRows.all();
      let hoverOk = true;
      for (let i = 0; i < Math.min(3, visibleRows.length); i++) {
        const box = await visibleRows[i].boundingBox();
        if (box && box.height > 0) {
          await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
          await page.waitForTimeout(200);
          // After hover, the row should still be visible and positionally stable
          const boxAfter = await visibleRows[i].boundingBox();
          if (!boxAfter || Math.abs((boxAfter?.y ?? 0) - box.y) > 4) {
            console.log(`✗ Row ${i} position shifted on hover: before=${box.y}, after=${boxAfter?.y}`);
            hoverOk = false;
          }
        }
      }
      if (hoverOk) console.log('✓ Loose-files hover stable — no jumble detected');
    }

    // ─── Step 5 — Click a mod in folder content → sidebar should update ─────
    // In folder mode, rows live in .library-list-row (table) not .virtualized-row
    await page.waitForTimeout(500);
    let selectableRows = page.locator('.library-folder-content-pane .library-list-row, .folder-content-pane .library-list-row');
    let count = await selectableRows.count();
    if (count === 0) {
      // Try any .library-list-row in the page (expanded folder showing content)
      selectableRows = page.locator('.library-list-row');
      count = await selectableRows.count();
    }
    if (count === 0) {
      // Try virtualized rows
      selectableRows = page.locator('.virtualized-row');
      count = await selectableRows.count();
    }
    console.log(`✓ Found ${count} selectable rows in folder content`);
    
    if (count > 0) {
      await selectableRows.first().waitFor({ state: 'visible', timeout: 5000 });
      const t3 = Date.now();
      await selectableRows.first().click();
      await page.waitForTimeout(2000); // wait for getFileDetail async + re-render
      const dt3 = Date.now() - t3;

      // Screen should NOT be blank
      const bodyText = await page.evaluate(() => document.body.textContent?.trim() ?? '');
      if (!bodyText || bodyText.length < 50) {
        console.log(`✗ BLANK SCREEN after mod selection (${dt3}ms)`);
      } else {
        // Sidebar should have detail content
        const detailPanel = page.locator('.library-details-panel, .library-detail-sheet, #library-detail-sheet');
        const detailVisible = await detailPanel.isVisible().catch(() => false);
        // Also check for empty state — the error handler should set selected=null on failure,
        // which shows the empty state, not a blank screen
        const emptyState = page.locator('.library-details-empty, .detail-empty');
        const emptyVisible = await emptyState.isVisible().catch(() => false);
        console.log(`✓ Mod selected in ${dt3}ms — sidebar visible: ${detailVisible}, empty state: ${emptyVisible}`);
        if (!detailVisible && !emptyVisible) {
          // Neither detail nor empty — this is a real problem
          console.log('⚠ Sidebar not showing detail or empty state — investigating...');
          const bodySnippet = bodyText.slice(0, 150).replace(/\s+/g, ' ');
          console.log(`  Body text preview: ${bodySnippet}`);
        }
      }
    } else {
      console.log('⚠ No selectable rows found — folder may need expansion or has no loose files');
    }

    // ─── Step 6 — Regression: verify list and grid still work ─
    const listBtn = page.locator('[aria-label="List view"]');
    await listBtn.waitFor({ state: 'visible', timeout: 5000 });
    await listBtn.click();
    await page.waitForTimeout(500);
    const tableRows = await page.locator('.library-table tbody tr').count();
    console.log(`✓ List mode: ${tableRows} table rows`);

    const gridBtn = page.locator('[aria-label="Grid view"]');
    await gridBtn.waitFor({ state: 'visible', timeout: 5000 });
    await gridBtn.click();
    await page.waitForTimeout(500);
    const gridCards = await page.locator('.library-card, .card').count();
    console.log(`✓ Grid mode: ${gridCards} cards`);

    console.log('\n=== Phase 5ah — ALL CHECKS COMPLETE ===');
  } catch (err) {
    console.error('✗ Test failed:', err.message);
    // Try to get page state even on failure
    const url = page.url();
    const bodyText = await page.evaluate(() => document.body?.textContent?.trim() ?? 'N/A').catch(() => 'N/A');
    console.error('URL:', url);
    console.error('Body preview:', bodyText.slice(0, 200));
    process.exit(1);
  } finally {
    await browser.close();
  }
}

run();