/**
 * Phase 5ah — Targeted Folder Mode Test
 * Specifically tests:
 * 1. Root-level loose-files section visible at folder mode entry
 * 2. Hover stability on virtualized loose-file rows
 * 3. Clicking a loose file does NOT blank the screen
 * 4. Folder navigation to a subfolder with files
 */

import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';
const VIEWPORT = { width: 1440, height: 900 };

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: VIEWPORT });

  try {
    await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });
    console.log('✓ App loaded\n');

    // ─── STEP 1 — Enter folder mode and stay at ROOT ────────────────────────
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    await foldersBtn.waitFor({ state: 'visible', timeout: 10000 });
    const t0 = Date.now();
    await foldersBtn.click();

    // Wait for the folders layout to be fully rendered
    await page.locator('.library-folders-layout').waitFor({ state: 'visible', timeout: 8000 });
    const folderEntryMs = Date.now() - t0;
    console.log(`[1] Folder mode entry: ${folderEntryMs}ms`);

    // ─── STEP 2 — Check root-level content pane (folderPath = null) ────────
    // At root (folderPath=null), the content pane shows the loose-files section
    await page.waitForTimeout(2000); // let content pane hydrate

    const contentPane = page.locator('.library-folder-content-pane');
    await contentPane.waitFor({ state: 'visible', timeout: 8000 });

    // Check for loose-files section (ModsLooseFilesSection)
    const looseFilesSection = page.locator('.folder-loose-source-group, .folder-loose-files-badge');
    const looseFilesCount = await looseFilesSection.count();
    console.log(`[2] Loose-files elements found: ${looseFilesCount}`);

    // Check for "Show all" button (expands pagination)
    const showAllBtn = page.locator('.folder-load-more');
    const showAllCount = await showAllBtn.count();
    console.log(`[2] Show-all/more buttons: ${showAllCount}`);

    // Check for virtualized rows (should be in loose-files section)
    await page.waitForTimeout(500);
    const virtualRows = page.locator('.virtualized-row');
    const virtualRowCount = await virtualRows.count();
    console.log(`[2] Virtualized rows: ${virtualRowCount}`);

    // Get the full inner text of the content pane
    const paneText = await contentPane.innerText().catch(() => '');
    console.log(`[2] Content pane text (first 400 chars):\n${paneText.slice(0, 400)}`);

    // ─── STEP 3 — Expand loose files if there's a "Show all" button ────────
    if (showAllCount > 0) {
      console.log('[3] Clicking "Show all" to expand loose files...');
      await showAllBtn.first().click();
      await page.waitForTimeout(2000);

      const expandedVirtualRows = await page.locator('.virtualized-row').count();
      console.log(`[3] Virtualized rows after expand: ${expandedVirtualRows}`);

      // ─── STEP 4 — Hover stability test ────────────────────────────────
      if (expandedVirtualRows > 0) {
        console.log('[4] Testing hover stability...');
        const rows = await page.locator('.virtualized-row').all();
        let jumbleFound = false;

        for (let i = 0; i < Math.min(5, rows.length); i++) {
          const row = rows[i];
          const box = await row.boundingBox();
          if (!box || box.height === 0) continue;

          const yBefore = box.y;
          await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
          await page.waitForTimeout(400); // let framer settle

          const boxAfter = await row.boundingBox();
          const shift = Math.abs((boxAfter?.y ?? yBefore) - yBefore);
          console.log(`  Row ${i}: shift=${shift.toFixed(1)}px ${shift > 5 ? '✗ JUMBLE' : '✓'}`);

          if (shift > 5) jumbleFound = true;
        }

        if (!jumbleFound) {
          console.log('[4] ✓ PASS — No hover jumble detected');
        } else {
          console.log('[4] ✗ FAIL — Hover position shifting detected');
        }

        // ─── STEP 5 — Selection blank screen test ─────────────────────
        console.log('\n[5] Testing file selection (blank screen check)...');
        const firstRow = rows[0];
        const rowBox = await firstRow.boundingBox();

        if (rowBox) {
          const tSelect = Date.now();
          await firstRow.click();
          await page.waitForTimeout(3000); // wait for async detail fetch
          const selectMs = Date.now() - tSelect;

          const bodyText = await page.evaluate(() => document.body.innerText?.trim() ?? '');
          const isBlank = !bodyText || bodyText.length < 100;
          console.log(`[5] Selection took ${selectMs}ms, isBlank=${isBlank}`);

          if (isBlank) {
            console.log('[5] ✗ FAIL — Screen is blank after selection');
          } else {
            // Check sidebar state
            const detailSheet = page.locator('#library-detail-sheet');
            const detailVisible = await detailSheet.isVisible().catch(() => false);
            const emptyState = page.locator('.library-details-empty, .detail-empty');
            const emptyVisible = await emptyState.isVisible().catch(() => false);
            console.log(`[5] Detail sheet visible: ${detailVisible}, Empty state: ${emptyVisible}`);

            if (!detailVisible && !emptyVisible) {
              console.log('[5] ⚠ Neither detail nor empty — possible partial render');
              const bodySnippet = bodyText.slice(0, 200).replace(/\s+/g, ' ');
              console.log(`    Body preview: ${bodySnippet}`);
            } else {
              console.log('[5] ✓ PASS — Selection works without blank screen');
            }
          }
        }
      } else {
        console.log('[4] ⚠ No virtualized rows to test hover/selection');
      }
    } else {
      console.log('[3] ⚠ No "Show all" button — loose files section might not have items');
      console.log('[4] ⚠ Skipping hover test — no rows visible');
      console.log('[5] ⚠ Skipping selection test — no rows visible');
    }

    // ─── STEP 6 — Navigate to a subfolder ────────────────────────────────
    console.log('\n[6] Testing subfolder navigation...');
    // Find the first subfolder in the tree
    const subfoldersInTree = page.locator('.folder-tree-children .folder-tree-row');
    const subfolderCount = await subfoldersInTree.count();
    console.log(`[6] Subfolders visible in tree: ${subfolderCount}`);

    if (subfolderCount > 0) {
      // Click the first subfolder
      await subfoldersInTree.first().click();
      await page.waitForTimeout(2000);

      const subfolderText = await subfoldersInTree.first().textContent().catch(() => '');
      console.log(`[6] Clicked subfolder: ${subfolderText?.trim()}`);

      // Check content pane
      const paneText2 = await contentPane.innerText().catch(() => '');
      console.log(`[6] Content pane after nav (first 300 chars):\n${paneText2.slice(0, 300)}`);

      // Try to find file rows in the subfolder content
      const contentRows = page.locator('.library-folder-content-pane .library-list-row, .folder-loose-source-group .library-list-row');
      const contentRowCount = await contentRows.count();
      console.log(`[6] File rows in subfolder content: ${contentRowCount}`);

      if (contentRowCount > 0) {
        // Test selection in subfolder
        const tSelect = Date.now();
        await contentRows.first().click();
        await page.waitForTimeout(3000);
        const selectMs = Date.now() - tSelect;
        const bodyText3 = await page.evaluate(() => document.body.innerText?.trim() ?? '');
        const isBlank3 = !bodyText3 || bodyText3.length < 100;
        console.log(`[6] Subfolder selection: ${selectMs}ms, blank=${isBlank3}`);
        console.log(`[6] ${isBlank3 ? '✗ FAIL' : '✓ PASS'} — ${isBlank3 ? 'blank screen on subfolder select' : 'subfolder select works'}`);
      }
    }

    // ─── STEP 7 — Regression checks ──────────────────────────────────────
    console.log('\n[7] Regression: list mode...');
    await page.locator('[aria-label="List view"]').click();
    await page.waitForTimeout(1500);
    const tableRows = await page.locator('.library-table tbody tr').count();
    console.log(`[7] List mode rows: ${tableRows} ${tableRows === 0 ? '⚠' : '✓'}`);

    console.log('[7] Regression: grid mode...');
    await page.locator('[aria-label="Grid view"]').click();
    await page.waitForTimeout(1500);
    const cardCount = await page.locator('.library-card, [class*="library-card"]').count();
    console.log(`[7] Grid mode cards: ${cardCount} ${cardCount > 0 ? '✓' : '✗'}`);

    console.log('\n=== COMPLETE ===');
  } catch (err) {
    console.error('✗ Test error:', err.message);
    const bodyPreview = await page.evaluate(() => document.body?.textContent?.slice(0, 200)?.replace(/\s+/g, ' ')).catch(() => 'N/A');
    console.error('Body:', bodyPreview);
    process.exit(1);
  } finally {
    await browser.close();
  }
}

run();
