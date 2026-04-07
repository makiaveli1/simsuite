import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";

const BASE_URL = process.env.SIMSORT_AUDIT_URL ?? "http://127.0.0.1:3101/#library";
const ROOT = "/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort";
const OUTPUT_DIR = path.join(ROOT, "output", "library-ui-audit-2026-04-07");
const STORAGE_KEY = "simsuite:user-view";

const MODES = [
  { id: "casual", label: "Casual" },
  { id: "seasoned", label: "Seasoned" },
  { id: "creator", label: "Creator" },
];

const VIEWPORTS = [
  { key: "1366x768", width: 1366, height: 768 },
  { key: "1440x900", width: 1440, height: 900 },
  { key: "1920x1080", width: 1920, height: 1080 },
  { key: "2560x1440", width: 2560, height: 1440 },
];

const FILES = [
  {
    key: "cas",
    filename: "NorthernSiberiaWinds_Skinblend.package",
    expectedKind: "CAS",
    actions: ["inspect"],
  },
  {
    key: "script",
    filename: "MCCC_MCCommandCenter.ts4script",
    expectedKind: "Script Mods",
    actions: ["inspect", "health", "edit"],
  },
  {
    key: "tray",
    filename: "OakHousehold_0x00ABCDEF.trayitem",
    expectedKind: "Tray Household",
    actions: ["inspect"],
  },
];

const ACTION_BUTTONS = {
  inspect: ["Inspect file", "More details"],
  health: ["Warnings & updates"],
  edit: ["Edit details"],
};

const SHEET_TITLES = {
  inspect: [
    "File facts and deeper clues",
    "Embedded names, version clues, and file facts",
    "Embedded identity, clues, and full path",
  ],
  health: [
    "Warnings, updates, and bundle notes",
    "Warnings, updates, and bundle context",
    "Diagnostics, watch evidence, and bundle context",
  ],
  edit: [
    "Fix the saved details here",
    "Fix creator and type details without moving the file",
    "Edit creator learning and type overrides",
  ],
};

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function ensureDir(dir) {
  await fs.mkdir(dir, { recursive: true });
}

async function clickSidebarLibrary(page) {
  const libraryButton = page.getByRole("button", { name: /^library$/i });
  if ((await libraryButton.count()) > 0) {
    await libraryButton.first().click();
    await sleep(300);
  }
  if (!page.url().includes("#library")) {
    await page.goto(BASE_URL, { waitUntil: "networkidle" });
  }
}

async function setMode(page, mode) {
  await page.goto(BASE_URL, { waitUntil: "networkidle" });
  await page.evaluate((payload) => {
    globalThis.localStorage?.setItem(payload.key, payload.value);
  }, { key: STORAGE_KEY, value: mode });
  await page.reload({ waitUntil: "networkidle" });
  await clickSidebarLibrary(page);
  await page.waitForLoadState("networkidle");
  await sleep(500);
}

async function clearSearch(page) {
  const search = page.locator('.library-toolbar-search input');
  if ((await search.count()) === 0) return;
  await search.fill("");
  await sleep(250);
}

async function selectFile(page, filename) {
  await clearSearch(page);
  const search = page.locator('.library-toolbar-search input');
  if ((await search.count()) > 0) {
    await search.fill(filename);
    await sleep(500);
  }

  const row = page
    .locator('.library-list-row, .library-table tbody tr')
    .filter({ hasText: filename })
    .first();

  await row.waitFor({ state: "visible", timeout: 10000 });
  await row.click();
  const detailTitle = page.locator('.detail-filename');
  await detailTitle.waitFor({ state: "visible", timeout: 10000 });
  await page.waitForFunction(
    (expected) => {
      const el = document.querySelector('.detail-filename');
      return !!el && (el.textContent || '').includes(expected);
    },
    filename,
    { timeout: 10000 },
  );
  await sleep(250);
}

