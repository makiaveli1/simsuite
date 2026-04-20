// Check what clicking Mods folder shows
const { chromium } = require('playwright');
const URL = 'http://localhost:1420';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  
  try {
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    
    // Close guide
    await page.keyboard.press('Escape').catch(() => {});
    await page.waitForTimeout(500);
    
    // Navigate to Library
    await page.locator('text=LIBRARY').first().click();
    await page.waitForTimeout(3000);
    
    // Switch to folder view
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    if (await foldersBtn.isVisible({ timeout: 3000 })) {
      await foldersBtn.click();
      await page.waitForTimeout(4000);
    }
    
    console.log('=== ALL FOLDERS STATE ===');
    const summary = await page.locator('[class*="folder-content-summary"]').first().textContent({ timeout: 2000 }).catch(() => '?');
    const sections = await page.locator('[class*="folder-content-section"]').allTextContents();
    console.log('Summary:', summary?.trim());
    console.log('Sections:', sections);
    const treeRows = await page.locator('[class*="folder-row"]').all();
    console.log('Tree rows:', treeRows.length);
    for (const row of treeRows.slice(0, 8)) {
      const name = await row.locator('[class*="folder-row__name"]').textContent().catch(() => '?');
      const count = await row.locator('[class*="folder-row__count"]').textContent().catch(() => '?');
      console.log(`  "${name?.trim()}" - ${count?.trim()}`);
    }
    
    // Click Mods in the sidebar
    console.log('\n=== AFTER CLICKING "Mods" IN SIDEBAR ===');
    const modsRow = page.locator('[class*="folder-row"]:has-text("Mods")').first();
    if (await modsRow.isVisible({ timeout: 2000 })) {
      await modsRow.click();
      await page.waitForTimeout(3000);
      
      const breadcrumb = await page.locator('[class*="folder-breadcrumb"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      const newSummary = await page.locator('[class*="folder-content-summary"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      const newSections = await page.locator('[class*="folder-content-section"]').allTextContents();
      const looseSections = await page.locator('[class*="loose-files"], [class*="loose-source-group"]').allTextContents();
      console.log('Breadcrumb:', breadcrumb?.trim());
      console.log('Summary:', newSummary?.trim());
      console.log('Content sections:', newSections);
      console.log('Loose files sections:', looseSections.length);
      for (const ls of looseSections.slice(0, 3)) {
        console.log(`  "${ls?.trim().slice(0, 100)}"`);
      }
    } else {
      console.log('Mods row NOT found!');
    }
    
    // Also try clicking in content pane if available
    const contentMods = page.locator('[class*="folder-row"]:has-text("Mods")').first();
    const contentPane = page.locator('[class*="folder-content-pane"]').first();
    const contentVisible = await contentPane.isVisible({ timeout: 1000 }).catch(() => false);
    if (contentVisible) {
      console.log('\n=== CLICKING "Mods" IN CONTENT PANE ===');
      const contentRows = await page.locator('[class*="folder-content-pane"] [class*="folder-row"]').all();
      console.log('Content pane folder rows:', contentRows.length);
      for (const row of contentRows.slice(0, 5)) {
        const name = await row.textContent().catch(() => '?');
        console.log(`  "${name?.trim().slice(0, 80)}"`);
      }
    }
    
    console.log('\nErrors:', errors.length ? errors.slice(0, 5) : 'none');
    await page.screenshot({ path: '/tmp/simsuite-mods-click.png', fullPage: false });
    console.log('Screenshot: /tmp/simsuite-mods-click.png');
  } finally {
    await browser.close();
  }
}

run().catch(e => { console.error('Failed:', e.message); process.exit(1); });
