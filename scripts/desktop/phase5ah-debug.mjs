/**
 * Phase 5ah — Capture FolderContentPane props via injected debugging
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on('pageerror', err => errors.push(`PAGE: ${err.message}`));
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(`CONSOLE: ${msg.text()}`);
  });

  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(4000); // wait extra long for all data

  // Inject a script to find the FolderContentPane React fiber and capture its memorized values
  const debugInfo = await page.evaluate(() => {
    // Try to find the library-folders-layout and inspect children
    const layout = document.querySelector('.library-folders-layout');
    if (!layout) return { error: 'no layout found' };

    const contentPane = layout.querySelector('.library-folder-content-pane');
    if (!contentPane) return { error: 'no content pane in layout' };

    // Get all sections in the content pane
    const sections = Array.from(contentPane.querySelectorAll('section')).map((s, i) => ({
      index: i,
      className: s.className || '(none)',
      childCount: s.childElementCount,
      text: s.textContent?.trim().slice(0, 200)
    }));

    // Check the raw HTML structure more carefully
    const html = contentPane.innerHTML;

    // Find ALL child elements that are sections
    const allSections = contentPane.querySelectorAll('section');

    // Get all direct children of content pane
    const directChildren = Array.from(contentPane.children).map((child, i) => ({
      index: i,
      tag: child.tagName,
      className: child.className,
      childCount: child.childElementCount
    }));

    return {
      htmlLength: html.length,
      sectionCount: allSections.length,
      sections: Array.from(allSections).map(s => ({
        className: s.className,
        text: s.textContent?.slice(0, 100),
        children: s.children.length
      })),
      directChildren,
      first500html: html.slice(0, 500)
    };
  });

  console.log('=== FolderContentPane Structure ===');
  console.log(JSON.stringify(debugInfo, null, 2));

  // Also check if there are any hidden or conditional sections
  const rawHtml = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    return cp ? cp.innerHTML : 'NOT FOUND';
  });
  console.log('\n=== Raw Content Pane HTML (500 chars) ===');
  console.log(rawHtml.slice(0, 500));

  // Count how many times "section" appears as a tag in the full HTML
  const sectionMatches = rawHtml.match(/<section[^>]*>/gi) || [];
  console.log('\nSection tags in content pane HTML:', sectionMatches.length);
  console.log('Section matches:', sectionMatches);

  if (errors.length > 0) {
    console.log('\nErrors:');
    errors.forEach(e => console.log('  ', e));
  }

  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });