const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  await page.goto('http://localhost:1421/#library', { waitUntil: 'networkidle', timeout: 30000 });
  
  const url = page.url();
  console.log('URL:', url);
  
  // Check what's visible
  const bodyText = await page.evaluate(() => document.body.innerText.slice(0, 400));
  console.log('BODY_TEXT:', bodyText.replace(/\n+/g, ' ').slice(0, 300));
  
  // Check selects
  const allSelects = await page.locator('select').allTextContents();
  console.log('ALL_SELECTS:', JSON.stringify(allSelects));
  
  // Check metrics text
  const metrics = await page.evaluate(() => {
    const el = document.querySelector('.library-toolbar-metrics');
    return el ? el.innerText : 'NOT_FOUND';
  });
  console.log('METRICS:', metrics);
  
  // Check if grid or list is visible
  const gridExists = await page.locator('.library-grid').count();
  const listExists = await page.locator('.library-list').count();
  console.log('GRID_EXISTS:', gridExists, 'LIST_EXISTS:', listExists);
  
  // Check cards
  const cards = await page.locator('.library-card').count();
  console.log('CARDS:', cards);
  
  // Check rows
  const rows = await page.locator('.library-list-row').count();
  console.log('LIST_ROWS:', rows);
  
  // Check CSS variable
  const cardMin = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--card-min').trim());
  console.log('CARD_MIN:', cardMin);
  
  await browser.close();
  console.log('DONE');
})();