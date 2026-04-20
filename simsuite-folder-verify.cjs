// Level 2 folder mode verification - proper navigation
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
    console.log('Step 1: Navigate to SimSuite...');
    await page.goto(URL, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    
    console.log('Step 2: Go to Library...');
    await page.locator('text=LIBRARY').first().click();
    await page.waitForTimeout(2000);
    
    // Close guide overlay if open
    console.log('Step 3: Close guide overlay if visible...');
    try {
      const escKey = page.keyboard.press('Escape');
      await page.waitForTimeout(500);
      console.log('Pressed Escape');
    } catch(e) {}
    
    // Look for Close button
    try {
      const closeBtn = page.locator('button:has-text("Close")').first();
      if (await closeBtn.isVisible({ timeout: 1000 })) {
        await closeBtn.click();
        await page.waitForTimeout(500);
        console.log('Clicked Close button');
      }
    } catch(e) {}
    
    console.log('Step 4: Find and click folder view button...');
    // Look for Folders view button (aria-label)
    const foldersBtn = page.locator('[aria-label="Folders view"]');
    if (await foldersBtn.isVisible({ timeout: 3000 })) {
      await foldersBtn.click();
      console.log('Clicked "Folders view" button');
      await page.waitForTimeout(3000);
    } else {
      // Try by title or text
      const folderBtn = page.locator('button[title*="Folder"], button:has-text("Folder")').first();
      if (await folderBtn.isVisible({ timeout: 2000 })) {
        await folderBtn.click();
        console.log('Clicked folder button');
        await page.waitForTimeout(3000);
      }
    }
    
    console.log('Step 5: Check folder mode state...');
    
    // Check breadcrumb text
    const allFoldersEl = page.locator('text=All folders').first();
    const hasAllFolders = await allFoldersEl.isVisible({ timeout: 3000 }).catch(() => false);
    console.log('"All folders" breadcrumb visible:', hasAllFolders);
    
    // Check file count display
    const body = await page.textContent('body');
    const hasZeroFiles = body.includes('0 files');
    const hasEmptyState = body.includes('This folder is empty');
    console.log('Page contains "0 files":', hasZeroFiles);
    console.log('Page contains "This folder is empty":', hasEmptyState);
    
    // Check tree pane area
    const folderTreePane = page.locator('.folder-tree-pane, [class*="folder-tree-pane"]').first();
    const treeVisible = await folderTreePane.isVisible({ timeout: 3000 }).catch(() => false);
    console.log('Tree pane element visible:', treeVisible);
    
    // Check content pane
    const contentPane = page.locator('.folder-content-pane, [class*="folder-content"]').first();
    const contentVisible = await contentPane.isVisible({ timeout: 3000 }).catch(() => false);
    console.log('Content pane visible:', contentVisible);
    
    // Get sidebar state
    const sidebarItem = page.locator('.rail-nav .is-selected, [class*="rail-nav"] [class*="selected"]').first();
    const selectedText = await sidebarItem.textContent({ timeout: 2000 }).catch(() => null);
    console.log('Selected sidebar item:', selectedText);
    
    // Count visible items in tree
    const treeRows = await page.locator('.folder-tree-row, [class*="tree-row"]').all();
    console.log('Tree rows visible:', treeRows.length);
    
    console.log('\nConsole errors:', errors.length ? errors.slice(0, 5) : 'none');
    await page.screenshot({ path: '/tmp/simsuite-folder-test3.png', fullPage: true });
    console.log('Screenshot: /tmp/simsuite-folder-test3.png');
    
  } finally {
    await browser.close();
  }
}

run().catch(e => { console.error('Failed:', e.message); process.exit(1); });