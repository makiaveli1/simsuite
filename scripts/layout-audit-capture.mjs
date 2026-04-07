import { chromium } from 'playwright';
import fs from 'node:fs';
import path from 'node:path';

const BASE_URL = process.env.AUDIT_BASE_URL || 'http://127.0.0.1:1501';
const OUT_DIR = path.resolve('output/layout-audit');
const viewports = [
  { name: '1366x768', width: 1366, height: 768 },
  { name: '1280x720', width: 1280, height: 720 },
  { name: '1600x900', width: 1600, height: 900 },
  { name: '1920x1080', width: 1920, height: 1080 },
  { name: '2560x1440', width: 2560, height: 1440 },
  { name: '3840x2160', width: 3840, height: 2160 },
  { name: '900x1400', width: 900, height: 1400 },
];
const routes = [
  { name: 'home', hash: '#home' },
  { name: 'library', hash: '#library' },
  { name: 'updates', hash: '#updates' },
  { name: 'settings', hash: '#settings' },
  { name: 'downloads', hash: '#downloads' },
];

fs.mkdirSync(OUT_DIR, { recursive: true });

const browser = await chromium.launch({ headless: true });
const results = [];

for (const viewport of viewports) {
  const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height } });
  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.error(`[console:${viewport.name}]`, msg.text());
    }
  });

  for (const route of routes) {
    const url = `${BASE_URL}/${route.hash}`;
    await page.goto(url, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(1500);

    const metrics = await page.evaluate(() => {
      const qs = sel => document.querySelector(sel);
      const rect = el => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height, bottom: r.bottom };
      };
      return {
        title: document.title,
        route: location.hash,
        rootLen: document.getElementById('root')?.innerHTML.length ?? 0,
        appShell: rect(qs('.app-shell')),
        mainShell: rect(qs('.main-shell')),
        screenFrame: rect(qs('.screen-frame')),
        screenShell: rect(qs('.screen-shell')),
        workbench: rect(qs('.workbench')),
        homeHubShell: rect(qs('.home-hub-shell')),
        homeHero: rect(qs('.home-hero')),
        homeModuleStack: rect(qs('.home-module-stack')),
        settingsLayout: rect(qs('.settings-layout')),
        settingsFocusPanel: rect(qs('.settings-focus-panel')),
        libraryStageShell: rect(qs('.library-stage-shell')),
        libraryListShell: rect(qs('.library-list-shell')),
        libraryDetailPanel: rect(qs('.library-details-panel')),
        updatesStage: rect(qs('.updates-stage')),
        updatesTableScroll: rect(qs('.updates-table-scroll')),
        updatesStageFooter: rect(qs('.updates-stage-footer')),
        downloadsShell: rect(qs('.downloads-shell')),
        downloadsQueuePanel: rect(qs('.downloads-queue-panel')),
        downloadsPreviewPanel: rect(qs('.downloads-preview-panel')),
        bodyHeight: document.body.scrollHeight,
        viewportHeight: window.innerHeight,
      };
    });

    const file = path.join(OUT_DIR, `${viewport.name}-${route.name}.png`);
    await page.screenshot({ path: file, fullPage: false });
    results.push({ viewport, route, file, metrics });
    console.log(`captured ${viewport.name} ${route.name}`);
  }

  await page.close();
}

await browser.close();
fs.writeFileSync(path.join(OUT_DIR, 'results.json'), JSON.stringify(results, null, 2));
console.log(`saved ${results.length} captures to ${OUT_DIR}`);
