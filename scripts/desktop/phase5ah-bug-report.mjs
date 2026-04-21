/**
 * Phase 5ah — SimSuite Bug Verification Report
 * Tests all 3 bugs with live Playwright + simulated path logic
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
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(5000);

  console.log('=== SimSuite Folder-Mode Bug Verification ===\n');

  // ── BUG 1: Folder navigation speed ──────────────────────────────────────────
  console.log('【Bug 1】Folder navigation speed');
  console.log('  Testing: Mods → Gameplay → first file click');

  const t0 = Date.now();
  await page.locator('.folder-row', { hasText: 'Mods' }).click();
  await page.waitForTimeout(1500);
  const t1 = Date.now();
  console.log(`  Mods→content: ${t1-t0}ms`);

  const gameplayRow = page.locator('.folder-row', { hasText: 'Gameplay' });
  if (await gameplayRow.count() > 0) {
    await gameplayRow.click();
    await page.waitForTimeout(1500);
    const t2 = Date.now();
    console.log(`  Gameplay→content: ${t2-t1}ms`);

    const tableRows = await page.locator('.library-folder-content-pane .library-list-row').count();
    console.log(`  Table rows visible: ${tableRows}`);

    if (tableRows > 0) {
      await page.locator('.library-folder-content-pane .library-list-row').first().click();
      await page.waitForTimeout(3000);
      const t3 = Date.now();
      console.log(`  File click→detail: ${t3-t2}ms`);

      const blankCheck = await page.evaluate(() => {
        const body = document.body.innerText.trim();
        const detail = document.querySelector('.library-inspector-panel, #library-detail-sheet');
        return { bodyLen: body.length, hasDetail: !!detail };
      });
      console.log(`  Body: ${blankCheck.bodyLen} chars, Detail panel: ${blankCheck.hasDetail}`);
      console.log(`  ${blankCheck.bodyLen < 50 && !blankCheck.hasDetail ? '  ❌ BLANK SCREEN' : '  ✓ No blank screen'}`);
    } else {
      console.log('  ⚠️  No table rows to click — empty subfolder?');
    }
  } else {
    console.log('  ❌ Gameplay subfolder not found');
  }

  // ── BUG 2: Loose-files hover jumble ────────────────────────────────────────
  console.log('\n【Bug 2】Loose-files hover jumble');
  console.log('  (Requires manual hover test — checking CSS vars instead)');

  const cssVars = await page.evaluate(() => {
    const row = document.querySelector('.virtualized-row');
    if (!row) return null;
    const style = window.getComputedStyle(row);
    return {
      transform: style.transform,
      willChange: style.willChange,
      contain: style.contain,
    };
  });
  console.log('  Virtualized row CSS:', JSON.stringify(cssVars));

  // Check virtualizer container contain
  const containerContain = await page.evaluate(() => {
    const sc = document.querySelector('.virtualized-loose-files__scroll-container, .virtualized-loose-files__inner');
    return sc ? window.getComputedStyle(sc).contain : 'not found';
  });
  console.log('  Scroll container contain:', containerContain);

  // Navigate back and test loose-files section at root
  await page.locator('.folder-content-header').first().click().catch(() => {});
  await page.waitForTimeout(1000);

  // ── BUG 3: Loose-files section missing ─────────────────────────────────────
  console.log('\n【Bug 3】Loose-files section missing at root');

  const sections = await page.locator('.library-folder-content-pane section').count();
  console.log(`  Total sections at root: ${sections}`);

  const sectionLabels = await page.locator('.folder-content-section').allTextContents();
  console.log(`  Section labels:`, sectionLabels);

  const looseSection = await page.locator('.library-folder-content-pane .folder-loose-source-group').count();
  console.log(`  Loose-files sections: ${looseSection}`);
  console.log(`  ${looseSection === 0 ? '  ❌ MISSING (the bug)' : '  ✓ Present'}`);

  // Check summary vs actual sections
  const summary = await page.locator('.folder-content-summary').textContent().catch(() => '');
  console.log(`  Summary text: "${summary}"`);

  if (errors.length > 0) {
    console.log('\nErrors detected:');
    errors.forEach(e => console.log('  ⚠️ ', e));
  } else {
    console.log('\n  ✓ No JS errors');
  }

  console.log('\n=== DONE ===');
  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });