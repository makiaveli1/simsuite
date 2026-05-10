import { Builder, By, Capabilities, until } from "selenium-webdriver";
import fs from "node:fs";
import path from "node:path";

const WEBDRIVER_URL = process.env.SIMSUITE_WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const SESSION_FILE = process.env.SIMSUITE_TAURI_DRIVER_SESSION_FILE
  ? path.resolve(process.env.SIMSUITE_TAURI_DRIVER_SESSION_FILE)
  : path.resolve("output", "desktop", "tauri-driver-session.json");
const OUTPUT_ROOT = process.env.SIMSUITE_DESKTOP_PROOF_OUTPUT
  ? path.resolve(process.env.SIMSUITE_DESKTOP_PROOF_OUTPUT)
  : path.resolve("output", "desktop", "library-proof");
const APP_PATHS = [
  path.resolve("src-tauri", "target", "release", "simsuite.exe"),
  path.resolve("src-tauri", "target", "release", "SimSuite.exe"),
  path.resolve("src-tauri", "target", "debug", "simsuite.exe"),
  path.resolve("src-tauri", "target", "debug", "SimSuite.exe"),
];

function resolveAppPath() {
  const explicit = process.env.SIMSUITE_TAURI_APP_PATH;
  if (explicit && fs.existsSync(explicit)) {
    return explicit;
  }

  for (const candidate of APP_PATHS) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error("SimSuite could not find the Tauri app binary. Build debug or release first.");
}

function loadSession() {
  const raw = fs.readFileSync(SESSION_FILE, "utf8").replace(/^\uFEFF/, "");
  return JSON.parse(raw);
}

function timestampLabel(date = new Date()) {
  return date.toISOString().replace(/[:.]/g, "-");
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function xpathString(value) {
  if (!value.includes("'")) {
    return `'${value}'`;
  }
  if (!value.includes('"')) {
    return `"${value}"`;
  }
  return `concat(${value.split("'").map((part) => `'${part}'`).join(`, "'", `)})`;
}

async function sleep(driver, ms) {
  await driver.sleep(ms);
}

async function getBodyText(driver) {
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      return await driver.findElement(By.css("body")).getText();
    } catch {
      await sleep(driver, 150);
    }
  }
  return await driver.findElement(By.css("body")).getText();
}

async function getHash(driver) {
  return await driver.executeScript("return window.location.hash;");
}

async function waitForAnyText(driver, texts, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const body = (await getBodyText(driver)).toLowerCase();
    const hit = texts.find((text) => body.includes(text.toLowerCase()));
    if (hit) {
      return hit;
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for any of: ${texts.join(", ")}`);
}

async function waitForHash(driver, expectedHash, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const hash = await getHash(driver);
    if (hash === expectedHash) {
      return hash;
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for hash ${expectedHash}`);
}

async function navigateToScreen(driver, screen, timeoutMs = 30000) {
  const expectedHash = `#${screen}`;
  await driver.executeScript(
    `
      if (window.location.hash !== arguments[0]) {
        window.location.hash = arguments[0];
      }
      window.dispatchEvent(new PopStateEvent("popstate"));
    `,
    expectedHash,
  );
  await waitForHash(driver, expectedHash, timeoutMs).catch(() => null);
}

async function clickVisibleButton(driver, partialText, timeoutMs = 30000) {
  const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const buttons = await driver.findElements(locator);
    for (const button of buttons) {
      try {
        if ((await button.isDisplayed()) && (await button.isEnabled())) {
          try {
            await button.click();
          } catch {
            await driver.executeScript("arguments[0].click()", button);
          }
          return;
        }
      } catch {
        // Element went stale during a route transition; retry the locator.
      }
    }
    await sleep(driver, 250);
  }

  throw new Error(`Could not find an enabled button containing \"${partialText}\".`);
}

async function clickVisibleElement(driver, xpath, timeoutMs = 30000) {
  const locator = By.xpath(xpath);
  await driver.wait(until.elementLocated(locator), timeoutMs);

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const elements = await driver.findElements(locator);
    for (const element of elements) {
      try {
        if (await element.isDisplayed()) {
          await driver.executeScript(
            "arguments[0].scrollIntoView({ block: 'center', inline: 'nearest' })",
            element,
          );
          try {
            await element.click();
          } catch {
            await driver.executeScript("arguments[0].click()", element);
          }
          return;
        }
      } catch {
        // Element went stale during a route transition; retry the locator.
      }
    }
    await sleep(driver, 250);
  }

  throw new Error(`Could not click a visible element for xpath: ${xpath}`);
}

async function waitForVisibleCssText(driver, selector, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const elements = await driver.findElements(By.css(selector));
    for (const element of elements) {
      try {
        if (await element.isDisplayed()) {
          return await element.getText();
        }
      } catch {
        // Element went stale during a route transition; retry the selector.
      }
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for visible selector ${selector}`);
}

async function waitForVisibleElement(driver, selector, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const elements = await driver.findElements(By.css(selector));
    for (const element of elements) {
      try {
        if (await element.isDisplayed()) {
          return element;
        }
      } catch {
        // Element went stale during a route transition; retry the selector.
      }
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for visible selector ${selector}`);
}

async function clickVisibleButtonByAriaLabel(driver, ariaLabel, timeoutMs = 30000) {
  const locator = By.xpath(`//button[@aria-label = ${xpathString(ariaLabel)}]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const buttons = await driver.findElements(locator);
    for (const button of buttons) {
      try {
        if ((await button.isDisplayed()) && (await button.isEnabled())) {
          try {
            await button.click();
          } catch {
            await driver.executeScript("arguments[0].click()", button);
          }
          return;
        }
      } catch {
        // Element went stale during a route transition; retry the locator.
      }
    }
    await sleep(driver, 250);
  }

  throw new Error(`Could not find an enabled button with aria-label "${ariaLabel}".`);
}

async function clickAnyVisibleButton(driver, partialTexts, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    for (const partialText of partialTexts) {
      const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
      const buttons = await driver.findElements(locator);
      for (const button of buttons) {
        try {
          if ((await button.isDisplayed()) && (await button.isEnabled())) {
            try {
              await button.click();
            } catch {
              await driver.executeScript("arguments[0].click()", button);
            }
            return partialText;
          }
        } catch {
          // Element went stale during a route transition; retry the locator.
        }
      }
    }
    await sleep(driver, 250);
  }

  throw new Error(`Could not find an enabled button containing any of: ${partialTexts.join(", ")}.`);
}

async function openLibraryScreen(driver) {
  await navigateToScreen(driver, "library", 30000);
  await waitForVisibleElement(driver, ".library-top-strip", 30000);
}

async function takeScreenshot(driver, outputPath) {
  const base64 = await driver.takeScreenshot();
  fs.writeFileSync(outputPath, Buffer.from(base64, "base64"));
}

async function invokeTauri(driver, command, payload = {}) {
  return await driver.executeAsyncScript(
    async (cmd, args, done) => {
      try {
        const response = await window.__TAURI_INTERNALS__.invoke(cmd, args);
        done({ ok: true, response });
      } catch (error) {
        done({ ok: false, error: String(error?.message ?? error) });
      }
    },
    command,
    payload,
  );
}

async function installRuntimeErrorCapture(driver) {
  await driver.executeScript(() => {
    if (window.__SIMSUITE_LIBRARY_PROOF_ERROR_CAPTURED__) {
      return;
    }

    window.__SIMSUITE_LIBRARY_PROOF_ERROR_CAPTURED__ = true;
    window.__SIMSUITE_LIBRARY_PROOF_ERRORS__ = [];

    const serialize = (value) => {
      try {
        if (value instanceof Error) {
          return value.stack || value.message;
        }
        if (typeof value === "object" && value !== null) {
          return JSON.stringify(value);
        }
        return String(value);
      } catch {
        return Object.prototype.toString.call(value);
      }
    };

    const record = (kind, values) => {
      window.__SIMSUITE_LIBRARY_PROOF_ERRORS__.push({
        kind,
        message: values.map(serialize).join(" "),
        at: new Date().toISOString(),
      });
    };

    const originalConsoleError = console.error.bind(console);
    console.error = (...args) => {
      record("console.error", args);
      originalConsoleError(...args);
    };

    window.addEventListener("error", (event) => {
      record("error", [
        event.message,
        event.filename,
        event.lineno,
        event.colno,
        event.error,
      ]);
    });

    window.addEventListener("unhandledrejection", (event) => {
      record("unhandledrejection", [event.reason]);
    });
  });
}

async function readRuntimeErrors(driver) {
  return await driver.executeScript(
    "return Array.isArray(window.__SIMSUITE_LIBRARY_PROOF_ERRORS__) ? window.__SIMSUITE_LIBRARY_PROOF_ERRORS__ : [];",
  );
}

async function readBrowserLogEntries(driver) {
  try {
    const entries = await driver.manage().logs().get("browser");
    return {
      supported: true,
      entries: entries.map((entry) => ({
        level: String(entry.level?.name ?? entry.level ?? ""),
        message: String(entry.message ?? ""),
        timestamp: entry.timestamp ?? null,
      })),
    };
  } catch (error) {
    return {
      supported: false,
      error: error instanceof Error ? error.message : String(error),
      entries: [],
    };
  }
}

async function assertNoRuntimeErrors(driver, summary, label) {
  const errors = await readRuntimeErrors(driver);
  if (!Array.isArray(summary.runtimeErrorChecks)) {
    summary.runtimeErrorChecks = [];
  }
  summary.runtimeErrorChecks.push({
    label,
    count: errors.length,
    errors,
  });
  if (errors.length > 0) {
    throw new Error(`${label} captured ${errors.length} runtime error(s).`);
  }
}

async function collectLibraryGeometry(driver) {
  return await driver.executeScript(() => {
    const rectFor = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      return {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      };
    };

    const doc = document.documentElement;
    const rowClipFailures = Array.from(document.querySelectorAll(".library-list-body .library-list-row"))
      .slice(0, 10)
      .flatMap((row, rowIndex) => {
        const rowRect = row.getBoundingClientRect();
        return Array.from(
          row.querySelectorAll(
            ".library-row-title, .library-row-meta, .library-row-identity, .library-health-pill, .library-row-fact, .library-row-swatches",
          ),
        )
          .map((child) => {
            const childRect = child.getBoundingClientRect();
            if (childRect.width <= 0 || childRect.height <= 0) return null;
            const bottomOverflow = childRect.bottom - rowRect.bottom;
            const topOverflow = rowRect.top - childRect.top;
            if (bottomOverflow > 3 || topOverflow > 3) {
              return {
                rowIndex,
                className: child.className,
                topOverflow,
                bottomOverflow,
                row: {
                  top: rowRect.top,
                  bottom: rowRect.bottom,
                  height: rowRect.height,
                },
                child: {
                  top: childRect.top,
                  bottom: childRect.bottom,
                  height: childRect.height,
                },
              };
            }
            return null;
          })
          .filter(Boolean);
      });

    return {
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight,
        clientWidth: doc.clientWidth,
        scrollWidth: doc.scrollWidth,
      },
      workbench: rectFor(".library-workbench"),
      stage: rectFor(".library-stage-shell"),
      inspector: rectFor(".library-inspector-shell"),
      inspectorExpanded: rectFor(".library-inspector-shell:not(.inspector-collapsed)"),
      topStrip: rectFor(".library-top-strip"),
      browseRow: rectFor(".library-browse-row"),
      listShell: rectFor(".library-list-shell"),
      listHeader: rectFor(".library-list-header"),
      listBody: rectFor(".library-list-body"),
      footer: rectFor(".library-stage-shell > .table-footer"),
      activeNav: rectFor(".rail-nav.is-active"),
      activeNavText: rectFor(".rail-nav.is-active span"),
      rowClipFailures,
    };
  });
}

function assertLibraryGeometry(geometry) {
  const tolerance = 4;
  const failures = [];
  const overflow = geometry.viewport.scrollWidth - geometry.viewport.clientWidth;

  if (overflow > tolerance) {
    failures.push(`document has ${overflow}px horizontal overflow`);
  }

  if (geometry.stage && geometry.inspectorExpanded && geometry.stage.right > geometry.inspectorExpanded.left + tolerance) {
    failures.push("Library stage overlaps the inspector column");
  }

  if (geometry.listShell && geometry.inspectorExpanded && geometry.listShell.right > geometry.inspectorExpanded.left + tolerance) {
    failures.push("Library list viewport extends under the inspector");
  }

  if (geometry.topStrip && geometry.stage && geometry.topStrip.right > geometry.stage.right + tolerance) {
    failures.push("Library toolbar extends outside the stage column");
  }

  if (geometry.browseRow && geometry.listHeader && geometry.browseRow.bottom > geometry.listHeader.top + tolerance) {
    failures.push("Library filter chips overlap the list header");
  }

  if (geometry.listShell && geometry.footer && geometry.listShell.bottom > geometry.footer.top + tolerance) {
    failures.push("Library list viewport overlaps the pagination footer");
  }

  if (
    geometry.activeNav &&
    geometry.activeNavText &&
    (geometry.activeNavText.left < geometry.activeNav.left - tolerance ||
      geometry.activeNavText.right > geometry.activeNav.right + tolerance ||
      geometry.activeNavText.bottom > geometry.activeNav.bottom + tolerance)
  ) {
    failures.push("Active sidebar label is clipped outside its nav item");
  }

  if (geometry.rowClipFailures?.length > 0) {
    failures.push(`Library row content is clipped in ${geometry.rowClipFailures.length} visible element(s)`);
  }

  return {
    ok: failures.length === 0,
    failures,
  };
}

async function assertLibraryLayoutGeometry(driver, summary, label) {
  const geometry = await collectLibraryGeometry(driver);
  const result = assertLibraryGeometry(geometry);
  if (!Array.isArray(summary.layoutGeometryChecks)) {
    summary.layoutGeometryChecks = [];
  }
  summary.layoutGeometryChecks.push({
    label,
    ...result,
    geometry,
  });
  if (!result.ok) {
    throw new Error(`${label} layout geometry failed: ${result.failures.join("; ")}`);
  }
}

async function getLibraryInspectorWidth(driver) {
  return await driver.executeScript(() => {
    const inspector = document.querySelector(".library-inspector-shell");
    return inspector ? inspector.getBoundingClientRect().width : null;
  });
}

async function dragLibraryInspectorHandle(driver, deltaX) {
  const handles = await driver.findElements(By.css(".library-inspector-shell .resize-handle-left"));
  if (handles.length === 0) {
    return { ok: false, reason: "resize handle not found" };
  }

  try {
    await driver
      .actions({ async: true })
      .move({ origin: handles[0], x: 6, y: 24 })
      .press()
      .move({ origin: "pointer", x: deltaX, y: 0, duration: 280 })
      .release()
      .perform();
    return { ok: true, method: "webdriver-actions" };
  } catch (error) {
    const fallback = await driver.executeScript((moveBy) => {
    const handle = document.querySelector(".library-inspector-shell .resize-handle-left");
    const inspector = document.querySelector(".library-inspector-shell");
    if (!handle || !inspector) {
      return { ok: false, reason: "resize handle or inspector not found" };
    }

    const rect = handle.getBoundingClientRect();
    const startX = rect.left + rect.width / 2;
    const startY = rect.top + rect.height / 2;
    const pointerId = 11;
    const eventBase = {
      bubbles: true,
      cancelable: true,
      pointerId,
      pointerType: "mouse",
      isPrimary: true,
      button: 0,
      buttons: 1,
    };

    handle.dispatchEvent(new PointerEvent("pointerdown", {
      ...eventBase,
      clientX: startX,
      clientY: startY,
    }));
    window.dispatchEvent(new PointerEvent("pointermove", {
      ...eventBase,
      clientX: startX + moveBy,
      clientY: startY,
    }));
    window.dispatchEvent(new PointerEvent("pointerup", {
      ...eventBase,
      buttons: 0,
      clientX: startX + moveBy,
      clientY: startY,
    }));

    return {
      ok: true,
      cssWidth: getComputedStyle(document.documentElement).getPropertyValue("--library-detail-width"),
    };
    }, deltaX);
    return {
      ...fallback,
      method: "synthetic-pointer-fallback",
      actionError: error instanceof Error ? error.message : String(error),
    };
  }
}

async function verifyLibraryInspectorAdjustability(driver, summary, runDir) {
  await waitForVisibleElement(driver, ".library-inspector-shell:not(.inspector-collapsed)", 30000);
  let initialWidth = await getLibraryInspectorWidth(driver);
  if (initialWidth !== null && initialWidth > 360) {
    await dragLibraryInspectorHandle(driver, 96);
    await sleep(driver, 450);
    initialWidth = await getLibraryInspectorWidth(driver);
  }
  const widerDrag = await dragLibraryInspectorHandle(driver, -96);
  await sleep(driver, 450);
  const widerWidth = await getLibraryInspectorWidth(driver);
  if (!widerDrag.ok || widerWidth === null || initialWidth === null || widerWidth <= initialWidth + 20) {
    throw new Error(`Inspector resize wider failed: ${JSON.stringify({ initialWidth, widerWidth, widerDrag })}`);
  }
  const widerShot = path.join(runDir, "08-library-inspector-wider.png");
  await takeScreenshot(driver, widerShot);
  summary.screenshots.push(widerShot);

  const narrowerDrag = await dragLibraryInspectorHandle(driver, 84);
  await sleep(driver, 450);
  const narrowerWidth = await getLibraryInspectorWidth(driver);
  if (!narrowerDrag.ok || narrowerWidth === null || narrowerWidth >= widerWidth - 20) {
    throw new Error(`Inspector resize narrower failed: ${JSON.stringify({ widerWidth, narrowerWidth, narrowerDrag })}`);
  }
  const narrowerShot = path.join(runDir, "09-library-inspector-narrower.png");
  await takeScreenshot(driver, narrowerShot);
  summary.screenshots.push(narrowerShot);

  await clickVisibleButtonByAriaLabel(driver, "Collapse inspector", 10000);
  await waitForVisibleElement(driver, ".library-inspector-shell.inspector-collapsed", 10000);
  await assertLibraryLayoutGeometry(driver, summary, "inspector-collapsed-layout");
  const collapsedShot = path.join(runDir, "10-library-inspector-collapsed.png");
  await takeScreenshot(driver, collapsedShot);
  summary.screenshots.push(collapsedShot);

  await clickVisibleButtonByAriaLabel(driver, "Expand inspector panel", 10000);
  await waitForVisibleElement(driver, ".library-inspector-shell:not(.inspector-collapsed)", 10000);
  await assertLibraryLayoutGeometry(driver, summary, "inspector-expanded-layout");

  if (!Array.isArray(summary.inspectorAdjustabilityChecks)) {
    summary.inspectorAdjustabilityChecks = [];
  }
  summary.inspectorAdjustabilityChecks.push({
    initialWidth,
    widerWidth,
    narrowerWidth,
  });
}

async function setExperienceMode(driver, mode) {
  const label = mode === "casual" ? "Casual" : mode === "creator" ? "Creator" : "Seasoned";
  await navigateToScreen(driver, "settings", 30000);
  await waitForAnyText(driver, ["Pick your household vibe", "Experience"], 30000);
  await clickVisibleElement(
    driver,
    `//button[contains(@class, 'settings-view-card') and .//strong[normalize-space(.) = ${xpathString(label)}]]`,
    30000,
  );

  const startedAt = Date.now();
  while (Date.now() - startedAt < 10000) {
    const activeMode = await driver.executeScript(
      "return document.documentElement.dataset.userView || null;",
    );
    if (activeMode === mode) {
      return;
    }
    await sleep(driver, 200);
  }

  throw new Error(`Timed out waiting for ${label} mode to apply.`);
}

async function verifyLibraryLayoutForMode(driver, summary, mode, outputPath) {
  await setExperienceMode(driver, mode);
  await openLibraryScreen(driver);
  await clickVisibleButtonByAriaLabel(driver, "List view", 10000).catch(() => null);
  await waitForVisibleElement(driver, ".library-list-shell", 30000);
  await assertLibraryLayoutGeometry(driver, summary, `${mode}-list-layout`);
  await takeScreenshot(driver, outputPath);
  summary.screenshots.push(outputPath);
}

async function ensureLibraryIndexed(driver) {
  await openLibraryScreen(driver);
  await waitForVisibleElement(driver, ".library-top-strip", 30000);

  const listResult = await invokeTauri(driver, "list_library_files", { query: { limit: 200 } });
  if (!listResult.ok) {
    throw new Error(`Could not list library files: ${listResult.error}`);
  }

  if ((listResult.response?.items?.length ?? 0) > 0) {
    return listResult.response.items;
  }

  const startScan = await invokeTauri(driver, "start_scan");
  if (!startScan.ok) {
    throw new Error(`Could not start scan: ${startScan.error}`);
  }

  const deadline = Date.now() + 120000;
  while (Date.now() < deadline) {
    const status = await invokeTauri(driver, "get_scan_status");
    if (!status.ok) {
      throw new Error(`Could not read scan status: ${status.error}`);
    }
    const state = status.response?.state;
    if (state && state !== "running") {
      if (state !== "succeeded") {
        throw new Error(`Scan ended in unexpected state: ${state}`);
      }
      break;
    }
    await sleep(driver, 500);
  }

  await sleep(driver, 1200);
  const refreshed = await invokeTauri(driver, "list_library_files", { query: { limit: 200 } });
  if (!refreshed.ok) {
    throw new Error(`Could not list library files after scan: ${refreshed.error}`);
  }
  return refreshed.response?.items ?? [];
}

function pickTargets(items, session) {
  const genericWatchFile = session?.fixture?.genericWatchFile ?? "Generic_Watch_Mod_v1.0.package";
  const mccc = items.find((item) => {
    const filename = String(item.filename ?? "").toLowerCase();
    return filename.includes("mc_cmd_center") || filename.includes("mccc");
  });
  const generic = items.find((item) => String(item.filename ?? "").toLowerCase() === genericWatchFile.toLowerCase());

  if (!mccc) {
    throw new Error("Could not find an MCCC fixture row in the desktop library.");
  }

  return { mccc, generic };
}

function buildRowNeedles(item) {
  const filename = String(item.filename ?? "");
  const basename = filename.replace(/\.[^.]+$/, "");
  const normalized = basename.replace(/[_-]+/g, " ").replace(/\s+/g, " ").trim();
  const candidates = [
    item.title,
    item.displayName,
    item.name,
    filename,
    basename,
    normalized,
  ]
    .filter((value) => typeof value === "string" && value.trim().length > 0)
    .map((value) => value.trim());

  return [...new Set(candidates)];
}

function buildRowXpath(label) {
  const quoted = xpathString(label);
  return `//*[contains(@class, 'library-list-row')][contains(normalize-space(.), ${quoted})] | //tr[@role='button'][contains(normalize-space(.), ${quoted})]`;
}

