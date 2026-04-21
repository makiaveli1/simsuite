/**
 * Phase 5ah — Root Cause Diagnostic
 * Tests what getFolderContents actually returns at each navigation level
 * by injecting debugging into the React component tree
 */
import { chromium } from 'playwright';

const APP_URL = 'http://127.0.0.1:1420/#library';

async function run() {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const logs = [];
  page.on('console', msg => logs.push(`${msg.type()}: ${msg.text()}`));

  await page.goto(APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  await page.locator('[aria-label="Folders view"]').click();
  await page.waitForTimeout(6000); // wait for full data

  console.log('=== Navigation + State Diagnostic ===\n');

  // Step 1: At root level — check what subfolders and rootFiles are
  const root = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const summary = cp?.querySelector('.folder-content-summary')?.textContent;
    const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section')).map(s => s.textContent);
    const folderRows = Array.from(cp?.querySelectorAll('.folder-row')).map(r => r.textContent?.trim());
    const allSections = cp?.querySelectorAll('section').length ?? 0;
    return { summary, sectionLabels, folderRows, allSections };
  });
  console.log('ROOT LEVEL:');
  console.log('  Summary:', root.summary);
  console.log('  Section labels:', root.sectionLabels);
  console.log('  Folder rows:', root.folderRows);
  console.log('  Total <section> count:', root.allSections);

  // Step 2: Click Mods — check what content appears
  console.log('\nCLICKING MODS...');
  await page.locator('.folder-row', { hasText: 'Mods' }).click();
  await page.waitForTimeout(2500);

  const mods = await page.evaluate(() => {
    const cp = document.querySelector('.library-folder-content-pane');
    const header = cp?.querySelector('.folder-content-title span')?.textContent;
    const summary = cp?.querySelector('.folder-content-summary')?.textContent;
    const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section')).map(s => s.textContent);
    const folderRows = Array.from(cp?.querySelectorAll('.folder-row')).map(r => r.textContent?.trim());
    const libRows = cp?.querySelectorAll('.library-list-row').length ?? 0;
    const empty = cp?.querySelector('.library-list-empty')?.textContent;
    return { header, summary, sectionLabels, folderRows, libRows, empty };
  });
  console.log('  Header:', mods.header);
  console.log('  Summary:', mods.summary);
  console.log('  Section labels:', mods.sectionLabels);
  console.log('  Folder rows:', mods.folderRows);
  console.log('  Library list rows:', mods.libRows);
  console.log('  Empty state:', mods.empty ?? '(none)');

  // Step 3: Click Gameplay — check for content (this is Bug 3 — blank/empty)
  const gameplayRow = page.locator('.folder-row', { hasText: 'Gameplay' });
  if (await gameplayRow.count() > 0) {
    console.log('\nCLICKING GAMEPLAY...');
    await gameplayRow.click();
    await page.waitForTimeout(2500);

    const gameplay = await page.evaluate(() => {
      const cp = document.querySelector('.library-folder-content-pane');
      const header = cp?.querySelector('.folder-content-title span')?.textContent;
      const summary = cp?.querySelector('.folder-content-summary')?.textContent;
      const sectionLabels = Array.from(cp?.querySelectorAll('.folder-content-section')).map(s => s.textContent);
      const libRows = cp?.querySelectorAll('.library-list-row').length ?? 0;
      const folderRows = Array.from(cp?.querySelectorAll('.folder-row')).map(r => r.textContent?.trim());
      const looseSection = cp?.querySelectorAll('.folder-loose-source-group').length ?? 0;
      const empty = cp?.querySelector('.library-list-empty')?.textContent;
      return { header, summary, sectionLabels, libRows, folderRows, looseSection, empty };
    });
    console.log('  Header:', gameplay.header);
    console.log('  Summary:', gameplay.summary);
    console.log('  Section labels:', gameplay.sectionLabels);
    console.log('  Library list rows:', gameplay.libRows);
    console.log('  Folder rows:', gameplay.folderRows);
    console.log('  Loose sections:', gameplay.looseSection);
    console.log('  Empty state:', gameplay.empty ?? '(none)');

    if (gameplay.libRows === 0 && gameplay.folderRows.length === 0) {
      console.log('\n  ❌ BUG 3 CONFIRMED: Gameplay content pane is EMPTY');
    } else {
      console.log('\n  ✓ Content pane has data');
    }

    // Step 4: Click a file row — check for blank screen
    if (gameplay.libRows > 0) {
      console.log('\nCLICKING FIRST FILE ROW...');
      const t0 = Date.now();
      await page.locator('.library-list-row').first().click();
      await page.waitForTimeout(3000);
      const t1 = Date.now();

      const blank = await page.evaluate(() => {
        const body = document.body.innerText.trim();
        const detail = document.querySelector('.library-inspector-panel, #library-detail-sheet');
        return { bodyLen: body.length, hasDetail: !!detail, bodyPreview: body.slice(0, 80) };
      });
      console.log(`  Click→render: ${t1-t0}ms`);
      console.log(`  Body len: ${blank.bodyLen}, Detail: ${blank.hasDetail}`);
      if (blank.bodyLen < 80 && !blank.hasDetail) {
        console.log('  ❌ BLANK SCREEN DETECTED');
      } else {
        console.log('  ✓ No blank screen');
      }
    }
  }

  // Show any errors
  const errors = logs.filter(l => l.startsWith('error:'));
  if (errors.length > 0) {
    console.log('\nJS Errors:');
    errors.forEach(e => console.log('  ', e));
  }

  console.log('\n=== DONE ===');
  await browser.close();
}

run().catch(err => { console.error('Fatal:', err.message); process.exit(1); });