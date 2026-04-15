const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  
  const page = await browser.newPage();
  await page.setViewportSize({ width: 1440, height: 900 });
  
  const modes = [
    { name: 'casual', badge: 'Easygoing' },
    { name: 'seasoned', badge: 'Balanced' },
    { name: 'creator', badge: 'Full receipts' },
  ];
  
  try {
    await page.goto('http://localhost:1420', { timeout: 15000 });
    await page.waitForLoadState('networkidle', { timeout: 10000 });
    await page.waitForTimeout(2000);
    
    // Set default to seasoned first to get into the app properly
    await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);
    
    for (const mode of modes) {
      console.log(`\n=== ${mode.name.toUpperCase()} ===`);
      
      // Set mode and reload fresh
      await page.evaluate((m) => {
        localStorage.setItem('simsuite:user-view', m);
        document.documentElement.dataset.userView = m;
      }, mode.name);
      
      await page.reload({ waitUntil: 'networkidle' });
      await page.waitForTimeout(4000);
      
      const badge = await page.locator('.workspace-toolbar-mode-badge').innerText().catch(() => 'N/A');
      const railText = await page.locator('button.rail-nav').allInnerTexts().catch(() => []);
      console.log(`  Badge: "${badge}" | Rail: ${railText.join(',')}`);
      
      // Navigate to library
      await page.goto('http://localhost:1420/#library');
      await page.waitForTimeout(4000);
      
      // Get ALL library-row elements with their full content
      const allRowData = await page.evaluate(() => {
        const rowEls = document.querySelectorAll('[class*="library-list-row"]');
        return Array.from(rowEls).map(row => {
          const title = row.querySelector('[class*="library-row-title"]')?.textContent?.trim() || '';
          const facts = row.querySelector('[class*="library-row-facts"]')?.textContent?.trim() || '';
          const kind = row.querySelector('[class*="library-row-meta"]')?.textContent?.trim() || '';
          const identity = row.querySelector('[class*="library-row-identity"]')?.textContent?.trim() || '';
          return { title, facts, kind, identity };
        });
      });
      
      console.log(`  Rows: ${allRowData.length}`);
      const scriptMods = allRowData.filter(r => 
        r.title.includes('BetterExceptions') || r.title.includes('MCCC') || r.title.includes('Miiko')
      );
      
      for (const row of scriptMods) {
        console.log(`  ---`);
        console.log(`    Title:   ${row.title}`);
        console.log(`    Kind:    ${row.kind}`);
        console.log(`    Identity: ${row.identity}`);
        console.log(`    Facts:   "${row.facts}"`);
      }
      
      // Grid view
      const gridToggle = page.locator('[class*="library-view-toggle"] button').first();
      if (await gridToggle.isVisible().catch(() => false)) {
        await gridToggle.click();
        await page.waitForTimeout(2000);
        
        // Get first ScriptMods card content  
        const cardData = await page.evaluate(() => {
          const cards = document.querySelectorAll('[class*="library-card"]');
          for (const card of cards) {
            const text = card.textContent || '';
            if (text.includes('BetterExceptions') || text.includes('MCCC') || text.includes('Script Mods')) {
              const contentInner = card.querySelector('[class*="library-card-content-inner"]');
              return {
                title: card.querySelector('[class*="library-card-title"]')?.textContent?.trim() || '',
                content: contentInner?.textContent?.trim() || '',
                allText: text.replace(/\s+/g, ' ').trim().substring(0, 200)
              };
            }
          }
          return null;
        });
        
        if (cardData) {
          console.log(`  Grid card content: "${cardData.content}"`);
          console.log(`  Grid card all: ${cardData.allText.substring(0, 150)}`);
        }
        
        await page.screenshot({
          path: `/home/likwid/.openclaw/workspace/simsort-ph4-${mode.name}-grid.png`,
          fullPage: true
        });
      }
    }
    
  } catch (err) {
    console.error('Error:', err.message);
  } finally {
    await browser.close();
  }
})();