async function inspectorState(page) {
  return await page.evaluate(() => {
    const inspector = document.querySelector('.library-details-panel');
    const actions = Array.from(document.querySelectorAll('.library-details-actions button')).map((node) => (node.textContent || '').replace(/\s+/g, ' ').trim());
    const detailHeader = document.querySelector('.detail-header');
    const watchPill = document.querySelector('.library-health-pill');
    const footer = document.querySelector('.workbench-sheet-footer');
    const actionGrid = document.querySelector('.library-details-actions-grid');
    const inspectorRect = inspector?.getBoundingClientRect();
    const actionGridRect = actionGrid?.getBoundingClientRect();
    return {
      actions,
      inspectorHeight: inspectorRect ? Math.round(inspectorRect.height) : null,
      headerHeight: detailHeader ? Math.round(detailHeader.getBoundingClientRect().height) : null,
      watchLabel: watchPill ? (watchPill.textContent || '').replace(/\s+/g, ' ').trim() : null,
      hasHorizontalOverflow: inspector ? inspector.scrollWidth > inspector.clientWidth : false,
      actionGridWrapped: actionGrid ? actionGrid.scrollHeight > actionGrid.clientHeight + 2 : false,
      actionGridWidth: actionGridRect ? Math.round(actionGridRect.width) : null,
      footerVisible: footer ? (() => {
        const rect = footer.getBoundingClientRect();
        return rect.bottom <= window.innerHeight && rect.top >= 0;
      })() : null,
    };
  });
}

async function openSheet(page, actionKey) {
  const names = ACTION_BUTTONS[actionKey];
  for (const name of names) {
    const button = page.getByRole('button', { name, exact: true });
    if ((await button.count()) > 0) {
      await button.first().click();
      const sheet = page.locator('.library-detail-sheet');
      await sheet.waitFor({ state: 'visible', timeout: 10000 });
      await sleep(300);
      return name;
    }
  }
  return null;
}

async function closeSheet(page) {
  const closeButton = page.getByRole('button', { name: /close library detail sheet/i });
  if ((await closeButton.count()) > 0) {
    await closeButton.first().click();
  } else {
    const doneButton = page.getByRole('button', { name: /^done$/i });
    if ((await doneButton.count()) > 0) {
      await doneButton.first().click();
    }
  }
  await page.waitForFunction(() => !document.querySelector('.library-detail-sheet'), null, { timeout: 10000 });
  await sleep(250);
}

async function sheetState(page) {
  return await page.evaluate(() => {
    const sheet = document.querySelector('.library-detail-sheet');
    const body = document.querySelector('.library-detail-sheet-body');
    const footer = document.querySelector('.workbench-sheet-footer');
    const title = document.querySelector('#library-detail-sheet-title');
    const copy = document.querySelector('.workbench-sheet-copy');
    const lead = document.querySelector('.library-detail-sheet-lead');
    const bodyRect = body?.getBoundingClientRect();
    const footerRect = footer?.getBoundingClientRect();
    return {
      title: title ? (title.textContent || '').trim() : null,
      copy: copy ? (copy.textContent || '').replace(/\s+/g, ' ').trim() : null,
      leadHeight: lead ? Math.round(lead.getBoundingClientRect().height) : null,
      bodyHeight: bodyRect ? Math.round(bodyRect.height) : null,
      bodyScrollHeight: body ? body.scrollHeight : null,
      bodyScrollable: body ? body.scrollHeight > body.clientHeight + 2 : false,
      bodyHorizontalOverflow: body ? body.scrollWidth > body.clientWidth : false,
      footerVisible: footer ? footerRect.bottom <= window.innerHeight && footerRect.top >= 0 : false,
      footerBottomGap: footerRect ? Math.round(window.innerHeight - footerRect.bottom) : null,
      shellWidth: sheet ? Math.round(sheet.getBoundingClientRect().width) : null,
      shellHeight: sheet ? Math.round(sheet.getBoundingClientRect().height) : null,
      sectionLabels: Array.from(document.querySelectorAll('.library-detail-sheet .section-label')).map((node) => (node.textContent || '').trim()).filter(Boolean),
      buttonLabels: Array.from(document.querySelectorAll('.library-detail-sheet button')).map((node) => (node.textContent || '').replace(/\s+/g, ' ').trim()).filter(Boolean),
    };
  });
}

async function captureAudit() {
  await ensureDir(OUTPUT_DIR);
  const browser = await chromium.launch({ headless: true });
  const results = {
    baseUrl: BASE_URL,
    capturedAt: new Date().toISOString(),
    modes: {},
  };

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const page = await context.newPage();

  for (const mode of MODES) {
    results.modes[mode.id] = { representative: {}, responsive: {} };
    await setMode(page, mode.id);

    for (const file of FILES) {
      await page.setViewportSize({ width: 1440, height: 900 });
      await setMode(page, mode.id);
      await selectFile(page, file.filename);
      const modeDir = path.join(OUTPUT_DIR, mode.id);
      await ensureDir(modeDir);

      const inspectorPath = path.join(modeDir, `${file.key}-inspector-1440x900.png`);
      await page.locator('.library-details-panel').screenshot({ path: inspectorPath });
      const fullPath = path.join(modeDir, `${file.key}-library-1440x900.png`);
      await page.screenshot({ path: fullPath, fullPage: false });

      const inspector = await inspectorState(page);
      const fileResult = {
        filename: file.filename,
        expectedKind: file.expectedKind,
        inspector,
        screenshots: { inspector: inspectorPath, full: fullPath },
        sheets: {},
      };

      for (const actionKey of file.actions) {
        const clicked = await openSheet(page, actionKey);
        if (!clicked) {
          fileResult.sheets[actionKey] = { missingAction: true };
          continue;
        }
        const sheetPath = path.join(modeDir, `${file.key}-${actionKey}-1440x900.png`);
        await page.locator('.library-detail-sheet').screenshot({ path: sheetPath });
        fileResult.sheets[actionKey] = {
          buttonUsed: clicked,
          screenshot: sheetPath,
          metrics: await sheetState(page),
        };
        await closeSheet(page);
      }

      results.modes[mode.id].representative[file.key] = fileResult;
    }

    for (const viewport of VIEWPORTS) {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await setMode(page, mode.id);
      await selectFile(page, FILES[1].filename);
      const responsiveDir = path.join(OUTPUT_DIR, mode.id, 'responsive');
      await ensureDir(responsiveDir);
      const inspectorPath = path.join(responsiveDir, `script-inspector-${viewport.key}.png`);
      await page.locator('.library-details-panel').screenshot({ path: inspectorPath });
      const inspectorMetrics = await inspectorState(page);
      const responsiveResult = {
        viewport,
        inspector: {
          screenshot: inspectorPath,
          metrics: inspectorMetrics,
        },
        sheets: {},
      };

      const availableActions = mode.id === 'casual' ? ['inspect'] : ['inspect', 'health', 'edit'];
      for (const actionKey of availableActions) {
        const clicked = await openSheet(page, actionKey);
        if (!clicked) {
          responsiveResult.sheets[actionKey] = { missingAction: true };
          continue;
        }
        const sheetPath = path.join(responsiveDir, `script-${actionKey}-${viewport.key}.png`);
        await page.locator('.library-detail-sheet').screenshot({ path: sheetPath });
        responsiveResult.sheets[actionKey] = {
          buttonUsed: clicked,
          screenshot: sheetPath,
          metrics: await sheetState(page),
        };
        await closeSheet(page);
      }

      results.modes[mode.id].responsive[viewport.key] = responsiveResult;
    }
  }

  const jsonPath = path.join(OUTPUT_DIR, 'audit-results.json');
  await fs.writeFile(jsonPath, JSON.stringify(results, null, 2));
  await browser.close();
  console.log(JSON.stringify({ ok: true, outputDir: OUTPUT_DIR, jsonPath }, null, 2));
}

captureAudit().catch((error) => {
  console.error(error);
  process.exit(1);
});
