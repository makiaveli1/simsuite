const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  
  await page.goto('http://localhost:1420/#library', { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(3000);
  
  const getState = async () => {
    const metrics = await page.evaluate(() => {
      const el = document.querySelector('.library-toolbar-metrics');
      return el ? el.innerText.replace(/\s+/g, ' ').trim() : 'NOT_FOUND';
    });
    const rows = await page.locator('.library-list-row').count();
    const cards = await page.locator('.library-card').count();
    return { metrics, rows, cards };
  };
  
  // Use nth(1) because getByLabel resolves to [div, select] — we want the <select>
  const pageSizeSelect = page.getByLabel('Items per page').nth(1);
  const selectCount = await pageSizeSelect.count();
  console.log('PAGE_SIZE_SELECT_EXISTS:', selectCount);
  
  if (selectCount === 0) {
    console.log('ERROR: No Items per page select found');
    await browser.close();
    process.exit(1);
  }
  
  // Default (50/page)
  const vals = await pageSizeSelect.locator('option').evaluateAll(els => els.map(e => e.value));
  console.log('OPTIONS:', JSON.stringify(vals));
  console.log('DEFAULT:', JSON.stringify(await getState()));
  
  // Try 50/page - with 67 items, 50 should show 50, rest on page 2
  await pageSizeSelect.selectOption('50');
  await page.waitForTimeout(2000);
  console.log('50/PAGE:', JSON.stringify(await getState()));
  
  // Try 100/page - with 67 items, all 67 should show
  await pageSizeSelect.selectOption('100');
  await page.waitForTimeout(2000);
  console.log('100/PAGE:', JSON.stringify(await getState()));
  
  console.log('ERRORS:', JSON.stringify(errors.slice(0, 3)));
  
  await browser.close();
})();
