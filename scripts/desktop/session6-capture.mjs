import { chromium } from 'playwright';
import { mkdirSync } from 'fs';

mkdirSync('output/library-ui-audit-2026-04-07/session6', { recursive: true });

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

async function openSheet(mode, itemText, buttonName) {
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
}

async function closeSheet() {
  try {
    const sheet = page.locator('.library-detail-sheet');
    if (await sheet.isVisible({ timeout: 1000 })) {
      await page.getByRole('button', { name: /Close Library detail sheet/i }).click();
      await page.waitForTimeout(300);
    }
  } catch { /* not open */ }
}

// Seasoned inspect
await openSheet('seasoned', 'MCCC_MCCommandCenter.ts4script', 'Inspect file');
await page.screenshot({ path: 'output/library-ui-audit-2026-04-07/session6/seasoned-script-inspect-1440x900.png', fullPage: false });
await closeSheet();

// Creator inspect
await openSheet('creator', 'MCCC_MCCommandCenter.ts4script', 'Inspect file');
await page.screenshot({ path: 'output/library-ui-audit-2026-04-07/session6/creator-script-inspect-1440x900.png', fullPage: false });
await closeSheet();

// Seasoned health
await openSheet('seasoned', 'MCCC_MCCommandCenter.ts4script', 'Warnings & updates');
await page.screenshot({ path: 'output/library-ui-audit-2026-04-07/session6/seasoned-script-health-1440x900.png', fullPage: false });
await closeSheet();

// Casual CAS inspector
await openSheet('casual', 'NorthernSiberiaWinds_Skinblend.package', null);
await page.screenshot({ path: 'output/library-ui-audit-2026-04-07/session6/casual-cas-inspector-1440x900.png', fullPage: false });

await browser.close();
console.log('Done');
