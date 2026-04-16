const { chromium } = require('@playwright/test');
(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.setViewportSize({ width: 1440, height: 900 });
  
  // Go to the library screen specifically
  await page.goto('http://localhost:1420/#/library', { timeout: 10000 });
  await page.waitForTimeout(3000);
  await page.screenshot({ path: '/tmp/simsuite-phase5f/library-screen.png', fullPage: true });
  
  const info = await page.evaluate(() => {
    const body = document.body.innerHTML;
    return {
      url: window.location.href,
      hasLibraryGrid: !!document.querySelector('.library-grid'),
      hasLibraryRow: !!document.querySelector('.library-list-row'),
      hasDownloadsGrid: !!document.querySelector('.downloads-grid'),
      bodySnippet: body.substring(0, 500)
    };
  });
  
  console.log('INFO:', JSON.stringify(info, null, 2));
  
  await browser.close();
  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });