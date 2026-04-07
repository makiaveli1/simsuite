import { chromium } from 'playwright';

const pageUrl = 'http://127.0.0.1:3101/#library';
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

await page.goto(pageUrl, { waitUntil: 'networkidle' });
await page.evaluate(() => localStorage.setItem('simsuite:user-view', 'seasoned'));
await page.reload({ waitUntil: 'networkidle' });
const row = page.locator('.library-list-row, .library-table tbody tr').filter({ hasText: 'MCCC_MCCommandCenter.ts4script' }).first();
await row.waitFor({ state: 'visible', timeout: 30000 });
await row.click();

for (const btn of ['Inspect file', 'Warnings & updates', 'Edit details']) {
  await page.getByRole('button', { name: btn, exact: true }).click();
  await page.waitForTimeout(1500);
  const data = await page.evaluate(() => ({
    title: document.querySelector('#library-detail-sheet-title')?.textContent?.trim() ?? null,
    headerTitles: Array.from(document.querySelectorAll('.library-detail-sheet .dock-section-header strong')).map((n) => n.textContent?.trim() ?? ''),
    headerCount: document.querySelectorAll('.library-detail-sheet .dock-section-header strong').length,
    resetVisible: !!Array.from(document.querySelectorAll('.library-detail-sheet button')).find((n) => (n.textContent || '').trim() === 'Reset'),
    inputCount: document.querySelectorAll('.library-detail-sheet input, .library-detail-sheet select, .library-detail-sheet textarea').length,
    pathCardCount: document.querySelectorAll('.library-detail-sheet .path-card').length,
    footerTop: document.querySelector('.library-detail-sheet .workbench-sheet-footer')?.getBoundingClientRect().top ?? null,
    footerBottom: document.querySelector('.library-detail-sheet .workbench-sheet-footer')?.getBoundingClientRect().bottom ?? null,
    viewportHeight: window.innerHeight,
  }));
  console.log(JSON.stringify({ button: btn, ...data }, null, 2));
  await page.getByRole('button', { name: /Close Library detail sheet/i }).click();
  await page.waitForTimeout(250);
}

await browser.close();
