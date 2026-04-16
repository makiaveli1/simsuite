const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  await page.goto('http://localhost:1421/#library', { waitUntil: 'networkidle', timeout: 30000 });
  
  console.log('=== PHASE 5G VERIFICATION ===');
  
  // 1. Switch to grid view
  const gridBtn = page.locator('.library-view-btn').filter({ hasAttribute: 'aria-pressed' }).first();
  const gridPressed = await gridBtn.getAttribute('aria-pressed');
  console.log('CURRENT_VIEW:', gridPressed === 'true' ? 'grid' : 'list');
  
  if (gridPressed !== 'true') {
    await gridBtn.click();
    await page.waitForTimeout(500);
    console.log('SWITCHED_TO: grid');
  }
  
  // 2. Check hero zone height for fallback cards
  const heroFallback = page.locator('.library-card-hero .library-card-thumbnail-zone--fallback').first();
  if (await heroFallback.count() > 0) {
    const box = await heroFallback.boundingBox();
    console.log('HERO_FALLBACK_HEIGHT:', box?.height);
  } else {
    console.log('HERO_FALLBACK_HEIGHT: no fallback cards found (all may have real thumbnails)');
  }
  
  // 3. Check card density selector changes card-min
  const cardMinBalanced = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
  console.log('CARD_MIN_BALANCED:', cardMinBalanced);
  
  const densitySelect = page.locator('.library-density-select').first();
  if (await densitySelect.count() > 0) {
    await densitySelect.selectOption('compact');
    await page.waitForTimeout(300);
    const cardMinCompact = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
    
    await densitySelect.selectOption('comfortable');
    await page.waitForTimeout(300);
    const cardMinComfortable = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
    
    console.log('CARD_MIN_COMPACT:', cardMinCompact);
    console.log('CARD_MIN_COMFORTABLE:', cardMinComfortable);
    
    // Reset to balanced
    await densitySelect.selectOption('balanced');
  }
  
  // 4. Check items-per-page control
  const pageSizeSelect = page.locator('.library-density-select').nth(1);
  if (await pageSizeSelect.count() > 0) {
    const options = await pageSizeSelect.locator('option').allTextContents();
    console.log('PAGE_SIZE_OPTIONS:', JSON.stringify(options));
    
    // Check the current page count and shown count
    const metrics = await page.evaluate(() => {
      const el = document.querySelector('.library-toolbar-metrics');
      return el ? el.innerText.replace(/\s+/g, ' ').trim() : 'NOT_FOUND';
    });
    console.log('METRICS:', metrics);
  }
  
  // 5. Switch to list view and check row height
  const listBtn = page.locator('.library-view-btn').filter({ has: page.locator('svg') }).last();
  await listBtn.click();
  await page.waitForTimeout(500);
  
  const firstRow = page.locator('.library-list-row').first();
  if (await firstRow.count() > 0) {
    const box = await firstRow.boundingBox();
    console.log('LIST_ROW_HEIGHT:', box?.height);
    
    // Check type-color bar width (first column of grid)
    const typeCol = firstRow.locator('.library-list-col').first();
    const typeBox = await typeCol.boundingBox();
    console.log('TYPE_BAR_WIDTH:', typeBox?.width);
  }
  
  // 6. Check list row selection state
  if (await firstRow.count() > 0) {
    await firstRow.click();
    await page.waitForTimeout(300);
    const isSelected = await firstRow.evaluate(el => el.classList.contains('is-selected'));
    console.log('ROW_SELECTED:', isSelected);
    
    const bgColor = await firstRow.evaluate(el => {
      const style = getComputedStyle(el);
      return style.backgroundColor;
    });
    console.log('ROW_SELECTION_BG:', bgColor);
  }
  
  // 7. Check density selectors exist and are styled
  const densityCount = await page.locator('.library-density-control').count();
  console.log('DENSITY_CONTROLS:', densityCount);
  
  console.log('\n=== SUMMARY ===');
  console.log('Controls working:', densityCount >= 2 ? 'YES' : 'PARTIAL');
  
  await browser.close();
  console.log('DONE');
})();