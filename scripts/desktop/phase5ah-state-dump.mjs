/**
 * Phase 5ah — Full state dump: capture FolderContentPane rendering inputs
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.on('console', msg => console.log(`[browser ${msg.type()}] ${msg.text()}`));

  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(5000);

  // Inject spy to capture React component state
  const info = await page.evaluate(() => {
    // Find React fiber for FolderContentPane
    const allElements = Array.from(document.querySelectorAll('*'));
    let folderContentPaneFiber = null;

    // @ts-ignore
    const React = window.React;
    if (!React) {
      // Try to find via the fiber
      const rootEl = document.querySelector('#root') || document.querySelector('.app-root');
      if (rootEl) {
        // @ts-ignore
        const fiberHostRoot = rootEl[Object.keys(rootEl).find(k => k.startsWith('__reactFiber'))];
        if (fiberHostRoot) {
          // Walk the fiber tree looking for FolderContentPane
          function findFiber(fiber, depth = 0) {
            if (depth > 30) return null;
            const key = Object.keys(fiber).find(k => k.startsWith('__reactFiber'));
            if (!key) return null;
            const actualFiber = fiber[key];
            if (!actualFiber) return null;
            if (actualFiber.elementType && actualFiber.elementType.displayName === 'FolderContentPane') {
              return actualFiber;
            }
            if (actualFiber.child) {
              return findFiber(actualFiber.child, depth + 1) ||
                     (actualFiber.sibling ? findFiber(actualFiber.sibling, depth) : null);
            }
            return null;
          }
          const found = findFiber(rootEl);
          if (found) {
            return {
              props: JSON.stringify(Object.keys(found.memoizedProps)),
              state: JSON.stringify(Object.keys(found.memoizedState || {})),
            };
          }
        }
      }
      return { error: 'React not found, no spy possible' };
    }

    return { error: 'React not found via window.React' };
  });

  console.log('React fiber info:', JSON.stringify(info));

  // Try a different approach — read the actual rendered section count
  // and check what the summary says vs what sections appear
  const analysis = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const sections = cp ? Array.from(cp.querySelectorAll('section')).map(s => ({
      className: s.className || '(none)',
      text: s.textContent?.trim().slice(0, 80)
    })) : [];

    const header = document.querySelector('.folder-content-header .folder-content-summary')?.textContent;
    const sectionsText = Array.from(cp?.querySelectorAll('.folder-content-section') || []).map(s => s.textContent);

    // Also count all top-level divs/sections in the content pane
    const directKids = cp ? Array.from(cp.children).map(c => ({
      tag: c.tagName,
      class: c.className,
      children: c.childElementCount
    })) : [];

    return { sections, header, sectionsText, directKids, totalSections: sections.length };
  });

  console.log('\n=== FolderContentPane Full Analysis ===');
  console.log(JSON.stringify(analysis, null, 2));

  // Check if there's a second section hidden somewhere with opacity 0
  const allOpacity = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    return Array.from(cp?.querySelectorAll('*') || []).map(el => {
      const style = window.getComputedStyle(el);
      return {
        tag: el.tagName,
        class: el.className.slice(0, 40),
        opacity: style.opacity,
        display: style.display,
        height: style.height,
        width: style.width
      };
    }).filter(x => x.opacity !== '1' || x.display === 'none');
  });
  console.log('\nNon-visible elements:', JSON.stringify(allOpacity.slice(0, 20)));

  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });