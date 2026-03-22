const { chromium } = require('playwright');

const baseUrl = 'http://localhost:1420';
const screens = [
  { name: 'home', url: '?screen=home' },
  { name: 'downloads', url: '?screen=downloads' },
  { name: 'library', url: '?screen=library' },
  { name: 'updates', url: '?screen=updates' },
  { name: 'organize', url: '?screen=organize' },
  { name: 'review', url: '?screen=review' },
  { name: 'duplicates', url: '?screen=duplicates' },
  { name: 'creator-audit', url: '?screen=creatorAudit' },
  { name: 'category-audit', url: '?screen=categoryAudit' },
  { name: 'settings', url: '?screen=settings' },
];

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  // Collect console errors
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push({ screen: '', message: msg.text() });
    }
  });
  
  for (const screen of screens) {
    console.log(`Testing ${screen.name}...`);
    consoleErrors[consoleErrors.length - 1]?.screen ?? (consoleErrors[consoleErrors.length - 1] = { ...consoleErrors[consoleErrors.length - 1], screen: screen.name });
    
    await page.goto(baseUrl + screen.url);
    await page.waitForTimeout(2500);
    await page.screenshot({ path: `output/playwright/full-test-${screen.name}.png`, fullPage: true });
    
    console.log(`  ✓ ${screen.name} captured`);
  }
  
  await browser.close();
  
  // Report errors
  const errors = consoleErrors.filter(e => e.message && e.message.length > 0);
  if (errors.length > 0) {
    console.log('\n--- Console Errors ---');
    errors.forEach(e => console.log(`${e.screen}: ${e.message}`));
  } else {
    console.log('\n✓ No console errors detected');
  }
  
  console.log('\n✓ All screenshots captured');
})();