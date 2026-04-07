import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

mkdirSync('output/library-ui-audit-2026-04-07/session6', { recursive: true });

const browser = await chromium.launch({ headless: true });

async function capture(page, mode, itemText, buttonName, filename) {
  await page.goto('http://127.0.0.1:3101/#library', { waitUntil: 'networkidle' });
  await page.evaluate((m) => localStorage.setItem('simsuite:user-view', m), mode);
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForTimeout(1000);
  const row = page.locator('.library-list-row, .library-table tbody tr').filter({ hasText: itemText }).first();
  await row.waitFor({ state: 'visible', timeout: 30000 });
  await row.click();
  await page.waitForTimeout(600);
  if (buttonName) {
    await page.getByRole('button', { name: buttonName, exact: true }).click();
    await page.waitForTimeout(800);
  }
  await page.screenshot({ path: `output/library-ui-audit-2026-04-07/session6/${filename}`, fullPage: false });
  await page.getByRole('button', { name: /Close Library detail sheet/i }).click();
  await page.waitForTimeout(250);
}

const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

await capture(page, 'seasoned', 'MCCC_MCCommandCenter.ts4script', 'Inspect file', 'seasoned-script-inspect-1440x900.png');
await capture(page, 'creator', 'MCCC_MCCommandCenter.ts4script', 'Inspect file', 'creator-script-inspect-1440x900.png');
await capture(page, 'seasoned', 'MCCC_MCCommandCenter.ts4script', 'Warnings & updates', 'seasoned-script-health-1440x900.png');
await capture(page, 'casual', 'NorthernSiberiaWinds_Skinblend', null, 'casual-cas-inspect-1440x900.png');
await browser.close();
console.log('Done');
