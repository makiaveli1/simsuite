const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  await page.goto('http://localhost:1421/#library', { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(3000);
  
  const metrics = await page.evaluate(() => {
    const el = document.querySelector('.library-toolbar-metrics');
    return el ? el.innerText.replace(/\s+/g, ' ').trim() : 'NOT_FOUND';
  });
  
  console.log('METRICS:', metrics);
  
  const rowCount = await page.locator('.library-list-row').count();
  console.log('LIST_ROWS:', rowCount);
  
  await browser.close();
})();
