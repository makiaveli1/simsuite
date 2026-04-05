import { Builder, Capabilities, until, By } from "selenium-webdriver";
import fs from "node:fs";
import path from "node:path";

const WEBDRIVER_URL = process.env.SIMSUITE_WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const OUT_DIR = path.resolve("output", "desktop", "viewport-audit");
const APP_CANDIDATES = [
  path.resolve("src-tauri", "target", "debug", "simsuite.exe"),
  path.resolve("src-tauri", "target", "release", "simsuite.exe"),
];
const ROUTES = [
  "home",
  "downloads",
  "library",
  "updates",
  "review",
  "duplicates",
  "organize",
  "settings",
  "staging",
  "creatorAudit",
  "categoryAudit",
];
const VIEWPORTS = [
  { name: "1920x1080", width: 1920, height: 1080 },
  { name: "1600x900", width: 1600, height: 900 },
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1366x768", width: 1366, height: 768 },
  { name: "1280x720", width: 1280, height: 720 },
  { name: "1024x768", width: 1024, height: 768 },
  { name: "900x1400", width: 900, height: 1400 },
];

function resolveAppPath() {
  for (const candidate of APP_CANDIDATES) {
    if (fs.existsSync(candidate)) return candidate;
  }
  throw new Error("Could not find SimSuite desktop binary.");
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

async function waitForApp(driver) {
  await driver.wait(async () => {
    const body = await driver.findElement(By.css("body"));
    const text = await body.getText();
    return /HOME|INBOX|SETTINGS/i.test(text);
  }, 60000);
}

async function setViewport(driver, width, height) {
  try {
    await driver.manage().window().setRect({ width, height, x: 20, y: 20 });
  } catch {
    // ignore if unsupported; measurements will still record actual viewport
  }
}

async function navigateRoute(driver, route) {
  await driver.executeScript(`window.location.hash = '#${route}'`);
  await driver.sleep(900);
}

async function measure(driver) {
  return driver.executeScript(`
    const m = (sel) => {
      const el = document.querySelector(sel);
      if (!el) return null;
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      return {
        selector: sel,
        className: el.className || null,
        top: Number(r.top.toFixed(1)),
        height: Number(r.height.toFixed(1)),
        bottom: Number(r.bottom.toFixed(1)),
        display: s.display,
        overflowY: s.overflowY,
      };
    };
    const pageRoot = document.querySelector('.screen-frame > *');
    const pageRootData = pageRoot ? (() => {
      const r = pageRoot.getBoundingClientRect();
      const s = getComputedStyle(pageRoot);
      return {
        selector: '.screen-frame > *',
        className: String(pageRoot.className || ''),
        top: Number(r.top.toFixed(1)),
        height: Number(r.height.toFixed(1)),
        bottom: Number(r.bottom.toFixed(1)),
        display: s.display,
        overflowY: s.overflowY,
      };
    })() : null;
    const sf = m('.screen-frame');
    return {
      url: location.href,
      route: location.hash.replace(/^#/, ''),
      viewport: { width: window.innerWidth, height: window.innerHeight },
      appShell: m('.app-shell'),
      mainShell: m('.main-shell'),
      toolbar: m('.workspace-toolbar'),
      screenFrame: sf,
      workbench: m('.workbench'),
      workbenchSurface: m('.workbench-surface'),
      pageRoot: pageRootData,
      bottomGap: sf ? Number((window.innerHeight - sf.bottom).toFixed(1)) : null,
      bodyHeight: Number(document.body.getBoundingClientRect().height.toFixed(1)),
      docHeight: document.documentElement.offsetHeight,
    };
  `);
}

async function screenshot(driver, filepath) {
  const png = await driver.takeScreenshot();
  fs.writeFileSync(filepath, Buffer.from(png, 'base64'));
}

async function run() {
  ensureDir(OUT_DIR);
  const appPath = resolveAppPath();
  const capabilities = new Capabilities();
  capabilities.setBrowserName('wry');
  capabilities.set('tauri:options', { application: appPath });

  const driver = await new Builder()
    .usingServer(WEBDRIVER_URL)
    .withCapabilities(capabilities)
    .build();

  const report = { generatedAt: new Date().toISOString(), viewports: [] };

  try {
    await waitForApp(driver);

    for (const vp of VIEWPORTS) {
      await setViewport(driver, vp.width, vp.height);
      await driver.sleep(600);
      const viewportResult = { viewport: vp, actual: null, pages: [] };

      for (const route of ROUTES) {
        await navigateRoute(driver, route);
        const data = await measure(driver);
        if (!viewportResult.actual) viewportResult.actual = data.viewport;
        viewportResult.pages.push(data);

        if (
          (vp.name === '1366x768' && ['home', 'library', 'downloads', 'settings'].includes(route)) ||
          (vp.name === '1920x1080' && ['updates'].includes(route))
        ) {
          await screenshot(driver, path.join(OUT_DIR, `${vp.name}-${route}.png`));
        }
      }

      report.viewports.push(viewportResult);
    }
  } finally {
    await driver.quit();
  }

  const reportPath = path.join(OUT_DIR, 'viewport-report.json');
  fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(reportPath);
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
