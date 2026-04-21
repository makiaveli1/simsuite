/**
 * Phase 5ah — Full navigation test with correct selectors
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', err => errors.push(err.message));
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(`console: ${msg.text()}`);
  });

  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  // Click Folders view
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(6000); // wait for full data load

  console.log('=== Folder Mode Navigation ===\n');

  // ── ROOT LEVEL ────────────────────────────────────────────────────────────
  console.log('【Root Level】');
  const rootInfo = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const sections = Array.from(cp?.querySelectorAll('section') || []);
    const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section') || []).map(s => s.textContent?.trim());
    return {
      summary: cp?.querySelector('.folder-content-summary')?.textContent,
      sectionCount: sections.length,
      sectionLabels,
      looseGroups: cp?.querySelectorAll('.folder-loose-source-group').length || 0,
      header: cp?.querySelector('.folder-content-title span')?.textContent,
      sectionHTML: sections.map(s => s.innerHTML.slice(0, 100)),
    };
  });
  console.log('  Header:', rootInfo.header);
  console.log('  Summary:', rootInfo.summary);
  console.log('  Section labels:', rootInfo.sectionLabels);
  console.log('  Loose sections:', rootInfo.looseGroups);
  console.log('  Total <section> count:', rootInfo.sectionCount);
  console.log('  Section HTMLs:', rootInfo.sectionHTML);

  // ── MODS FOLDER ───────────────────────────────────────────────────────────
  console.log('\n【Clicking Mods】');
  await page.locator('.folder-row', { hasText: 'Mods' }).click();
  await page.waitForTimeout(2000);

  const modsInfo = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const sections = Array.from(cp?.querySelectorAll('section') || []);
    const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section') || []).map(s => s.textContent?.trim());
    const allRows = Array.from(cp?.querySelectorAll('.folder-row, .library-list-row') || []).map(r => r.textContent?.trim().slice(0, 60));
    return {
      summary: cp?.querySelector('.folder-content-summary')?.textContent,
      header: cp?.querySelector('.folder-content-title span')?.textContent,
      sectionCount: sections.length,
      sectionLabels,
      allRows: allRows.slice(0, 10),
      totalSections: sections.length,
    };
  });
  console.log('  Header:', modsInfo.header);
  console.log('  Summary:', modsInfo.summary);
  console.log('  Section labels:', modsInfo.sectionLabels);
  console.log('  All folder/file rows:', modsInfo.allRows);

  // ── GAMEPLAY SUBFOLDER ────────────────────────────────────────────────────
  const gameplayRow = page.locator('.folder-row', { hasText: 'Gameplay' });
  if (await gameplayRow.count() > 0) {
    console.log('\n【Clicking Gameplay】');
    await gameplayRow.click();
    await page.waitForTimeout(2000);

    const gameplayInfo = await page.evaluate(() => {
      const cp = document.querySelector('.library-folder-content-pane');
      const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section') || []).map(s => s.textContent?.trim());
      const listRows = Array.from(cp?.querySelectorAll('.library-list-row') || []).length;
      const folderRows = Array.from(cp?.querySelectorAll('.folder-row') || []).map(r => r.textContent?.trim());
      return {
        header: cp?.querySelector('.folder-content-title span')?.textContent,
        summary: cp?.querySelector('.folder-content-summary')?.textContent,
        sectionLabels,
        listRows,
        folderRows,
      };
    });
    console.log('  Header:', gameplayInfo.header);
    console.log('  Summary:', gameplayInfo.summary);
    console.log('  Section labels:', gameplayInfo.sectionLabels);
    console.log('  Library list rows:', gameplayInfo.listRows);
    console.log('  Folder rows:', gameplayInfo.folderRows);

    // ── FILE CLICK (blank screen test) ──────────────────────────────────────
    if (gameplayInfo.listRows > 0) {
      console.log('\n【Clicking first file row】');
      const t0 = Date.now();
      await page.locator('.library-list-row').first().click();
      await page.waitForTimeout(3000);
      const t1 = Date.now();

      const blankInfo = await page.evaluate(() => {
        const detail = document.querySelector('.library-inspector-panel, #library-detail-sheet, [class*="detail"]');
        const body = document.body.innerText.trim();
        return {
          detailExists: !!detail,
          bodyLen: body.length,
          bodyPreview: body.slice(0, 100),
        };
      });
      console.log(`  Click→render: ${t1-t0}ms`);
      console.log('  Detail panel:', blankInfo.detailExists);
      console.log('  Body len:', blankInfo.bodyLen);
      console.log('  Body preview:', blankInfo.bodyPreview);
      console.log(blankInfo.bodyLen < 80 && !blankInfo.detailExists ? '  ❌ BLANK SCREEN' : '  ✓ No blank screen');
    }
  }

  // ── NAVIGATE BACK TO ROOT — CHECK LOOSE FILES ───────────────────────────────
  console.log('\n【Back to root — checking loose files】');
  // Click the header/folder icon to go back
  const backBtn = page.locator('.folder-content-title').first();
  await backBtn.click().catch(() => {});
  await page.waitForTimeout(2000);

  const backInfo = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const sections = Array.from(cp?.querySelectorAll('section') || []);
    const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section') || []).map(s => s.textContent?.trim());
    const looseGroups = cp?.querySelectorAll('.folder-loose-source-group').length || 0;
    return {
      summary: cp?.querySelector('.folder-content-summary')?.textContent,
      header: cp?.querySelector('.folder-content-title span')?.textContent,
      sectionLabels,
      looseGroups,
    };
  });
  console.log('  Header:', backInfo.header);
  console.log('  Summary:', backInfo.summary);
  console.log('  Section labels:', backInfo.sectionLabels);
  console.log('  Loose source groups:', backInfo.looseGroups);

  if (errors.length > 0) {
    console.log('\nErrors:');
    errors.forEach(e => console.log('  ⚠️ ', e));
  } else {
    console.log('\n  ✓ No JS errors');
  }

  console.log('\n=== DONE ===');
  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });