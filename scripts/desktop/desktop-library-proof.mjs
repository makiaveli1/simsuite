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

async function waitForBodyMatch(driver, pattern, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const body = await getBodyText(driver);
    if (pattern.test(body)) {
      return body;
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for body text matching ${pattern}`);
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

async function collectPreviewDiagnostics(driver, summary, label) {
  const result = await invokeTauri(driver, "get_library_preview_diagnostics");
  if (!Array.isArray(summary.previewDiagnostics)) {
    summary.previewDiagnostics = [];
  }
  if (!result.ok) {
    summary.previewDiagnostics.push({ label, ok: false, error: result.error });
    throw new Error(`Preview diagnostics failed: ${result.error}`);
  }

  const diagnostics = result.response;
  summary.previewDiagnostics.push({ label, ok: true, diagnostics });
  if (
    diagnostics &&
    diagnostics.totalRows !== diagnostics.rowsWithPreview + diagnostics.rowsWithoutPreview
  ) {
    throw new Error(
      `Preview diagnostics count mismatch for ${label}: totalRows=${diagnostics.totalRows}`,
    );
  }
  return diagnostics;
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
    const rectForElement = (element) => {
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
    const isVisible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = window.getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden";
    };
    const commandControls = Array.from(document.querySelectorAll(".library-command-actions > *"))
      .filter(isVisible)
      .map((element, index) => ({ index, className: element.className, rect: rectForElement(element) }));
    const commandControlOverlapFailures = [];
    for (let a = 0; a < commandControls.length; a += 1) {
      for (let b = a + 1; b < commandControls.length; b += 1) {
        const first = commandControls[a];
        const second = commandControls[b];
        const horizontalOverlap = Math.min(first.rect.right, second.rect.right) - Math.max(first.rect.left, second.rect.left);
        const verticalOverlap = Math.min(first.rect.bottom, second.rect.bottom) - Math.max(first.rect.top, second.rect.top);
        if (horizontalOverlap > 3 && verticalOverlap > 3) {
          commandControlOverlapFailures.push({ first, second, horizontalOverlap, verticalOverlap });
        }
      }
    }
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
    const rowThumbnailFailures = Array.from(document.querySelectorAll(".library-list-body .library-list-row"))
      .slice(0, 10)
      .map((row, rowIndex) => {
        const thumb = row.querySelector(".library-row-thumb-frame");
        if (!thumb) {
          return { rowIndex, reason: "missing thumbnail frame" };
        }
        const rect = thumb.getBoundingClientRect();
        if (rect.width < 40 || rect.height < 40) {
          return {
            rowIndex,
            reason: "thumbnail frame too small",
            width: rect.width,
            height: rect.height,
          };
        }
        return null;
      })
      .filter(Boolean);

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
      commandRow: rectFor(".library-command-row"),
      filterPanel: rectFor(".library-filter-collapse-shell"),
      filterToggle: rectFor("button[aria-controls='library-filter-panel']"),
      filterDeck: rectFor(".library-filter-deck"),
      activeFilterRow: rectFor(".library-active-filter-row"),
      advancedButton: rectFor(".library-advanced-btn"),
      searchInput: rectFor("input[aria-label='Search library']"),
      browseRow: rectFor(".library-browse-row"),
      listShell: rectFor(".library-list-shell"),
      listHeader: rectFor(".library-list-header"),
      listBody: rectFor(".library-list-body"),
      footer: rectFor(".library-stage-shell > .table-footer"),
      activeNav: rectFor(".rail-nav.is-active"),
      activeNavText: rectFor(".rail-nav.is-active span"),
      commandControlOverlapFailures,
      rowClipFailures,
      rowThumbnailFailures,
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

  if (geometry.activeFilterRow && geometry.listHeader && geometry.activeFilterRow.bottom > geometry.listHeader.top + tolerance) {
    failures.push("Library active filter row overlaps the list header");
  }

  if (geometry.commandControlOverlapFailures?.length > 0) {
    failures.push(`Library command controls overlap in ${geometry.commandControlOverlapFailures.length} visible pair(s)`);
  }

  if (!geometry.searchInput || geometry.searchInput.width < 120 || geometry.searchInput.height < 20) {
    failures.push("Library search input is not visible or usable");
  }

  if (!geometry.filterToggle || geometry.filterToggle.width < 64 || geometry.filterToggle.height < 24) {
    failures.push("Library Filters toggle is not visible or usable");
  }

  if (!geometry.advancedButton || geometry.advancedButton.width < 64 || geometry.advancedButton.height < 24) {
    failures.push("Library Advanced control is not visible or usable");
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

  if (geometry.rowThumbnailFailures?.length > 0) {
    failures.push(`Library row thumbnails are missing or too small in ${geometry.rowThumbnailFailures.length} visible row(s)`);
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

async function assertLibraryFilterCollapseGeometry(driver, summary, label, expectedExpanded) {
  const state = await driver.executeScript(() => {
    const rectFor = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const style = window.getComputedStyle(element);
      return {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
        width: rect.width,
        height: rect.height,
        display: style.display,
        visibility: style.visibility,
        opacity: style.opacity,
      };
    };
    const doc = document.documentElement;
    const toggle = document.querySelector("button[aria-controls='library-filter-panel']");
    const shell = document.querySelector(".library-filter-collapse-shell");
    return {
      toggle: rectFor("button[aria-controls='library-filter-panel']"),
      shell: rectFor(".library-filter-collapse-shell"),
      searchInput: rectFor("input[aria-label='Search library']"),
      listHeader: rectFor(".library-list-header"),
      topStrip: rectFor(".library-top-strip"),
      ariaExpanded: toggle?.getAttribute("aria-expanded") ?? null,
      ariaHidden: shell?.getAttribute("aria-hidden") ?? null,
      className: shell?.className ?? "",
      overflow: doc.scrollWidth - doc.clientWidth,
    };
  });

  const failures = [];
  const tolerance = 4;
  if (!state.toggle || state.toggle.width < 64 || state.toggle.height < 24) {
    failures.push("Filters toggle is not visible or usable");
  }
  if (!state.searchInput || state.searchInput.width < 120 || state.searchInput.height < 20) {
    failures.push("Search input is not visible while filters are collapsed/expanded");
  }
  if (!state.shell) {
    failures.push("Filter collapse shell is missing");
  }
  if (state.overflow > tolerance) {
    failures.push(`document has ${state.overflow}px horizontal overflow`);
  }
  if (expectedExpanded) {
    if (state.ariaExpanded !== "true" || state.ariaHidden !== "false" || !state.className.includes("is-expanded")) {
      failures.push("Filter panel did not report the expanded state");
    }
    if (state.shell && state.shell.height < 40) {
      failures.push("Expanded filter panel is unexpectedly short");
    }
  } else {
    if (state.ariaExpanded !== "false" || state.ariaHidden !== "true" || !state.className.includes("is-collapsed")) {
      failures.push("Filter panel did not report the collapsed state");
    }
    if (state.shell && state.shell.height > 12) {
      failures.push(`Collapsed filter panel still consumes ${Math.round(state.shell.height)}px`);
    }
  }
  if (state.topStrip && state.listHeader && state.topStrip.bottom > state.listHeader.top + tolerance) {
    failures.push("Collapsed/expanded filter area overlaps the list header");
  }

  if (!Array.isArray(summary.filterCollapseChecks)) {
    summary.filterCollapseChecks = [];
  }
  summary.filterCollapseChecks.push({
    label,
    ok: failures.length === 0,
    failures,
    state,
  });
  if (failures.length > 0) {
    throw new Error(`${label} filter collapse geometry failed: ${failures.join("; ")}`);
  }
}

async function assertLibraryInspectorRouteActions(driver, summary, label) {
  const result = await driver.executeScript(() => {
    const routeLabels = [
      "Open in Updates",
      "Compare in Duplicates",
      "Open in Duplicates",
      "Open Needs Review",
      "Review this file",
    ];
    const isVisible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = window.getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none";
    };
    const buttons = Array.from(document.querySelectorAll(".library-inspector-shell button"))
      .filter(isVisible)
      .map((button) => (button.textContent ?? "").replace(/\s+/g, " ").trim())
      .filter(Boolean);
    const counts = Object.fromEntries(
      routeLabels.map((routeLabel) => [
        routeLabel,
        buttons.filter((label) => label.toLowerCase() === routeLabel.toLowerCase()).length,
      ]),
    );
    return {
      buttons,
      counts,
      duplicates: Object.entries(counts)
        .filter(([, count]) => count > 1)
        .map(([routeLabel, count]) => ({ routeLabel, count })),
    };
  });

  if (!Array.isArray(summary.inspectorRouteActionChecks)) {
    summary.inspectorRouteActionChecks = [];
  }
  summary.inspectorRouteActionChecks.push({
    label,
    ...result,
  });
  if (result.duplicates.length > 0) {
    throw new Error(
      `${label} inspector has repeated route actions: ${result.duplicates
        .map((item) => `${item.routeLabel} x${item.count}`)
        .join(", ")}`,
    );
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

async function setLibrarySearch(driver, value) {
  const input = await waitForVisibleElement(driver, "input[aria-label='Search library']", 30000);
  await input.clear();
  if (value) {
    await input.sendKeys(value);
  }
  await sleep(driver, 600);
}

async function verifyLibraryFilterUx(driver, summary, runDir) {
  if (!Array.isArray(summary.filterUxChecks)) {
    summary.filterUxChecks = [];
  }

  await setExperienceMode(driver, "seasoned");
  await openLibraryScreen(driver);
  await clickVisibleButtonByAriaLabel(driver, "List view", 10000).catch(() => null);
  await waitForVisibleElement(driver, ".library-list-shell", 30000);

  await clickVisibleButton(driver, "Script Mods");
  await sleep(driver, 500);
  await clickVisibleButton(driver, "No update source");
  await sleep(driver, 500);
  await setLibrarySearch(driver, "mc");
  await waitForVisibleElement(driver, ".library-active-filter-row", 30000);
  await assertLibraryLayoutGeometry(driver, summary, "filter-active-state-layout");
  const activeShot = path.join(runDir, "library-filter-active-state.png");
  await takeScreenshot(driver, activeShot);
  summary.screenshots.push(activeShot);

  await clickVisibleButton(driver, "Advanced");
  await waitForVisibleElement(driver, ".library-filter-drawer", 30000);
  await assertLibraryLayoutGeometry(driver, summary, "filter-advanced-open-layout");
  const advancedShot = path.join(runDir, "library-filter-advanced-open.png");
  await takeScreenshot(driver, advancedShot);
  summary.screenshots.push(advancedShot);

  await setLibrarySearch(driver, "zzzz-no-match-filter-proof");
  await waitForAnyText(driver, ["No indexed files match", "No files match"], 30000);
  await assertLibraryLayoutGeometry(driver, summary, "filter-no-results-layout");
  const noResultsShot = path.join(runDir, "library-filter-no-results.png");
  await takeScreenshot(driver, noResultsShot);
  summary.screenshots.push(noResultsShot);

  await clickVisibleButton(driver, "Clear filters");
  await sleep(driver, 700);
  await clickVisibleButton(driver, "Advanced").catch(() => null);
  await waitForVisibleElement(driver, ".library-list-shell", 30000);
  await assertLibraryLayoutGeometry(driver, summary, "filter-cleared-layout");

  await clickVisibleButton(driver, "Filters");
  await sleep(driver, 500);
  await assertLibraryFilterCollapseGeometry(driver, summary, "filter-collapsed-layout", false);
  const collapsedShot = path.join(runDir, "library-filter-collapsed.png");
  await takeScreenshot(driver, collapsedShot);
  summary.screenshots.push(collapsedShot);

  await clickVisibleButton(driver, "Filters");
  await sleep(driver, 500);
  await assertLibraryFilterCollapseGeometry(driver, summary, "filter-expanded-layout", true);
  await assertLibraryLayoutGeometry(driver, summary, "filter-reexpanded-layout");

  summary.filterUxChecks.push({
    activeStateScreenshot: activeShot,
    advancedScreenshot: advancedShot,
    noResultsScreenshot: noResultsShot,
    collapsedScreenshot: collapsedShot,
  });
}

async function ensureLibraryFiltersExpanded(driver) {
  const expanded = await driver.executeScript(() => {
    return document
      .querySelector("button[aria-controls='library-filter-panel']")
      ?.getAttribute("aria-expanded");
  });
  if (expanded !== "true") {
    await clickVisibleButton(driver, "Filters");
    await sleep(driver, 400);
  }
}

async function ensureLibraryAdvancedOpen(driver) {
  await ensureLibraryFiltersExpanded(driver);
  const expanded = await driver.executeScript(() => {
    return document
      .querySelector(".library-advanced-btn")
      ?.getAttribute("aria-expanded");
  });
  if (expanded !== "true") {
    await clickVisibleButton(driver, "Advanced");
    await sleep(driver, 400);
  }
  await waitForVisibleElement(driver, ".library-filter-drawer", 30000);
}

async function assertLibraryAtmosphere(driver, summary, label) {
  const state = await driver.executeScript(() => {
    const doc = document.documentElement;
    const button = Array.from(document.querySelectorAll("button")).find((candidate) =>
      (candidate.getAttribute("aria-label") ?? "").toLowerCase().includes("cozy library glow"),
    );
    const workbench = document.querySelector(".library-workbench");
    const stage = document.querySelector(".library-stage-shell");
    const inspector = document.querySelector(".library-inspector-shell");
    const rectFor = (element) => {
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
    return {
      rootAtmosphere: doc.dataset.libraryAtmosphere ?? null,
      workbenchAtmosphere: workbench?.getAttribute("data-library-atmosphere") ?? null,
      buttonPressed: button?.getAttribute("aria-pressed") ?? null,
      buttonLabel: button?.getAttribute("aria-label") ?? null,
      overflow: doc.scrollWidth - doc.clientWidth,
      stage: rectFor(stage),
      inspector: rectFor(inspector),
    };
  });

  const failures = [];
  if (state.rootAtmosphere !== "cozy") {
    failures.push("Root atmosphere state is not cozy");
  }
  if (state.workbenchAtmosphere !== "cozy") {
    failures.push("Library workbench does not expose cozy atmosphere");
  }
  if (state.buttonPressed !== "true") {
    failures.push("Cozy glow toggle is not pressed");
  }
  if (state.overflow > 4) {
    failures.push(`document has ${state.overflow}px horizontal overflow`);
  }

  if (!Array.isArray(summary.atmosphereChecks)) {
    summary.atmosphereChecks = [];
  }
  summary.atmosphereChecks.push({
    label,
    ok: failures.length === 0,
    failures,
    state,
  });

  if (failures.length > 0) {
    throw new Error(`${label} atmosphere check failed: ${failures.join("; ")}`);
  }
}

async function verifyLibraryAtmosphere(driver, summary, runDir) {
  await setExperienceMode(driver, "seasoned");
  await openLibraryScreen(driver);
  await clickVisibleButtonByAriaLabel(driver, "List view", 10000).catch(() => null);
  await waitForVisibleElement(driver, ".library-list-shell", 30000);
  await ensureLibraryAdvancedOpen(driver);

  const pressed = await driver.executeScript(() => {
    const button = Array.from(document.querySelectorAll("button")).find((candidate) =>
      (candidate.getAttribute("aria-label") ?? "").toLowerCase().includes("cozy library glow"),
    );
    return button?.getAttribute("aria-pressed") ?? null;
  });
  if (pressed !== "true") {
    await clickVisibleElement(
      driver,
      "//button[contains(@aria-label, 'cozy Library glow')]",
      30000,
    );
    await sleep(driver, 500);
  }

  await assertLibraryAtmosphere(driver, summary, "library-atmosphere-on");
  const atmosphereShot = path.join(runDir, "library-cozy-polish-atmosphere-on.png");
  await takeScreenshot(driver, atmosphereShot);
  summary.screenshots.push(atmosphereShot);

  await clickVisibleButton(driver, "Filters");
  await sleep(driver, 500);
  await assertLibraryFilterCollapseGeometry(driver, summary, "cozy-filter-collapsed-layout", false);
  const collapsedShot = path.join(runDir, "library-cozy-polish-filters-collapsed.png");
  await takeScreenshot(driver, collapsedShot);
  summary.screenshots.push(collapsedShot);

  await clickVisibleButton(driver, "Filters");
  await sleep(driver, 500);
  await assertLibraryFilterCollapseGeometry(driver, summary, "cozy-filter-expanded-layout", true);
  await assertLibraryLayoutGeometry(driver, summary, "cozy-list-layout");
  const listShot = path.join(runDir, "library-cozy-polish-list.png");
  await takeScreenshot(driver, listShot);
  summary.screenshots.push(listShot);
}

async function assertLibraryGridDensityControl(driver, summary, label) {
  if (!Array.isArray(summary.gridDensityControlChecks)) {
    summary.gridDensityControlChecks = [];
  }

  const geometry = await driver.executeScript(() => {
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

    const slider = document.querySelector("input[aria-label='Grid card size']");

    return {
      rail: rectFor(".library-density-rail"),
      label: document.querySelector(".density-rail-label")?.textContent?.trim() ?? null,
      value: document.querySelector(".density-rail-value")?.textContent?.trim() ?? null,
      track: rectFor(".density-rail-track"),
      thumb: rectFor(".density-rail-thumb"),
      slider: rectFor("input[aria-label='Grid card size']"),
      sliderValueText: slider?.getAttribute("aria-valuetext") ?? null,
    };
  });

  const failures = [];
  if (!geometry.rail || geometry.rail.width < 190 || geometry.rail.height < 30) {
    failures.push("Grid card size control is not visible or large enough");
  }
  if (!geometry.track || geometry.track.width < 70 || geometry.track.height < 20) {
    failures.push("Grid card size track is not visible or controllable");
  }
  if (!geometry.thumb || geometry.thumb.width < 16 || geometry.thumb.height < 16) {
    failures.push("Grid card size slider thumb is not visibly anchored");
  }
  if (!geometry.slider || geometry.slider.width < 70 || geometry.slider.height < 20 || !geometry.sliderValueText) {
    failures.push("Grid card size range input is not accessible");
  }
  if (!geometry.label) {
    failures.push("Grid card size label is missing");
  }

  summary.gridDensityControlChecks.push({ label, geometry, failures });

  if (failures.length > 0) {
    throw new Error(`Grid card size control geometry failed (${label}): ${failures.join("; ")}`);
  }
}

async function openFolderWithDirectFileRows(driver) {
  const folderNames = ["MCCC", "Lot51 Core Library", "Generic Watch", "Lumpinou Toolbox", "Mods"];
  let lastError = null;
  for (const folderName of folderNames) {
    try {
      await clickVisibleButton(driver, folderName);
      await waitForVisibleElement(driver, ".library-folder-content-pane .library-list-row .library-row-thumb-frame", 12000);
      return folderName;
    } catch (error) {
      lastError = error;
    }
  }

  throw lastError ?? new Error("Could not open a folder containing direct file rows.");
}

async function openEmptyProofFolder(driver, session) {
  const emptyFolderName = session?.fixture?.emptyProofFolder ?? "Empty Proof Folder";
  await clickVisibleButton(driver, emptyFolderName, 30000);
  await waitForAnyText(
    driver,
    [
      "0 files",
      "Folder is empty",
      "No direct files",
      "No files in this folder",
      "This folder exists on disk",
    ],
    30000,
  );
  return emptyFolderName;
}

async function setProofWindowSize(driver, width, height) {
  try {
    await driver.manage().window().setRect({ width, height });
    await sleep(driver, 700);
    return { ok: true, width, height };
  } catch (error) {
    return {
      ok: false,
      width,
      height,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

async function verifyLibraryResponsiveViewport(driver, summary, runDir, width, height) {
  const resize = await setProofWindowSize(driver, width, height);
  if (!Array.isArray(summary.responsiveViewportChecks)) {
    summary.responsiveViewportChecks = [];
  }

  if (!resize.ok) {
    summary.responsiveViewportChecks.push(resize);
    return;
  }

  await openLibraryScreen(driver);
  await clickVisibleButtonByAriaLabel(driver, "List view", 10000).catch(() => null);
  await waitForVisibleElement(driver, ".library-list-shell", 30000);
  await assertLibraryLayoutGeometry(driver, summary, `library-responsive-${width}x${height}`);
  const outputPath = path.join(runDir, `library-responsive-${width}x${height}.png`);
  await takeScreenshot(driver, outputPath);
  summary.screenshots.push(outputPath);
  const filterOutputPath = path.join(runDir, `library-filter-responsive-${width}x${height}.png`);
  await takeScreenshot(driver, filterOutputPath);
  summary.screenshots.push(filterOutputPath);
  summary.responsiveViewportChecks.push({
    ...resize,
    screenshot: outputPath,
    filterScreenshot: filterOutputPath,
  });
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
  const duplicate = generic?.hasDuplicate
    ? generic
    : items.find((item) => item?.hasDuplicate === true);

  if (!mccc) {
    throw new Error("Could not find an MCCC fixture row in the desktop library.");
  }

  return { mccc, generic, duplicate };
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
      duplicate: targets.duplicate?.filename ?? null,
    };
    await collectPreviewDiagnostics(driver, summary, "after-fixture-index");

    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "casual",
      path.join(runDir, "00-library-layout-casual.png"),
    );
    const filterCasualShot = path.join(runDir, "library-filter-ux-casual.png");
    await takeScreenshot(driver, filterCasualShot);
    summary.screenshots.push(filterCasualShot);
    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "seasoned",
      path.join(runDir, "00-library-layout-seasoned.png"),
    );
    const filterSeasonedShot = path.join(runDir, "library-filter-ux-seasoned.png");
    await takeScreenshot(driver, filterSeasonedShot);
    summary.screenshots.push(filterSeasonedShot);
    await verifyLibraryLayoutForMode(
      driver,
      summary,
      "creator",
      path.join(runDir, "00-library-layout-creator.png"),
    );
    const filterCreatorShot = path.join(runDir, "library-filter-ux-creator.png");
    await takeScreenshot(driver, filterCreatorShot);
    summary.screenshots.push(filterCreatorShot);
    await verifyLibraryFilterUx(driver, summary, runDir);
    await verifyLibraryAtmosphere(driver, summary, runDir);
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
    const fullUxShot = path.join(runDir, "library-full-ux-refinement.png");
    await takeScreenshot(driver, fullUxShot);
    summary.screenshots.push(fullUxShot);
    const visualListShot = path.join(runDir, "library-visual-productization-list.png");
    await takeScreenshot(driver, visualListShot);
    summary.screenshots.push(visualListShot);
    const visualInspectorShot = path.join(runDir, "library-visual-productization-inspector.png");
    await takeScreenshot(driver, visualInspectorShot);
    summary.screenshots.push(visualInspectorShot);
    await assertLibraryInspectorRouteActions(driver, summary, "selected-inspector-route-actions");
    await verifyLibraryInspectorAdjustability(driver, summary, runDir);

    await clickVisibleButtonByAriaLabel(driver, "Grid view");
    await waitForVisibleElement(driver, ".library-grid", 30000);
    await waitForVisibleElement(driver, ".library-card", 30000);
    const gridShot = path.join(runDir, "02-library-grid-view.png");
    await takeScreenshot(driver, gridShot);
    summary.screenshots.push(gridShot);
    const visualGridShot = path.join(runDir, "library-visual-productization-grid.png");
    await takeScreenshot(driver, visualGridShot);
    summary.screenshots.push(visualGridShot);
    await assertLibraryGridDensityControl(driver, summary, "grid-card-size-control");
    const gridSizeShot = path.join(runDir, "library-grid-size-control-polish.png");
    await takeScreenshot(driver, gridSizeShot);
    summary.screenshots.push(gridSizeShot);
    const cozyGridShot = path.join(runDir, "library-cozy-polish-grid.png");
    await takeScreenshot(driver, cozyGridShot);
    summary.screenshots.push(cozyGridShot);

    await clickVisibleButtonByAriaLabel(driver, "Folders view");
    await waitForVisibleElement(driver, ".library-folders-layout", 30000);
    await waitForAnyText(driver, ["Game folders", "Direct files", "Mods", "Tray"], 30000);
    const foldersShot = path.join(runDir, "03-library-folder-view.png");
    await takeScreenshot(driver, foldersShot);
    summary.screenshots.push(foldersShot);
    const visualFolderShot = path.join(runDir, "library-visual-productization-folder.png");
    await takeScreenshot(driver, visualFolderShot);
    summary.screenshots.push(visualFolderShot);
    summary.folderThumbnailProofFolder = await openFolderWithDirectFileRows(driver);
    await assertLibraryLayoutGeometry(driver, summary, "folder-direct-file-thumbnails-layout");
    const folderThumbnailShot = path.join(runDir, "library-folder-row-thumbnails.png");
    await takeScreenshot(driver, folderThumbnailShot);
    summary.screenshots.push(folderThumbnailShot);
    summary.emptyProofFolder = await openEmptyProofFolder(driver, session);
    await assertLibraryLayoutGeometry(driver, summary, "folder-empty-real-disk-folder-layout");
    const emptyFolderShot = path.join(runDir, "library-empty-folder-metadata.png");
    await takeScreenshot(driver, emptyFolderShot);
    summary.screenshots.push(emptyFolderShot);
    const cozyFolderShot = path.join(runDir, "library-cozy-polish-folder.png");
    await takeScreenshot(driver, cozyFolderShot);
    summary.screenshots.push(cozyFolderShot);

    await clickVisibleButtonByAriaLabel(driver, "List view");
    await openRow(driver, targets.mccc);
    summary.detailSheetButton = await clickAnyVisibleButton(driver, ["Inspect file", "More details"], 30000);
    await waitForVisibleElement(driver, ".library-detail-sheet", 30000);
    await collectPreviewDiagnostics(driver, summary, "after-detail-open");
    const detailSheetShot = path.join(runDir, "04-library-detail-sheet.png");
    await takeScreenshot(driver, detailSheetShot);
    summary.screenshots.push(detailSheetShot);
    const visualMoreDetailsShot = path.join(runDir, "library-visual-productization-more-details.png");
    await takeScreenshot(driver, visualMoreDetailsShot);
    summary.screenshots.push(visualMoreDetailsShot);
    const cozyDetailSheetShot = path.join(runDir, "library-cozy-polish-detail-sheet.png");
    await takeScreenshot(driver, cozyDetailSheetShot);
    summary.screenshots.push(cozyDetailSheetShot);
    await clickAnyVisibleButton(driver, ["Done"], 30000);
    await sleep(driver, 400);

    await clickVisibleButton(driver, "Review cautions");
    summary.mcccDetailText = await waitForVisibleCssText(driver, ".action-preflight-detail-block", 30000);
    const detailShot = path.join(runDir, "05-library-preflight-detail-mccc.png");
    await takeScreenshot(driver, detailShot);
    summary.screenshots.push(detailShot);
    const visualPreflightShot = path.join(runDir, "library-visual-productization-preflight.png");
    await takeScreenshot(driver, visualPreflightShot);
    summary.screenshots.push(visualPreflightShot);
    const cozyPreflightShot = path.join(runDir, "library-cozy-polish-preflight.png");
    await takeScreenshot(driver, cozyPreflightShot);
    summary.screenshots.push(cozyPreflightShot);

    if (!targets.duplicate) {
      throw new Error("Could not find an exact duplicate fixture row in the desktop library.");
    }
    summary.duplicatesPreflightText = await openLibraryPreflightFor(driver, targets.duplicate);
    await clickVisibleButton(driver, "Open in Duplicates");
    summary.duplicatesHash = await waitForHash(driver, "#duplicates", 30000);
    await waitForVisibleElement(driver, ".duplicates-screen", 30000);
    await waitForAnyText(driver, ["Opened from Library"], 30000);
    const duplicatesBody = await getBodyText(driver);
    const duplicateTargetFilename = String(targets.duplicate.filename ?? "");
    const duplicateTargetStem = duplicateTargetFilename.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ");
    summary.duplicatesBodyHasTarget =
      duplicatesBody.toLowerCase().includes(duplicateTargetFilename.toLowerCase()) ||
      duplicatesBody.toLowerCase().includes(duplicateTargetStem.toLowerCase());
    summary.duplicatesBodyHasLibraryFocus = /opened from library/i.test(duplicatesBody);
    summary.duplicatesContextExcerpt = duplicatesBody.slice(0, 1500);
    if (!summary.duplicatesBodyHasTarget || !summary.duplicatesBodyHasLibraryFocus) {
      throw new Error("Duplicates bridge opened Duplicates without visible Library file context.");
    }
    const duplicatesShot = path.join(runDir, "06-duplicates-bridge-exact.png");
    await takeScreenshot(driver, duplicatesShot);
    summary.screenshots.push(duplicatesShot);
    await assertNoRuntimeErrors(driver, summary, "duplicates-bridge");

    summary.updatesPreflightText = await openLibraryPreflightFor(driver, targets.mccc);
    await clickVisibleButton(driver, "Open in Updates");
    summary.updatesHash = await waitForHash(driver, "#updates", 30000);
    await waitForVisibleElement(driver, ".updates-workbench", 30000);
    await waitForAnyText(driver, ["Updates", "Needs source", "No update source"], 30000);
    const updatesBody = await waitForBodyMatch(
      driver,
      /mc_cmd_center|mc cmd center|mccc/i,
      30000,
    );
    summary.updatesBodyHasMccc = /mc_cmd_center|mc cmd center|mccc/i.test(updatesBody);
    summary.updatesContextExcerpt = updatesBody.slice(0, 1500);
    if (!summary.updatesBodyHasMccc) {
      throw new Error("Updates bridge opened Updates without visible MCCC file context.");
    }
    const updatesShot = path.join(runDir, "07-updates-bridge-mccc.png");
    await takeScreenshot(driver, updatesShot);
    summary.screenshots.push(updatesShot);
    await assertNoRuntimeErrors(driver, summary, "updates-bridge");

    await navigateToScreen(driver, "downloads", 30000);
    summary.inboxHash = await waitForHash(driver, "#downloads", 30000);
    await waitForVisibleElement(driver, ".downloads-shell", 30000);
    await waitForAnyText(
      driver,
      ["Review new downloads and imported batches", "No files changed", "Inbox"],
      30000,
    );
    const inboxBody = await getBodyText(driver);
    summary.inboxBodyHasIntakePurpose =
      /review new downloads and imported batches|inbox is the intake area/i.test(inboxBody);
    summary.inboxBodyHasNoFilesChanged = /no files changed/i.test(inboxBody);
    summary.inboxBodyHandsOffToOrganize =
      /create preview plan|open organize/i.test(inboxBody);
    summary.inboxPrimaryLabelsHaveRawIds = await driver.executeScript(`
      return Array.from(document.querySelectorAll(
        ".downloads-rail-title, .inbox-intake-copy h2, .downloads-item-main > strong"
      ))
        .map((element) => element.textContent || "")
        .some((text) => /^\\s*\\d{8,}\\s*$/.test(text));
    `);
    summary.inboxEnabledFileChangingButtons = await driver.executeScript(`
      return Array.from(document.querySelectorAll("button"))
        .filter((button) => !button.disabled)
        .map((button) => button.textContent || "")
        .filter((text) => /\\b(apply|commit|move files|clean up|cleanup|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|reject)\\b/i.test(text));
    `);
    summary.inboxContextExcerpt = inboxBody.slice(0, 1800);
    if (
      !summary.inboxBodyHasIntakePurpose ||
      !summary.inboxBodyHasNoFilesChanged ||
      !summary.inboxBodyHandsOffToOrganize
    ) {
      throw new Error("Inbox did not show the intake review boundary.");
    }
    if (summary.inboxPrimaryLabelsHaveRawIds) {
      throw new Error("Inbox used a raw internal ID as a primary batch label.");
    }
    if (summary.inboxEnabledFileChangingButtons.length > 0) {
      throw new Error(
        `Inbox exposed enabled file-changing controls: ${summary.inboxEnabledFileChangingButtons.join(", ")}`,
      );
    }
    const inboxShot = path.join(runDir, "inbox-batch-review-clarity-v1.png");
    await takeScreenshot(driver, inboxShot);
    summary.screenshots.push(inboxShot);
    await assertNoRuntimeErrors(driver, summary, "inbox-batch-review-clarity-v1");

    await navigateToScreen(driver, "organize", 30000);
    summary.organizeHash = await waitForHash(driver, "#organize", 30000);
    await waitForVisibleElement(driver, ".organize-screen", 30000);
    summary.sidebarHasTopLevelPlanPreview = await driver.executeScript(`
      return Array.from(document.querySelectorAll(".nav-stack[aria-label='Primary'] .rail-nav span"))
        .some((element) => /\\bPlan Preview\\b/i.test(element.textContent || ""));
    `);
    if (summary.sidebarHasTopLevelPlanPreview) {
      throw new Error("Plan Preview is still exposed as a top-level sidebar item.");
    }
    await waitForAnyText(
      driver,
      ["Create plan", "Saved plans", "Pending batches", "No files changed"],
      30000,
    );
    await clickVisibleButton(driver, "Generate preview", 30000);
    await waitForAnyText(
      driver,
      ["Suggested organization preview", "Why SimSuite suggested this", "No preview items", "Blocked"],
      60000,
    );
    await waitForAnyText(driver, ["Save preview plan", "No files changed"], 30000);
    const organizeCreateBody = await getBodyText(driver);
    summary.organizeCreateBodyHasSavePreview =
      /save preview plan/i.test(organizeCreateBody);
    await clickVisibleButton(driver, "Save preview plan", 30000);
    await waitForAnyText(
      driver,
      ["Saved as draft preview plan", "Saved plans", "Draft preview plans", "Plan details"],
      60000,
    );
    const organizeBody = await getBodyText(driver);
    summary.organizeBodyHasNoFilesChanged = /no files changed/i.test(organizeBody);
    summary.organizeBodyHasPlanBoundary =
      /create plan|saved plans|pending batches|preview only|review suggested organization plans/i.test(organizeBody);
    summary.organizeBodyHasPlanDetails =
      /plan details|source signals|blockers|no preview items|blocked/i.test(organizeBody);
    summary.organizeSavedPlanCreated =
      /saved as draft preview plan|draft preview plans/i.test(organizeBody);
    summary.organizeSavedPlanCancelVisible =
      /cancel draft|this only cancels the saved draft record/i.test(organizeBody);
    summary.organizeValidationPreviewVisible = /validation preview|check saved plan/i.test(
      organizeBody,
    );
    summary.organizeContextExcerpt = organizeBody.slice(0, 1800);
    summary.organizeEnabledFileChangingButtons = await driver.executeScript(`
      return Array.from(document.querySelectorAll("button"))
        .filter((button) => !button.disabled)
        .map((button) => button.textContent || "")
        .filter((text) => /\\b(apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation)\\b/i.test(text));
    `);
    if (
      !summary.organizeCreateBodyHasSavePreview ||
      !summary.organizeBodyHasNoFilesChanged ||
      !summary.organizeBodyHasPlanBoundary ||
      !summary.organizeSavedPlanCreated
    ) {
      throw new Error("Organize did not show the preview-only plan boundary.");
    }
    if (!summary.organizeBodyHasPlanDetails) {
      throw new Error("Organize generated a plan without visible review details.");
    }
    if (!summary.organizeSavedPlanCancelVisible) {
      throw new Error("Organize saved-plan review did not expose safe draft cancellation wording.");
    }
    if (!summary.organizeValidationPreviewVisible) {
      throw new Error("Organize saved-plan details did not expose validation preview controls.");
    }
    if (summary.organizeEnabledFileChangingButtons.length > 0) {
      throw new Error(
        `Organize exposed enabled file-changing controls: ${summary.organizeEnabledFileChangingButtons.join(", ")}`,
      );
    }
    await clickVisibleButton(driver, "Check saved plan", 30000);
    await waitForAnyText(
      driver,
      [
        "Validation caveats",
        "No current blocker found, but still preview-only",
        "Destination exists",
        "Missing source",
      ],
      60000,
    );
    const organizeValidationBody = await getBodyText(driver);
    summary.organizeValidationBodyHasPreview = /validation preview/i.test(
      organizeValidationBody,
    );
    summary.organizeValidationBodyHasSummary =
      /blocked|needs review|conflicts|stale paths|missing files|backup required/i.test(
        organizeValidationBody,
      );
    summary.organizeValidationBodyHasNoFilesChanged = /no files changed/i.test(
      organizeValidationBody,
    );
    summary.organizeValidationBodyBlocksConfirmation =
      /future confirmation blocked|apply is still not available/i.test(
        organizeValidationBody,
      );
    summary.organizeValidationContextExcerpt = organizeValidationBody.slice(0, 2000);
    summary.organizeValidationEnabledFileChangingButtons = await driver.executeScript(`
      return Array.from(document.querySelectorAll("button"))
        .filter((button) => !button.disabled)
        .map((button) => button.textContent || "")
        .filter((text) => /\\b(apply|commit|move files|clean up|quarantine|delete|fix|auto-sort now|sort automatically|safe to move|safe to delete|ready to apply|proceed to confirmation)\\b/i.test(text));
    `);
    if (
      !summary.organizeValidationBodyHasPreview ||
      !summary.organizeValidationBodyHasSummary ||
      !summary.organizeValidationBodyHasNoFilesChanged ||
      !summary.organizeValidationBodyBlocksConfirmation
    ) {
      throw new Error("Organize validation preview did not show the review-only validation boundary.");
    }
    if (summary.organizeValidationEnabledFileChangingButtons.length > 0) {
      throw new Error(
        `Organize validation preview exposed enabled file-changing controls: ${summary.organizeValidationEnabledFileChangingButtons.join(", ")}`,
      );
    }
    const validationShot = path.join(runDir, "organize-validation-preview-ui-v1.png");
    await takeScreenshot(driver, validationShot);
    summary.screenshots.push(validationShot);
    const savedPlanShot = path.join(runDir, "organize-saved-plan-ui-review-v1.png");
    await takeScreenshot(driver, savedPlanShot);
    summary.screenshots.push(savedPlanShot);
    await clickVisibleButton(driver, "Pending batches", 30000);
    await waitForAnyText(
      driver,
      ["Pending batches", "No pending batches found", "No files changed"],
      30000,
    );
    const organizePendingBody = await getBodyText(driver);
    summary.organizePendingBodyHasPendingPlans = /pending batches/i.test(organizePendingBody);
    summary.organizePendingBodyHasNoFilesChanged = /no files changed/i.test(organizePendingBody);
    summary.organizePendingBodyExplainsMeaning =
      /saved organization drafts live in saved plans|pending batches|no pending batches found|inbox is the natural place/i.test(
        organizePendingBody,
      );
    summary.organizePendingTechnicalDetailsVisibleByDefault = /internal folder id/i.test(
      organizePendingBody,
    );
    summary.organizePendingPrimaryLabelsHaveRawIds = await driver.executeScript(`
      return Array.from(document.querySelectorAll(
        ".pending-batch-heading h3, .pending-plans-header h2, .pending-batches-heading h3"
      ))
        .map((element) => element.textContent || "")
        .some((text) => /\\b\\d{8,}\\b/.test(text));
    `);
    summary.organizePendingContextExcerpt = organizePendingBody.slice(0, 1800);
    if (
      !summary.organizePendingBodyHasPendingPlans ||
      !summary.organizePendingBodyHasNoFilesChanged ||
      !summary.organizePendingBodyExplainsMeaning
    ) {
      throw new Error("Organize did not show the pending plans preview boundary.");
    }
    if (summary.organizePendingTechnicalDetailsVisibleByDefault) {
      throw new Error("Organize showed internal pending-plan technical details by default.");
    }
    if (summary.organizePendingPrimaryLabelsHaveRawIds) {
      throw new Error("Organize used raw internal IDs as primary pending-plan labels.");
    }
    summary.organizePendingScrollCheck = await driver.executeScript(`
      const screen = document.querySelector(".organize-screen");
      const pending = document.querySelector(".organize-pending-plan-panel, .pending-plans-preview");
      if (!screen) {
        return { found: false };
      }
      screen.scrollTop = 0;
      const style = window.getComputedStyle(screen);
      const maxScroll = screen.scrollHeight - screen.clientHeight;
      const needsScroll = maxScroll > 4;
      screen.scrollTop = Math.max(0, maxScroll);
      const after = screen.scrollTop;
      const pendingRect = pending ? pending.getBoundingClientRect() : null;
      return {
        found: true,
        overflowY: style.overflowY,
        clientHeight: screen.clientHeight,
        scrollHeight: screen.scrollHeight,
        maxScroll,
        needsScroll,
        after,
        canScroll: !needsScroll || after > 0,
        pendingBottom: pendingRect ? pendingRect.bottom : null,
        viewportHeight: window.innerHeight,
      };
    `);
    if (
      !summary.organizePendingScrollCheck?.found ||
      summary.organizePendingScrollCheck.overflowY === "hidden" ||
      !summary.organizePendingScrollCheck.canScroll
    ) {
      throw new Error(
        `Organize pending plans view was not scrollable: ${JSON.stringify(summary.organizePendingScrollCheck)}`,
      );
    }
    const organizeShot = path.join(runDir, "organize-pending-batches-handoff-v1.png");
    await takeScreenshot(driver, organizeShot);
    summary.screenshots.push(organizeShot);
    await assertNoRuntimeErrors(driver, summary, "organize-plan-preview-consolidated-v1");

    await navigateToScreen(driver, "staging", 30000);
    summary.stagingHash = await waitForHash(driver, "#staging", 30000);
    await waitForVisibleElement(driver, ".staging-screen", 30000);
    await waitForAnyText(
      driver,
      ["Plan Preview", "Pending plans", "Preview plan only", "No files changed"],
      30000,
    );
    const stagingBody = await getBodyText(driver);
    summary.stagingBodyHasPreviewPlan = /plan preview|preview plan/i.test(stagingBody);
    summary.stagingBodyHasNoFilesChanged = /no files (will be )?changed/i.test(stagingBody);
    summary.stagingContextExcerpt = stagingBody.slice(0, 1500);
    summary.stagingEnabledFileChangingButtons = await driver.executeScript(`
      return Array.from(document.querySelectorAll("button"))
        .filter((button) => !button.disabled)
        .map((button) => button.textContent || "")
        .filter((text) => /commit|cleanup|delete|quarantine|move|apply/i.test(text));
    `);
    if (!summary.stagingBodyHasPreviewPlan || !summary.stagingBodyHasNoFilesChanged) {
      throw new Error("Plan Preview did not show the preview-only plan boundary.");
    }
    summary.stagingBodyExplainsOrganize = /main workflow now lives in organize|open organize/i.test(stagingBody);
    if (!summary.stagingBodyExplainsOrganize) {
      throw new Error("Direct Plan Preview route did not explain the Organize consolidation.");
    }
    if (summary.stagingEnabledFileChangingButtons.length > 0) {
      throw new Error(
        `Plan Preview exposed enabled file-changing controls: ${summary.stagingEnabledFileChangingButtons.join(", ")}`,
      );
    }
    const stagingShot = path.join(runDir, "plan-preview-direct-route-safe-v1.png");
    await takeScreenshot(driver, stagingShot);
    summary.screenshots.push(stagingShot);
    await assertNoRuntimeErrors(driver, summary, "plan-preview-direct-route-safe-v1");

    await verifyLibraryResponsiveViewport(driver, summary, runDir, 1366, 768);
    await verifyLibraryResponsiveViewport(driver, summary, runDir, 1440, 900);

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
