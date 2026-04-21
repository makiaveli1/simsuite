/**
 * Phase 5ah — JS Injection Diagnostic
 * Injects console.log to trace folderContents state in the running app
 */

import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  const consoleLogs = [];
  page.on('console', msg => {
    if (msg.type() === 'error' || msg.text().includes('folder') || msg.text().includes('tree') || msg.text().includes('rootFiles') || msg.text().includes('ModsLoose'))
      consoleLogs.push(`[${msg.type()}] ${msg.text()}`);
  });

  try {
    await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });

    // Inject debug logging before entering folder mode
    await page.evaluate(() => {
      // Patch FolderContentPane by monkeypatching the module
      console.log('=== PAGE LOADED, ENTERING FOLDER MODE ===');
    });

    // Enter folder mode
    await page.locator('[aria-label="Folders view"]').click();
    await page.waitForTimeout(5000); // wait for all effects

    // Inject debug AFTER folder mode
    const debugInfo = await page.evaluate(() => {
      // Try to find React fiber for the folder content pane
      const root = document.querySelector('#root');
      if (!root) return { error: 'No root' };

      // Read folder content pane header
      const headerEl = document.querySelector('.folder-content-title');
      const header = headerEl?.textContent ?? 'NOT FOUND';

      // Read loose files section
      const looseSection = document.querySelector('.folder-loose-source-group');
      const looseBadge = document.querySelector('.folder-loose-files-badge');
      const looseBadgeText = looseBadge?.textContent ?? 'NOT FOUND';

      // Read the content pane inner text fully
      const contentPane = document.querySelector('.library-folder-content-pane');
      const fullText = contentPane?.innerText ?? 'NOT FOUND';

      // Check for any virtualized content
      const virtualizers = document.querySelectorAll('[data-react-virtualized]');
      const virtualRows = document.querySelectorAll('.virtualized-row');

      // Check what's in the content area
      const sections = document.querySelectorAll('.folder-content-section');
      const sectionTexts = Array.from(sections).map(s => s.textContent);

      return {
        header,
        looseSectionFound: !!looseSection,
        looseBadgeText,
        fullText: fullText.slice(0, 600),
        virtualRowCount: virtualRows.length,
        sectionCount: sections.length,
        sectionTexts,
      };
    });

    console.log('\n=== DEBUG INFO ===');
    console.log(`Header: "${debugInfo.header}"`);
    console.log(`Loose section found: ${debugInfo.looseSectionFound}`);
    console.log(`Loose badge: "${debugInfo.looseBadgeText}"`);
    console.log(`Sections: ${debugInfo.sectionTexts.join(', ')}`);
    console.log(`Virtual rows: ${debugInfo.virtualRowCount}`);
    console.log(`\nFull content pane text:\n${debugInfo.fullText}`);

    // Filter relevant console logs
    const relevant = consoleLogs.filter(l => !l.includes('[verbose]') && !l.includes('[debug]'));
    if (relevant.length > 0) {
      console.log('\n=== CONSOLE LOGS ===');
      relevant.forEach(l => console.log(l));
    }

    console.log('\n=== DONE ===');
  } catch (err) {
    console.error('Error:', err.message);
    process.exit(1);
  } finally {
    await browser.close();
  }
}

run();
