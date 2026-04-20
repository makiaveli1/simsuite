// Check what happens when clicking Mods
const { chromium } = require('playwright');
const URL = 'http://localhost:1420';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  
  try {
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    await page.keyboard.press('Escape').catch(() => {});
    await page.locator('text=LIBRARY').first().click();
    await page.waitForTimeout(3000);
    
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    if (await foldersBtn.isVisible({ timeout: 3000 })) {
      await foldersBtn.click();
      await page.waitForTimeout(5000); // extra wait for full load
    }
    
    console.log('=== INITIAL STATE (All folders) ===');
    const summary = await page.locator('[class*="folder-content-summary"]').first().textContent({ timeout: 2000 }).catch(() => '?');
    const breadcrumb = await page.locator('[class*="folder-breadcrumb"]').first().textContent({ timeout: 2000 }).catch(() => '?');
    console.log('Breadcrumb:', breadcrumb?.trim());
    console.log('Summary:', summary?.trim());
    
    const treeRows = await page.locator('[class*="folder-row"]').all();
    console.log('Tree rows in sidebar:', treeRows.length);
    for (const row of treeRows.slice(0, 10)) {
      const name = await row.locator('[class*="folder-row__name"]').textContent().catch(() => '?');
      const count = await row.locator('[class*="folder-row__count"]').textContent().catch(() => '?');
      if (name?.trim()) console.log(`  [${name?.trim()}] - ${count?.trim()}`);
    }
    
    // Try clicking each tree row and check content
    console.log('\n=== CLICKING EACH VISIBLE FOLDER ===');
    for (const row of treeRows.slice(1)) {  // skip breadcrumb row
      const name = await row.locator('[class*="folder-row__name"]').textContent().catch(() => '?');
      const count = await row.locator('[class*="folder-row__count"]').textContent().catch(() => '?');
      if (!name?.trim() || name?.trim() === '?') continue;
      
      console.log(`\n  Clicking: "${name?.trim()}" (${count?.trim()})`);
      await row.click();
      await page.waitForTimeout(2000);
      
      const newBreadcrumb = await page.locator('[class*="folder-breadcrumb"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      const newSummary = await page.locator('[class*="folder-content-summary"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      const sections = await page.locator('[class*="folder-content-section"]').allTextContents();
      console.log(`  Breadcrumb: ${newBreadcrumb?.trim()}`);
      console.log(`  Summary: ${newSummary?.trim()}`);
      console.log(`  Sections: ${sections?.slice(0, 3)}`);
      
      // Re-click All folders to reset
      const allFoldersEl = page.locator('text=All folders').first();
      if (await allFoldersEl.isVisible({ timeout: 1000 })) {
        await allFoldersEl.click();
        await page.waitForTimeout(1000);
      }
    }
    
    console.log('\nErrors:', errors.slice(0, 3));
    await page.screenshot({ path: '/tmp/simsuite-click-all.png', fullPage: false });
    console.log('Screenshot: /tmp/simsuite-click-all.png');
  } finally {
    await browser.close();
  }
}

run().catch(e => { console.error('Failed:', e.message); process.exit(1); });
