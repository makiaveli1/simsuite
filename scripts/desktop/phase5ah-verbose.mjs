/**
 * Phase 5ah — Diagnostic: why is loose-files section not visible?
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', err => errors.push(`PAGE: ${err.message}`));
  page.on('console', msg => { if (msg.type() === 'error') errors.push(`CONSOLE: ${msg.text()}`); });

  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  // Enter folder mode
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(3000);

  // Dump the entire content pane HTML structure
  const contentPaneHtml = await page.evaluate(() => {
    const el = document.querySelector('.library-folder-content-pane');
    if (!el) return 'NOT FOUND';
    return el.innerHTML;
  });
  console.log('Content pane HTML (first 2000 chars):\n', contentPaneHtml.slice(0, 2000));

  // Count sections
  const sectionCount = await page.locator('.folder-content-pane section, .library-folder-content-pane section').count();
  console.log('\nSection count:', sectionCount);

  // Check for any section with "Loose" text
  const looseSection = await page.locator(':text("loose")').count();
  console.log('Elements containing "loose":', looseSection);

  // Check for any .folder-load-more buttons
  const loadMoreBtns = await page.locator('.folder-load-more').count();
  console.log('Show-all buttons:', loadMoreBtns);

  // Check for VirtualizedLooseFiles component
  const virtualizedEl = await page.locator('.virtualized-loose-files').count();
  console.log('Virtualized loose files elements:', virtualizedEl);

  // Check ModsLooseFilesSection wrapper
  const modsLooseSection = await page.locator('.folder-mods-loose-files').count();
  console.log('Mods loose-files section elements:', modsLooseSection);

  // Check what the summary says about loose files
  const summaryText = await page.evaluate(() => document.querySelector('.folder-content-summary')?.textContent ?? '');
  console.log('\nSummary:', summaryText);

  // List ALL class names in the content pane
  const allClasses = await page.evaluate(() => {
    const el = document.querySelector('.library-folder-content-pane');
    if (!el) return [];
    return Array.from(el.querySelectorAll('*')).map(e => e.className).filter(Boolean).slice(0, 30);
  });
  console.log('\nFirst 30 class names in content pane:', allClasses);

  if (errors.length > 0) {
    console.log('\nErrors:');
    errors.forEach(e => console.log(`  ${e}`));
  }

  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });