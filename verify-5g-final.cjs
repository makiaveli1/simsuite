const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  const errors = [];
  page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
  
  await page.goto('http://localhost:1420/#library', { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(3000);
  
  const densitySelect = page.getByLabel('Card density').nth(1);
  const pageSizeSelect = page.getByLabel('Items per page').nth(1);
  
  // Set 50/page  
  await pageSizeSelect.selectOption('50');
  await page.waitForTimeout(1000);
  
  // Switch to grid view FIRST
  await page.locator('.library-view-btn[title="Grid view"]').click();
  await page.waitForTimeout(2000);
  
  console.log('=== DENSITY TEST (50/page, grid view) ===');
  
  const densityOpts = await densitySelect.locator('option').evaluateAll(els => els.map(e => e.value));
  console.log('DENSITY_OPTIONS:', JSON.stringify(densityOpts));
  
  // Test each density
  for (const d of densityOpts) {
    await densitySelect.selectOption(d);
    await page.waitForTimeout(1500);
    
    const cardsPerRow = await page.evaluate(() => {
      const cards = Array.from(document.querySelectorAll('.library-card'));
      if (cards.length === 0) return 0;
      const firstTop = cards[0].getBoundingClientRect().top;
      return cards.filter(c => Math.abs(c.getBoundingClientRect().top - firstTop) < 5).length;
    });
    
    const cardWidth = await page.evaluate(() => {
      const card = document.querySelector('.library-card');
      return card ? card.offsetWidth : 0;
    });
    
    const heroHeight = await page.evaluate(() => {
      const hero = document.querySelector('.library-card-hero');
      return hero ? hero.offsetHeight : 0;
    });
    
    const metrics = await page.evaluate(() => {
      const el = document.querySelector('.library-toolbar-metrics');
      return el ? el.innerText.replace(/\s+/g, ' ').trim() : 'NOT_FOUND';
    });
    
    const cardCount = await page.locator('.library-card').count();
    console.log(`${d.toUpperCase()}: ${cardCount} cards | ${cardsPerRow} per row | ${cardWidth}px wide | hero=${heroHeight}px | ${metrics}`);
  }
  
  // Check NOT TRACKED pill is gone
  const notTrackedPills = await page.evaluate(() => {
    const pills = Array.from(document.querySelectorAll('.library-card .library-health-pill'));
    return pills.filter(el => el.innerText.trim() === 'Not tracked').length;
  });
  console.log('\n=== NOT TRACKED PILLS IN GRID ===');
  console.log('NOT_TRACKED_PILLS:', notTrackedPills, '(should be 0)');
  
  // Check health pills in cards
  const healthPills = await page.evaluate(() => {
    const pills = Array.from(document.querySelectorAll('.library-card .library-health-pill'));
    return pills.map(el => el.innerText.trim()).slice(0, 5);
  });
  console.log('HEALTH_PILLS_SAMPLE:', JSON.stringify(healthPills));
  
  // Switch to list view for row thumbnail check
  await page.locator('.library-view-btn[title="List view"]').click();
  await page.waitForTimeout(2000);
  
  const rowFallbacks = await page.locator('.library-row-thumb-fallback').count();
  const listRows = await page.locator('.library-list-row').count();
  console.log('\n=== ROW VIEW ===');
  console.log('LIST_ROWS:', listRows);
  console.log('ROW_FALLBACKS:', rowFallbacks, '(should match list rows)');
  
  // Check fallback has icon (::before pseudo-element with content)
  const fallbackHasIcon = await page.evaluate(() => {
    const el = document.querySelector('.library-row-thumb-fallback');
    if (!el) return false;
    const style = window.getComputedStyle(el, '::before');
    return style.content !== 'none' && style.content !== '';
  });
  console.log('ROW_FALLBACK_HAS_ICON:', fallbackHasIcon, '(should be true)');
  
  // Test pagination  
  console.log('\n=== PAGINATION TEST ===');
  // Switch to grid with 50/page, should show page 1 of ~2
  await page.locator('.library-view-btn[title="Grid view"]').click();
  await page.getByLabel('Items per page').nth(1).selectOption('50');
  await page.waitForTimeout(1000);
  
  const paginationInfo = await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('button'));
    return btns.map(b => ({ text: b.innerText.trim(), disabled: b.disabled })).filter(b => b.text.length > 0);
  });
  console.log('PAGINATION_BUTTONS:', JSON.stringify(paginationInfo.slice(0, 8)));
  
  console.log('\nERRORS:', JSON.stringify(errors.slice(0, 3)));
  await browser.close();
})();
