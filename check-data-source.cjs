const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  // Capture all responses from the Tauri backend
  const responses = [];
  page.on('response', async (response) => {
    const url = response.url();
    if (url.includes('_tauri') || url.includes('tauri')) {
      try {
        const body = await response.text();
        responses.push({ url: url.substring(0, 80), status: response.status(), body: body.substring(0, 200) });
      } catch (e) {}
    }
  });

  await page.goto('http://localhost:1421/#library', { waitUntil: 'networkidle', timeout: 30000 });
  
  // Wait for data to load
  await page.waitForTimeout(3000);
  
  // Check if rows.total > 7
  const result = await page.evaluate(() => {
    // Look for the LibraryScreen's rows state
    // Try to find the total in the DOM
    const metrics = document.querySelector('.library-toolbar-metrics');
    const metricsText = metrics ? metrics.innerText.replace(/\s+/g, ' ').trim() : 'NOT_FOUND';
    
    // Look for any evidence of total count
    const bodyText = document.body.innerText;
    const match = bodyText.match(/(\d+)\s+IN LIBRARY/);
    const totalMatch = match ? match[1] : 'unknown';
    
    return {
      metricsText,
      totalFromText: totalMatch,
      rowsCount: document.querySelectorAll('.library-list-row').length,
      cardsCount: document.querySelectorAll('.library-card').length
    };
  });
  
  console.log('RESULTS:', JSON.stringify(result, null, 2));
  console.log('\nTAURI RESPONSES:', responses.length);
  responses.forEach(r => {
    console.log(`  [${r.status}] ${r.url}`);
    if (r.body) console.log('   ', r.body.substring(0, 100));
  });
  
  await browser.close();
})();