async function openRow(driver, item) {
  const needles = buildRowNeedles(item);
  let lastError = null;
  for (const needle of needles) {
    try {
      await clickVisibleElement(driver, buildRowXpath(needle), 10000);
      await sleep(driver, 600);
      return needle;
    } catch (error) {
      lastError = error;
    }
  }

  throw lastError ?? new Error(`Could not open a library row for ${item.filename}`);
}

async function openLibraryPreflightFor(driver, item) {
  await openLibraryScreen(driver);
  await clickVisibleButtonByAriaLabel(driver, "List view", 10000).catch(() => null);
  await waitForVisibleElement(driver, ".library-list-body", 30000);
  await openRow(driver, item);
  await waitForVisibleCssText(driver, ".action-preflight-card", 30000);
  await clickVisibleButton(driver, "Review cautions");
  return await waitForVisibleCssText(driver, ".action-preflight-detail-block", 30000);
}

async function main() {
  const session = loadSession();
  const appPath = resolveAppPath();
  const runDir = path.join(OUTPUT_ROOT, timestampLabel());
  ensureDir(runDir);

  const summary = {
    startedAt: new Date().toISOString(),
    webdriverUrl: WEBDRIVER_URL,
    appPath,
    sessionFile: SESSION_FILE,
    outputDir: runDir,
    screenshots: [],
    targets: {},
  };

  const capabilities = new Capabilities();
  capabilities.setBrowserName("wry");
  capabilities.set("tauri:options", { application: appPath });

  const driver = await new Builder().usingServer(WEBDRIVER_URL).withCapabilities(capabilities).build();

  try {
    await waitForAnyText(driver, ["HOME", "INBOX", "SETTINGS"], 60000);
    await installRuntimeErrorCapture(driver);
    const items = await ensureLibraryIndexed(driver);
    const targets = pickTargets(items, session);
    summary.targets = {
      mccc: targets.mccc.filename,
      generic: targets.generic?.filename ?? null,
    };

    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "casual",
      path.join(runDir, "00-library-layout-casual.png"),
    );
    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "seasoned",
      path.join(runDir, "00-library-layout-seasoned.png"),
    );
    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "creator",
      path.join(runDir, "00-library-layout-creator.png"),
    );
    await setExperienceMode(driver, "seasoned");
    await openLibraryScreen(driver);
    summary.mcccRowNeedle = await openRow(driver, targets.mccc);
    summary.mcccCompactText = await waitForVisibleCssText(driver, ".action-preflight-card", 30000);
    const selectedShot = path.join(runDir, "01-library-selected-mccc.png");
    await takeScreenshot(driver, selectedShot);
    summary.screenshots.push(selectedShot);
    await assertLibraryLayoutGeometry(driver, summary, "list-selected-layout");
    const layoutShot = path.join(runDir, "library-layout-overlap-fixed.png");
    await takeScreenshot(driver, layoutShot);
    summary.screenshots.push(layoutShot);
    const polishShot = path.join(runDir, "library-row-sidebar-polish.png");
    await takeScreenshot(driver, polishShot);
    summary.screenshots.push(polishShot);
    await verifyLibraryInspectorAdjustability(driver, summary, runDir);

    await clickVisibleButtonByAriaLabel(driver, "Grid view");
    await waitForVisibleElement(driver, ".library-grid", 30000);
    await waitForVisibleElement(driver, ".library-card", 30000);
    const gridShot = path.join(runDir, "02-library-grid-view.png");
    await takeScreenshot(driver, gridShot);
    summary.screenshots.push(gridShot);

    await clickVisibleButtonByAriaLabel(driver, "Folders view");
    await waitForVisibleElement(driver, ".library-folders-layout", 30000);
    await waitForAnyText(driver, ["Game folders", "Direct files", "Mods", "Tray"], 30000);
    const foldersShot = path.join(runDir, "03-library-folder-view.png");
    await takeScreenshot(driver, foldersShot);
    summary.screenshots.push(foldersShot);

    await clickVisibleButtonByAriaLabel(driver, "List view");
    await openRow(driver, targets.mccc);
    summary.detailSheetButton = await clickAnyVisibleButton(driver, ["Inspect file", "More details"], 30000);
    await waitForVisibleElement(driver, ".library-detail-sheet", 30000);
    const detailSheetShot = path.join(runDir, "04-library-detail-sheet.png");
    await takeScreenshot(driver, detailSheetShot);
    summary.screenshots.push(detailSheetShot);
    await clickAnyVisibleButton(driver, ["Done"], 30000);
    await sleep(driver, 400);

    await clickVisibleButton(driver, "Review cautions");
    summary.mcccDetailText = await waitForVisibleCssText(driver, ".action-preflight-detail-block", 30000);
    const detailShot = path.join(runDir, "05-library-preflight-detail-mccc.png");
    await takeScreenshot(driver, detailShot);
    summary.screenshots.push(detailShot);

    summary.duplicatesPreflightText = summary.mcccDetailText;
    await clickVisibleButton(driver, "Open in Duplicates");
    summary.duplicatesHash = await waitForHash(driver, "#duplicates", 30000);
    await waitForVisibleElement(driver, ".duplicates-screen", 30000);
    await waitForAnyText(driver, ["Opened from Library"], 30000);
    const duplicatesBody = await getBodyText(driver);
    summary.duplicatesBodyHasMccc = /mc_cmd_center|mc cmd center|mccc/i.test(duplicatesBody);
    summary.duplicatesBodyHasLibraryFocus = /opened from library/i.test(duplicatesBody);
    summary.duplicatesContextExcerpt = duplicatesBody.slice(0, 1500);
    if (!summary.duplicatesBodyHasMccc || !summary.duplicatesBodyHasLibraryFocus) {
      throw new Error("Duplicates bridge opened Duplicates without visible Library file context.");
    }
    const duplicatesShot = path.join(runDir, "06-duplicates-bridge-mccc.png");
    await takeScreenshot(driver, duplicatesShot);
    summary.screenshots.push(duplicatesShot);
    await assertNoRuntimeErrors(driver, summary, "duplicates-bridge");

    summary.updatesPreflightText = await openLibraryPreflightFor(driver, targets.mccc);
    await clickVisibleButton(driver, "Open in Updates");
    summary.updatesHash = await waitForHash(driver, "#updates", 30000);
    await waitForVisibleElement(driver, ".updates-workbench", 30000);
    await waitForAnyText(driver, ["Updates", "Needs source", "No update source"], 30000);
    const updatesBody = await getBodyText(driver);
    summary.updatesBodyHasMccc = /mc_cmd_center|mc cmd center|mccc/i.test(updatesBody);
    summary.updatesContextExcerpt = updatesBody.slice(0, 1500);
    if (!summary.updatesBodyHasMccc) {
      throw new Error("Updates bridge opened Updates without visible MCCC file context.");
    }
    const updatesShot = path.join(runDir, "07-updates-bridge-mccc.png");
    await takeScreenshot(driver, updatesShot);
    summary.screenshots.push(updatesShot);
    await assertNoRuntimeErrors(driver, summary, "updates-bridge");

    summary.runtimeErrors = await readRuntimeErrors(driver);
    summary.browserLogInspection = await readBrowserLogEntries(driver);
    if (summary.runtimeErrors.length > 0) {
      throw new Error(
        `Library proof captured ${summary.runtimeErrors.length} runtime error(s).`,
      );
    }

    summary.finishedAt = new Date().toISOString();
    summary.ok = true;
  } catch (error) {
    summary.finishedAt = new Date().toISOString();
    summary.ok = false;
    summary.error = error instanceof Error ? error.message : String(error);
    try {
      summary.runtimeErrors = await readRuntimeErrors(driver);
      summary.browserLogInspection = await readBrowserLogEntries(driver);
    } catch {
      // keep the primary failure visible
    }
    try {
      const failureShot = path.join(runDir, "failure.png");
      await takeScreenshot(driver, failureShot);
      summary.screenshots.push(failureShot);
    } catch {
      // ignore screenshot failure on top of primary failure
    }
    throw error;
  } finally {
    fs.writeFileSync(path.join(runDir, "summary.json"), JSON.stringify(summary, null, 2));
    fs.writeFileSync(path.join(OUTPUT_ROOT, "latest-summary.json"), JSON.stringify(summary, null, 2));
    await driver.quit();
  }
}

main().then(() => {
  console.log(`DESKTOP_LIBRARY_PROOF_OK output=${path.resolve(OUTPUT_ROOT, "latest-summary.json")}`);
}).catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
