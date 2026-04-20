// Targeted folder content diagnosis
const { chromium } = require('playwright');

const URL = 'http://localhost:1420';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();
  
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  
  try {
    console.log('Step 1: Navigate...');
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    
    // Close guide overlay
    try {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(500);
    } catch(e) {}
    const closeBtn = page.locator('button:has-text("Close")').first();
    if (await closeBtn.isVisible({ timeout: 1000 })) {
      await closeBtn.click();
      await page.waitForTimeout(500);
    }
    
    console.log('Step 2: Click Library nav...');
    const libNav = page.locator('text=Library').first();
    if (await libNav.isVisible({ timeout: 3000 })) {
      await libNav.click();
      console.log('Clicked Library');
      await page.waitForTimeout(3000);
    } else {
      // Try the sidebar nav item by aria-label or role
      const sidebarLib = page.locator('[aria-label*="Library"], [aria-label*="library"]').first();
      if (await sidebarLib.isVisible({ timeout: 1000 })) {
        await sidebarLib.click();
        console.log('Clicked Library (aria)');
        await page.waitForTimeout(3000);
      }
    }
    
    console.log('Step 3: Switch to folder view...');
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    if (await foldersBtn.isVisible({ timeout: 3000 })) {
      await foldersBtn.click();
      console.log('Clicked Folders view');
      await page.waitForTimeout(4000);
    } else {
      console.log('Folders view button NOT visible - checking available buttons:');
      const btns = await page.locator('button').all();
      for (const btn of btns.slice(0, 10)) {
        const aria = await btn.getAttribute('aria-label').catch(() => null);
        const text = await btn.textContent().catch(() => '?');
        console.log(`  btn: "${text?.trim().slice(0,30)}" aria="${aria}"`);
      }
    }
    
    console.log('\nStep 4: Analyze folder mode...');
    
    // Check breadcrumb
    const breadcrumb = page.locator('[class*="folder-breadcrumb"]').first();
    const breadcrumbText = await breadcrumb.textContent({ timeout: 2000 }).catch(() => 'not found');
    console.log('Breadcrumb:', breadcrumbText?.trim());
    
    // Check summary
    const summary = page.locator('[class*="folder-content-summary"]').first();
    const summaryText = await summary.textContent({ timeout: 2000 }).catch(() => 'not found');
    console.log('Summary:', summaryText?.trim());
    
    // Check content sections
    const sections = await page.locator('[class*="folder-content-section"]').all();
    console.log(`Content sections: ${sections.length}`);
    for (const sec of sections) {
      const text = await sec.textContent().catch(() => '?');
      console.log(`  "${text?.trim().slice(0, 100)}"`);
    }
    
    // Check folder rows in sidebar
    const treeRows = await page.locator('[class*="folder-row"]').all();
    console.log(`\nFolder tree rows in sidebar: ${treeRows.length}`);
    for (const row of treeRows.slice(0, 5)) {
      const name = await row.locator('[class*="folder-row__name"]').textContent().catch(() => '?');
      const count = await row.locator('[class*="folder-row__count"]').textContent().catch(() => '?');
      console.log(`  "${name?.trim()}" - ${count?.trim()}`);
    }
    
    // Check root summary in sidebar
    const rootSummary = page.locator('.folder-root-summary, [class*="folder-root"]').first();
    const rootVisible = await rootSummary.isVisible({ timeout: 1000 }).catch(() => false);
    if (rootVisible) {
      const rootText = await rootSummary.textContent();
      console.log('\nRoot summary:', rootText?.trim().slice(0, 100));
    }
    
    // Check empty state
    const emptyState = page.locator('[class*="library-list-empty"]').first();
    const emptyVisible = await emptyState.isVisible({ timeout: 1000 }).catch(() => false);
    const emptyText = emptyVisible ? await emptyState.textContent() : 'not visible';
    console.log('Empty state:', emptyText);
    
    // Click "Mods" folder in tree
    const modsRow = page.locator('[class*="folder-row"]:has-text("Mods")').first();
    if (await modsRow.isVisible({ timeout: 2000 })) {
      console.log('\nClicking "Mods" in tree...');
      await modsRow.click();
      await page.waitForTimeout(2000);
      
      const newBreadcrumb = await page.locator('[class*="folder-breadcrumb"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      const newSummary = await page.locator('[class*="folder-content-summary"]').first().textContent({ timeout: 2000 }).catch(() => '?');
      console.log('After clicking Mods - breadcrumb:', newBreadcrumb?.trim());
      console.log('After clicking Mods - summary:', newSummary?.trim());
      
      const newSections = await page.locator('[class*="folder-content-section"]').all();
      console.log('After clicking Mods - sections:', newSections.length);
      for (const sec of newSections) {
        const text = await sec.textContent().catch(() => '?');
        console.log(`  "${text?.trim().slice(0, 100)}"`);
      }
    } else {
      console.log('\n"Mods" folder row not found in sidebar');
    }
    
    console.log('\nErrors:', errors.length ? errors.slice(0, 5) : 'none');
    await page.screenshot({ path: '/tmp/simsuite-diagnose2.png', fullPage: false });
    console.log('Screenshot: /tmp/simsuite-diagnose2.png');
    
  } finally {
    await browser.close();
  }
}

run().catch(e => { console.error('Failed:', e.message); process.exit(1); });
