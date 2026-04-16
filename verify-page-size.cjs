const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  await page.goto('http://localhost:1420/#library', { waitUntil: 'networkidle', timeout: 30000 });
  
  // 1. Test items-per-page control — does changing it actually change rendered count?
  const initialRowCount = await page.locator('.library-grid-row, .library-card, .library-list-row').count();
  console.log('INITIAL_RENDER_COUNT:', initialRowCount);
  
  // Find and interact with page-size control
  const pageSizeControl = page.locator('select').filter({ hasText: /50|100|200|500/ });
  const controlCount = await pageSizeControl.count();
  console.log('PAGE_SIZE_CONTROLS:', controlCount);
  
  if (controlCount > 0) {
    const initialOptions = await pageSizeControl.first().locator('option').allTextContents();
    console.log('PAGE_SIZE_OPTIONS:', JSON.stringify(initialOptions));
    
    // Change to 50
    await pageSizeControl.first().selectOption('50');
    await page.waitForTimeout(500);
    
    const after50Count = await page.locator('.library-grid-row, .library-card, .library-list-row').count();
    console.log('AFTER_50_COUNT:', after50Count);
    
    // Change to 200
    await pageSizeControl.first().selectOption('200');
    await page.waitForTimeout(500);
    
    const after200Count = await page.locator('.library-grid-row, .library-card, .library-list-row').count();
    console.log('AFTER_200_COUNT:', after200Count);
    
    // Change back to 100
    await pageSizeControl.first().selectOption('100');
    await page.waitForTimeout(500);
    
    const after100Count = await page.locator('.library-grid-row, .library-card, .library-list-row').count();
    console.log('AFTER_100_COUNT:', after100Count);
  }
  
  // 2. Check card density control
  const densityControl = page.locator('select').filter({ hasText: /compact|balanced|comfortable/i });
  const densityCount = await densityControl.count();
  console.log('DENSITY_CONTROLS:', densityCount);
  
  // 3. Check what view we're in
  const url = page.url();
  console.log('URL:', url);
  
  // 4. Get body text to understand what's visible
  const bodyText = await page.evaluate(() => document.body.innerText.slice(0, 300));
  console.log('BODY_TEXT:', bodyText.replace(/\n+/g, ' ').slice(0, 200));
  
  // 5. Check grid vs list visibility
  const gridVisible = await page.locator('.library-grid').isVisible().catch(() => false);
  const listVisible = await page.locator('.library-list').isVisible().catch(() => false);
  console.log('GRID_VISIBLE:', gridVisible, 'LIST_VISIBLE:', listVisible);
  
  // 6. Check card count in grid
  const cardCount = await page.locator('.library-card').count();
  console.log('CARD_COUNT:', cardCount);
  
  // 7. Get card min CSS variable
  const cardMin = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
  console.log('CARD_MIN_CSS:', cardMin);
  
  // 8. Check if density selector actually changes card min
  if (densityCount > 0) {
    const densityOptions = await densityControl.first().locator('option').allTextContents();
    console.log('DENSITY_OPTIONS:', JSON.stringify(densityOptions));
    
    // Switch to compact
    await densityControl.first().selectOption('compact');
    await page.waitForTimeout(300);
    const compactMin = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
    console.log('COMPACT_CARD_MIN:', compactMin);
    
    // Switch to comfortable
    await densityControl.first().selectOption('comfortable');
    await page.waitForTimeout(300);
    const comfortableMin = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
    console.log('COMFORTABLE_CARD_MIN:', comfortableMin);
  }
  
  // 9. Check pagination info
  const pageInfo = await page.evaluate(() => {
    const els = document.querySelectorAll('.library-pagination-info, .pagination-info, [class*="pagination"]');
    return els.length > 0 ? els[0].innerText : 'NOT_FOUND';
  });
  console.log('PAGE_INFO:', pageInfo);
  
  // 10. List all select elements
  const allSelects = await page.locator('select').allTextContents();
  console.log('ALL_SELECTS:', JSON.stringify(allSelects));
  
  await browser.close();
  console.log('DONE');
})();