/**
 * Phase 5ah — Navigation test: click through folder hierarchy
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  // Enter folder mode
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(3000);

  // Click "Mods" row (not in tree pane, in content pane)
  const modsRow = page.locator('.folder-row').filter({ hasText: /^Mods$/ }).first();
  if (await modsRow.count() > 0) {
    console.log('Clicking Mods...');
    await modsRow.click();
    await page.waitForTimeout(1200);

    const header = await page.locator('.folder-content-header span').textContent().catch(() => '');
    const summary = await page.locator('.folder-content-summary').textContent().catch(() => '');
    const sections = await page.locator('.folder-content-pane section, .library-folder-content-pane section').count();
    console.log(`After Mods click — header: "${header}", summary: "${summary}", sections: ${sections}`);

    // Should see 3 subfolders: Gameplay, Options, Repository
    const subfolders = await page.locator('.folder-row').all();
    console.log(`Subfolders: ${subfolders.length}`);
    for (const sf of subfolders) {
      const text = await sf.textContent().catch(() => '');
      console.log(`  - "${text}"`);
    }

    // Click "Gameplay" (156 files)
    const gameplayRow = page.locator('.folder-row').filter({ hasText: /Gameplay/ }).first();
    if (await gameplayRow.count() > 0) {
      console.log('\nClicking Gameplay...');
      await gameplayRow.click();
      await page.waitForTimeout(1500);

      const header2 = await page.locator('.folder-content-header span').textContent().catch(() => '');
      const summary2 = await page.locator('.folder-content-summary').textContent().catch(() => '');
      const tableRows = await page.locator('.library-folder-content-pane .library-list-row').count();
      const emptyState = await page.evaluate(() => document.body.innerText.includes('empty'));
      console.log(`Gameplay header: "${header2}", summary: "${summary2}", table rows: ${tableRows}, empty: ${emptyState}`);

      // Test clicking a row (blank screen test)
      if (tableRows > 0) {
        console.log('\nClicking first file row...');
        const firstRow = page.locator('.library-folder-content-pane .library-list-row').first();
        await firstRow.click();
        await page.waitForTimeout(3000);

        const bodyLen = await page.evaluate(() => document.body.innerText.trim().length);
        const detailVisible = await page.locator('.library-inspector-panel, #library-detail-sheet').isVisible().catch(() => false);
        const blankState = bodyLen < 100 && !detailVisible;
        console.log(`Body text: ${bodyLen} chars, Detail panel: ${detailVisible}`);
        console.log(`${blankState ? '✗ BLANK SCREEN DETECTED' : '✓ No blank screen'}`);
      }
    }

    // Also test the "Loose files in Mods" section at root
    // Navigate back
    await page.locator('.folder-content-header').first().click().catch(() => {});
    await page.waitForTimeout(800);

    // Check for any section after the folders section
    const allSections = await page.locator('.library-folder-content-pane section').count();
    console.log(`\nTotal sections at root: ${allSections}`);
    const allSectionTexts = await page.locator('.library-folder-content-pane section').allTextContents();
    console.log('Section texts:', allSectionTexts.map(t => t.slice(0, 100)));
  } else {
    console.log('No Mods row found');
  }

  console.log('\n=== DONE ===');
  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });