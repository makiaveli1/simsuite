const { chromium } = require('C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort\\node_modules\\playwright');
const fs = require('fs');

/**
 * SimSort Library Redesign — Playwright Smoke Test
 * Tests the railless toolbar layout with inline Type/Creator/Source filters.
 */
(async () => {
  const sessionFile = 'C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort\\output\\desktop\\libcheck.json';
  const outputDir = 'C:\\Users\\likwi\\OneDrive\\Desktop\\PROJS\\SimSort\\output\\desktop';

  let port;
  try {
    const session = JSON.parse(fs.readFileSync(sessionFile, 'utf8'));
    port = session.port;
  } catch (e) {
    console.error('Could not read session file:', e.message);
    process.exit(1);
  }

  console.log('Connecting to tauri-driver on port ' + port + '...');
  let browser;
  try {
    browser = await chromium.connectOverCDP('http://127.0.0.1:' + port);
  } catch (e) {
    console.error('CDP connect failed:', e.message.split('\n')[0]);
    process.exit(1);
  }

  const ctx = browser.contexts()[0];
  let page = ctx.pages()[0];
  if (!page) {
    page = await ctx.newPage();
    await page.goto('http://localhost:1420', { timeout: 15000 }).catch(async (e) => {
      console.log('goto failed:', e.message.split('\n')[0]);
      await page.goto('http://[::1]:1420', { timeout: 15000 }).catch(() => {});
    });
  }

  await page.waitForTimeout(3000);
  console.log('Page loaded:', page.url());

  const checks = [];
  const fail = (msg) => checks.push('FAIL: ' + msg);
  const pass = (msg) => checks.push('PASS: ' + msg);
  const info = (msg) => checks.push('INFO: ' + msg);

  // ── 1. Toolbar row ───────────────────────────────────────────────────────
  const metrics = await page.locator('.library-toolbar-metrics').count();
  metrics > 0 ? pass('Toolbar metrics visible') : fail('Toolbar metrics NOT found');

  const searchInput = await page.locator('.library-toolbar-search input').count();
  searchInput > 0 ? pass('Search input visible') : fail('Search input NOT found');

  const kindFilter = await page.locator('#lib-filter-kind').count();
  kindFilter > 0 ? pass('Type filter dropdown visible') : fail('Type filter dropdown NOT found');

  const creatorFilter = await page.locator('#lib-filter-creator').count();
  creatorFilter > 0 ? pass('Creator filter dropdown visible') : fail('Creator filter dropdown NOT found');

  const sourceFilter = await page.locator('#lib-filter-source').count();
  sourceFilter > 0 ? pass('Source filter dropdown visible') : fail('Source filter dropdown NOT found');

  const sortControl = await page.locator('.library-sort-control').count();
  sortControl > 0 ? pass('Sort control visible') : fail('Sort control NOT found');

  const advancedBtn = await page.locator('button:has-text("Filters")').count();
  advancedBtn > 0 ? pass('Advanced Filters button visible') : fail('Advanced Filters button NOT found');

  // ── 2. Quick chips ────────────────────────────────────────────────────────
  const quickChips = await page.locator('.library-quick-chip').count();
  quickChips >= 5 ? pass(`Quick chips: ${quickChips} found`) : fail(`Quick chips: only ${quickChips} (expected 5)`);

  const activeChip = await page.locator('.library-quick-chip.is-active').count();
  activeChip >= 1 ? pass('Active quick chip highlighted') : fail('No active chip found');

  // ── 3. Summary strip ──────────────────────────────────────────────────────
  const summary = await page.locator('.library-summary-strip').count();
  summary > 0 ? pass('Summary health strip visible') : info('Summary strip not shown (may be null state)');

  // ── 4. Table rows ─────────────────────────────────────────────────────────
  const rowCount = await page.locator('.library-table tbody tr').count();
  console.log('Table rows:', rowCount);

  if (rowCount > 0) {
    pass(`Table has ${rowCount} row(s)`);

    const accentBars = await page.locator('.type-accent').count();
    accentBars > 0 ? pass(`Type accent bars: ${accentBars}`) : fail('Type accent bars NOT found');

    const typePills = await page.locator('.library-type-pill').count();
    typePills > 0 ? pass(`Type pills: ${typePills}`) : fail('Type pills NOT found');

    const confBadges = await page.locator('.library-confidence-badge').count();
    confBadges > 0 ? pass(`Confidence badges: ${confBadges}`) : info(`No confidence badges (not script mods in sample)`);

    const statusPills = await page.locator('.library-health-pill').count();
    statusPills > 0 ? pass(`Watch status pills: ${statusPills}`) : fail('No watch status pills found');

  } else {
    fail('Table has NO rows');
  }

  // ── 5. Left rail removed ─────────────────────────────────────────────────
  const leftRail = await page.locator('.library-rail-shell').count();
  leftRail === 0 ? pass('Left filter rail removed') : fail('Left filter rail STILL present');

  // ── 6. Inspector ─────────────────────────────────────────────────────────
  const inspector = await page.locator('.library-inspector-shell').count();
  inspector > 0 ? pass('Inspector panel visible') : fail('Inspector panel NOT found');

  const firstRow = page.locator('.library-table tbody tr').first();
  if (await firstRow.count() > 0) {
    await firstRow.click();
    await page.waitForTimeout(500);
    const detailPanel = await page.locator('.library-details-panel, .library-details-empty').count();
    detailPanel > 0 ? pass('Detail panel opened after row click') : fail('Detail panel did not open');
  }

  // ── 7. Filter interaction ─────────────────────────────────────────────────
  const hasUpdatesChip = page.locator('.library-quick-chip:has-text("Has Updates")');
  if (await hasUpdatesChip.count() > 0) {
    await hasUpdatesChip.click();
    await page.waitForTimeout(1000);
    const activeAfter = await page.locator('.library-quick-chip.is-active:has-text("Has Updates")').count();
    activeAfter > 0 ? pass('Has Updates chip activates correctly') : fail('Has Updates chip did not activate');
  }

  // ── 8. Search ────────────────────────────────────────────────────────────
  const searchBox = page.locator('.library-toolbar-search input');
  if (await searchBox.count() > 0) {
    await searchBox.fill('MCCC');
    await page.waitForTimeout(1000);
    const val = await searchBox.inputValue();
    val === 'MCCC' ? pass('Search input accepts text') : fail('Search text not set');
    const clearBtn = page.locator('.search-clear');
    if (await clearBtn.count() > 0) {
      await clearBtn.click();
      await page.waitForTimeout(500);
      const cleared = await searchBox.inputValue();
      cleared === '' ? pass('Search clear works') : fail('Search clear did not work');
    }
  }

  // ── Results ───────────────────────────────────────────────────────────────
  console.log('\n══════════════════════════════════════');
  const fails = checks.filter(c => c.startsWith('FAIL'));
  const passes = checks.filter(c => c.startsWith('PASS'));
  const infos = checks.filter(c => c.startsWith('INFO'));
  passes.forEach(c => console.log('  ' + c));
  infos.forEach(c => console.log('  ' + c));
  fails.forEach(c => console.log('  ' + c));

  const shot = outputDir + '\\library-redesign-smoke.png';
  await page.screenshot({ path: shot, fullPage: false, timeout: 10000 }).catch(e =>
    console.log('Screenshot error:', e.message.split('\n')[0])
  );
  console.log('\nScreenshot:', shot);
  console.log('RESULT:', fails.length === 0 ? 'ALL PASSED' : `${fails.length} FAILURE(S)`);
  console.log('══════════════════════════════════════');

  await browser.close();
  process.exit(fails.length === 0 ? 0 : 1);
})().catch(err => { console.error('Fatal:', err.message.split('\n')[0]); process.exit(1); });
