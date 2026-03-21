import { Builder, By, Capabilities, until } from "selenium-webdriver";
import fs from "node:fs";
import path from "node:path";

const WEBDRIVER_URL = process.env.SIMSUITE_WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const DEFAULT_SPECIAL_ITEM = "MCCC_Update_Test";
const DEFAULT_BLOCKED_ITEM = "MCCC_Partial_Blocked_Test";
const DEFAULT_XML_SAME_ITEM = "XML_Injector_Same_Test";
const DEFAULT_XML_OLDER_ITEM = "XML_Injector_Older_Test";
const DEFAULT_S4CL_SAME_ITEM = "S4CL_Same_Test";
const DEFAULT_S4CL_OLDER_ITEM = "S4CL_Older_Test";
const DEFAULT_LOT51_SAME_ITEM = "Lot51_Core_Same_Test";
const DEFAULT_LOT51_OLDER_ITEM = "Lot51_Core_Older_Test";
const DEFAULT_TOOLBOX_SAME_ITEM = "Toolbox_Same_Test";
const DEFAULT_TOOLBOX_OLDER_ITEM = "Toolbox_Older_Test";
const DEFAULT_SMART_CORE_SAME_ITEM = "Smart_Core_Same_Test";
const DEFAULT_SMART_CORE_OLDER_ITEM = "Smart_Core_Older_Test";
const DEFAULT_GENERIC_WATCH_FILE = "LittleMsSam_SendSimsToBed.ts4script";
const DEFAULT_APP_PATHS = [
  path.resolve("src-tauri", "target", "debug", "simsuite.exe"),
  path.resolve("src-tauri", "target", "debug", "SimSuite.exe"),
  path.resolve("src-tauri", "target", "release", "simsuite.exe"),
  path.resolve("src-tauri", "target", "release", "SimSuite.exe"),
];
const INCLUDE_APPLY = process.argv.includes("--include-apply");
const DEFAULT_SESSION_FILE = path.resolve("output", "desktop", "tauri-driver-session.json");
const DEFAULT_APPLY_LABELS = ["Apply guided update", "Apply guided install", "Update safely", "Install safely"];
const SPECIAL_SETUP_LANE_LABELS = ["Special setup"];
const WAITING_ON_YOU_LANE_LABELS = ["Waiting on you"];
const DONE_LANE_LABELS = ["Done"];
const FOLLOW_UP_LANE_LABELS = ["Waiting on you", "Blocked"];

function resolveAppPath() {
  const explicit = process.env.SIMSUITE_TAURI_APP_PATH;
  if (explicit && fs.existsSync(explicit)) {
    return explicit;
  }

  for (const candidate of DEFAULT_APP_PATHS) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    "SimSuite could not find the Tauri app binary. Set SIMSUITE_TAURI_APP_PATH or build the debug app first.",
  );
}

function loadDriverSession() {
  const sessionFile = process.env.SIMSUITE_TAURI_DRIVER_SESSION_FILE ?? DEFAULT_SESSION_FILE;
  if (!fs.existsSync(sessionFile)) {
    return null;
  }

  try {
    return JSON.parse(fs.readFileSync(sessionFile, "utf8"));
  } catch {
    return null;
  }
}

function xpathString(value) {
  if (!value.includes("'")) {
    return `'${value}'`;
  }
  if (!value.includes('"')) {
    return `"${value}"`;
  }
  return `concat(${value
    .split("'")
    .map((part) => `'${part}'`)
    .join(`, "'", `)})`;
}

async function findVisibleButton(driver, partialText, timeoutMs = 30000) {
  const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);
  await driver.wait(async () => {
    const elements = await driver.findElements(locator);
    for (const element of elements) {
      if (await element.isDisplayed()) {
        return true;
      }
    }
    return false;
  }, timeoutMs);
  const elements = await driver.findElements(locator);
  for (const element of elements) {
    if (await element.isDisplayed()) {
      return element;
    }
  }
  throw new Error(`Could not find a visible button containing "${partialText}".`);
}

async function clickButton(driver, partialText, timeoutMs = 30000) {
  const button = await findVisibleButton(driver, partialText, timeoutMs);
  await driver.wait(until.elementIsVisible(button), timeoutMs);
  await driver.wait(until.elementIsEnabled(button), timeoutMs);
  try {
    await button.click();
  } catch (error) {
    if (String(error).toLowerCase().includes("click intercepted")) {
      await driver.executeScript("arguments[0].click()", button);
      return;
    }
    throw error;
  }
}

async function clickButtonViaDom(driver, partialText, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const clicked = await driver.executeScript(
      `
        const targetText = arguments[0].toLowerCase();
        const buttons = Array.from(document.querySelectorAll('button'));
        const button = buttons.find((element) =>
          (element.innerText || element.textContent || '').toLowerCase().includes(targetText),
        );
        if (!button) {
          return false;
        }
        button.scrollIntoView({ block: 'center', inline: 'nearest' });
        button.click();
        return true;
      `,
      partialText,
    );

    if (clicked) {
      return;
    }

    await driver.sleep(250);
  }

  throw new Error(`Could not click a button containing "${partialText}" through the DOM.`);
}

async function dispatchMouseClick(driver, element) {
  await driver.executeScript(
    `
      const target = arguments[0];
      target.scrollIntoView({ block: 'center', inline: 'nearest' });
      const rect = target.getBoundingClientRect();
      const base = {
        bubbles: true,
        cancelable: true,
        composed: true,
        view: window,
        button: 0,
        buttons: 1,
        clientX: rect.left + rect.width / 2,
        clientY: rect.top + rect.height / 2,
      };
      target.dispatchEvent(new PointerEvent('pointerdown', { ...base, pointerId: 1, pointerType: 'mouse', isPrimary: true }));
      target.dispatchEvent(new MouseEvent('mousedown', base));
      target.dispatchEvent(new PointerEvent('pointerup', { ...base, pointerId: 1, pointerType: 'mouse', isPrimary: true }));
      target.dispatchEvent(new MouseEvent('mouseup', base));
      target.dispatchEvent(new MouseEvent('click', base));
    `,
    element,
  );
}

async function clickFirstVisibleButton(driver, labels, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    for (const label of labels) {
      const clicked = await tryClickVisibleButtonIfEnabled(driver, label);
      if (clicked) {
        return label;
      }
    }
    await driver.sleep(250);
  }
  throw new Error(`Could not find an enabled button for any of: ${labels.join(", ")}`);
}

async function tryClickVisibleButtonIfEnabled(driver, partialText) {
  const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
  const elements = await driver.findElements(locator);
  for (const element of elements) {
    if ((await element.isDisplayed()) && (await element.isEnabled())) {
      await element.click();
      return true;
    }
  }
  return false;
}

async function findVisibleTextElement(driver, partialText, timeoutMs = 30000) {
  const locator = By.xpath(`//*[contains(normalize-space(.), ${xpathString(partialText)})]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);
  await driver.wait(async () => {
    const elements = await driver.findElements(locator);
    for (const element of elements) {
      if (await element.isDisplayed()) {
        return true;
      }
    }
    return false;
  }, timeoutMs);
  const elements = await driver.findElements(locator);
  for (const element of elements) {
    if (await element.isDisplayed()) {
      return element;
    }
  }
  throw new Error(`Could not find visible text containing "${partialText}".`);
}

async function clickLibraryRow(driver, filename, timeoutMs = 30000) {
  const exactLocator = By.xpath(
    `//tr[@role='button'][.//div[contains(@class, 'file-title') and normalize-space(.) = ${xpathString(
      filename,
    )}]]`,
  );
  const partialLocator = By.xpath(
    `//tr[@role='button'][.//div[contains(@class, 'file-title') and contains(normalize-space(.), ${xpathString(
      filename,
    )})]]`,
  );

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    for (const locator of [exactLocator, partialLocator]) {
      const rows = await driver.findElements(locator);
      for (const row of rows) {
        if (await row.isDisplayed()) {
          await dispatchMouseClick(driver, row);
          try {
            await row.click();
          } catch (error) {
            if (String(error).toLowerCase().includes("click intercepted")) {
              await dispatchMouseClick(driver, row);
            } else {
              throw error;
            }
          }
          return;
        }
      }
    }
    await driver.sleep(250);
  }

  throw new Error(`Could not find a visible Library row for "${filename}".`);
}

async function clickUpdatesRow(driver, filename, timeoutMs = 30000) {
  const exactLocator = By.xpath(
    `//table[contains(@class, 'updates-table')]//tr[@role='button'][.//div[contains(@class, 'file-title') and normalize-space(.) = ${xpathString(
      filename,
    )}]]`,
  );
  const partialLocator = By.xpath(
    `//table[contains(@class, 'updates-table')]//tr[@role='button'][.//div[contains(@class, 'file-title') and contains(normalize-space(.), ${xpathString(
      filename,
    )})]]`,
  );

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    for (const locator of [exactLocator, partialLocator]) {
      const rows = await driver.findElements(locator);
      for (const row of rows) {
        if (await row.isDisplayed()) {
          await dispatchMouseClick(driver, row);
          try {
            await row.click();
          } catch (error) {
            if (String(error).toLowerCase().includes("click intercepted")) {
              await dispatchMouseClick(driver, row);
            } else {
              throw error;
            }
          }
          return;
        }
      }
    }

    await driver.sleep(250);
  }

  throw new Error(`Could not find a visible Updates row for "${filename}".`);
}

async function clickLibraryWatchEntryAction(driver, filename, labels, timeoutMs = 30000) {
  const expectedLabels = Array.isArray(labels) ? labels : [labels];
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    for (const label of expectedLabels) {
      const locator = By.xpath(
        `//div[contains(@class, 'library-watch-list-entry')][contains(normalize-space(.), ${xpathString(
          filename,
        )})]//button[contains(normalize-space(.), ${xpathString(label)})]`,
      );
      const buttons = await driver.findElements(locator);
      for (const button of buttons) {
        if ((await button.isDisplayed()) && (await button.isEnabled())) {
          await dispatchMouseClick(driver, button);
          try {
            await button.click();
          } catch (error) {
            if (String(error).toLowerCase().includes("click intercepted")) {
              await dispatchMouseClick(driver, button);
            } else {
              throw error;
            }
          }
          return label;
        }
      }
    }

    await driver.sleep(250);
  }

  throw new Error(
    `Could not find a visible Library watch action for "${filename}" using any of: ${expectedLabels.join(", ")}`,
  );
}

async function hasVisibleButton(driver, partialText) {
  const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
  const buttons = await driver.findElements(locator);
  for (const button of buttons) {
    if ((await button.isDisplayed()) && (await button.isEnabled())) {
      return true;
    }
  }
  return false;
}

async function openTrackedWatchItemWithCheckNow(driver, timeoutMs = 30000) {
  const locator = By.xpath(
    "//div[contains(@class, 'library-watch-filter-row')]/following-sibling::div[contains(@class, 'library-watch-list')][1]//button[contains(@class, 'library-watch-list-main')]",
  );

  await driver.wait(until.elementLocated(locator), timeoutMs);
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const buttons = await driver.findElements(locator);
    for (const button of buttons) {
      if (!(await button.isDisplayed())) {
        continue;
      }

      await driver.executeScript("arguments[0].click()", button);
      await driver.sleep(400);

      if (await hasVisibleButton(driver, "Check now")) {
        return;
      }
    }

    await driver.sleep(250);
  }

  throw new Error("Could not open a tracked watch item with a visible Check now button.");
}

async function getBodyText(driver) {
  try {
    return await driver.findElement(By.css("body")).getText();
  } catch {
    return null;
  }
}

async function clickVisibleText(driver, partialText, timeoutMs = 30000) {
  try {
    await clickButton(driver, partialText, timeoutMs);
    return;
  } catch {
  }

  const element = await findVisibleTextElement(driver, partialText, timeoutMs);
  await driver.wait(until.elementIsVisible(element), timeoutMs);
  try {
    await element.click();
  } catch (error) {
    if (String(error).toLowerCase().includes("click intercepted")) {
      await driver.executeScript("arguments[0].click()", element);
      return;
    }
    throw error;
  }
}

async function findVisibleQueueRow(driver, partialText, timeoutMs = 30000) {
  const label = xpathString(partialText);
  const locator = By.xpath(
    `//button[contains(@class, 'downloads-item-row')][.//strong[contains(normalize-space(.), ${label})]]`,
  );
  await driver.wait(until.elementLocated(locator), timeoutMs);
  await driver.wait(async () => {
    const rows = await driver.findElements(locator);
    for (const row of rows) {
      if (await row.isDisplayed()) {
        return true;
      }
    }
    return false;
  }, timeoutMs);
  const rows = await driver.findElements(locator);
  for (const row of rows) {
    if (await row.isDisplayed()) {
      return row;
    }
  }
  throw new Error(`Could not find a visible inbox row named "${partialText}".`);
}

async function findVisibleDownloadsLaneButton(driver, partialText, timeoutMs = 30000) {
  const label = xpathString(partialText);
  const locator = By.xpath(
    `//div[contains(@class, 'downloads-lane-picker')]//button[.//span[contains(@class, 'downloads-lane-button-title')][contains(normalize-space(.), ${label})]]`,
  );
  await driver.wait(until.elementLocated(locator), timeoutMs);
  await driver.wait(async () => {
    const buttons = await driver.findElements(locator);
    for (const button of buttons) {
      if (await button.isDisplayed()) {
        return true;
      }
    }
    return false;
  }, timeoutMs);
  const buttons = await driver.findElements(locator);
  for (const button of buttons) {
    if (await button.isDisplayed()) {
      return button;
    }
  }
  throw new Error(`Could not find a visible Downloads lane button for "${partialText}".`);
}

async function openDownloadsLane(driver, labels, timeoutMs = 30000) {
  const laneLabels = Array.isArray(labels) ? labels : [labels];
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    for (const label of laneLabels) {
      try {
        const button = await findVisibleDownloadsLaneButton(
          driver,
          label,
          Math.min(5000, timeoutMs),
        );
        const classes = (await button.getAttribute("class")) ?? "";
        if (!classes.includes("is-active")) {
          await dispatchMouseClick(driver, button);
          try {
            await button.click();
          } catch (error) {
            if (String(error).toLowerCase().includes("click intercepted")) {
              await dispatchMouseClick(driver, button);
            } else {
              throw error;
            }
          }
        }

        await driver.wait(async () => {
          try {
            const refreshed = await findVisibleDownloadsLaneButton(driver, label, 1000);
            const refreshedClasses = (await refreshed.getAttribute("class")) ?? "";
            return refreshedClasses.includes("is-active");
          } catch {
            return false;
          }
        }, 4000);
        return label;
      } catch {
      }
    }

    await driver.sleep(250);
  }

  throw new Error(`Could not open any Downloads lane from: ${laneLabels.join(", ")}`);
}

async function clickQueueRow(driver, partialText, timeoutMs = 30000) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const row = await findVisibleQueueRow(driver, partialText, Math.min(5000, timeoutMs));
    await driver.wait(until.elementIsVisible(row), timeoutMs);
    await driver.wait(until.elementIsEnabled(row), timeoutMs);

    await dispatchMouseClick(driver, row);
    try {
      await row.click();
    } catch (error) {
      if (String(error).toLowerCase().includes("click intercepted")) {
        await dispatchMouseClick(driver, row);
      } else {
        throw error;
      }
    }

    const selected = await driver
      .wait(async () => {
        try {
          const refreshed = await findVisibleQueueRow(driver, partialText, 1000);
          const classes = (await refreshed.getAttribute("class")) ?? "";
          return classes.includes("is-selected");
        } catch {
          return false;
        }
      }, 2500)
      .then(() => true)
      .catch(() => false);

    if (selected) {
      return;
    }

    await driver.sleep(250);
  }

  throw new Error(`Could not select inbox row "${partialText}".`);
}

async function waitForQueueItemInLane(driver, partialText, laneLabels, timeoutMs = 90000) {
  const startedAt = Date.now();
  let lastRetryAt = 0;

  while (Date.now() - startedAt < timeoutMs) {
    for (const label of laneLabels) {
      try {
        await openDownloadsLane(driver, label, 5000);
        await findVisibleQueueRow(driver, partialText, 3000);
        return;
      } catch {
      }
    }

    if (
      Date.now() - startedAt > 15000 &&
      Date.now() - lastRetryAt > 12000 &&
      (await hasVisibleText(driver, "Check again"))
    ) {
      if (await tryClickVisibleButtonIfEnabled(driver, "Check again")) {
        lastRetryAt = Date.now();
      }
    }

    await driver.sleep(500);
  }

  await dumpBodyText(driver, `queue-timeout-${partialText}`);
  throw new Error(`Timed out waiting for inbox row "${partialText}" in lanes ${laneLabels.join(", ")}.`);
}

async function clickNamedQueueItemInLanes(driver, partialText, laneLabels, timeoutMs = 30000) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    for (const label of laneLabels) {
      try {
        await openDownloadsLane(driver, label, 5000);
        await clickQueueRow(driver, partialText, 5000);
        return;
      } catch {
      }
    }

    await driver.sleep(250);
  }

  throw new Error(`Could not select inbox row "${partialText}" in lanes ${laneLabels.join(", ")}.`);
}

async function waitForText(driver, text, timeoutMs = 30000) {
  const startedAt = Date.now();
  const target = text.toLowerCase();
  while (Date.now() - startedAt < timeoutMs) {
    const bodyText = await getBodyText(driver);
    if (bodyText && bodyText.toLowerCase().includes(target)) {
      return;
    }
    await driver.sleep(250);
  }
  throw new Error(`Timed out waiting for text "${text}".`);
}

async function waitForAnyText(driver, texts, timeoutMs = 30000) {
  const startedAt = Date.now();
  const targets = texts.map((text) => ({ raw: text, normalized: text.toLowerCase() }));
  while (Date.now() - startedAt < timeoutMs) {
    const bodyText = await getBodyText(driver);
    if (bodyText) {
      const normalizedBody = bodyText.toLowerCase();
      for (const target of targets) {
        if (normalizedBody.includes(target.normalized)) {
          return target.raw;
        }
      }
    }
    await driver.sleep(300);
  }
  throw new Error(`Timed out waiting for one of: ${texts.join(", ")}`);
}

async function hasVisibleText(driver, text) {
  const bodyText = await getBodyText(driver);
  return bodyText ? bodyText.toLowerCase().includes(text.toLowerCase()) : false;
}

async function acceptAlertIfPresent(driver, timeoutMs = 5000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const alert = await driver.switchTo().alert();
      await alert.accept();
      return true;
    } catch {
      await driver.sleep(200);
    }
  }
  return false;
}

async function ensureTextStaysHidden(driver, text, timeoutMs = 5000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    if (await hasVisibleText(driver, text)) {
      throw new Error(`Unexpected text became visible: ${text}`);
    }
    await driver.sleep(250);
  }
}

async function dumpBodyText(driver, label) {
  const body = (await getBodyText(driver)) ?? "[body not available]";
  console.log(`\n===== ${label} =====\n${body}\n`);
}

async function invokeTauriCommand(driver, command, payload = {}) {
  return driver.executeAsyncScript(
    async (tauriCommand, tauriPayload, done) => {
      try {
        const response = await window.__TAURI_INTERNALS__.invoke(tauriCommand, tauriPayload);
        done({ ok: true, response });
      } catch (error) {
        done({
          ok: false,
          error:
            typeof error === "string"
              ? error
              : error && typeof error === "object" && "message" in error
                ? String(error.message)
                : String(error),
        });
      }
    },
    command,
    payload,
  );
}

async function listLibraryRows(driver, limit = 200) {
  const response = await invokeTauriCommand(driver, "list_library_files", {
    query: { limit, offset: 0 },
  });
  if (!response.ok || !response.response) {
    throw new Error(`Could not read indexed Library rows: ${response.error ?? "unknown error"}`);
  }

  return response.response;
}

async function backendLibraryHasRows(driver, expectedRows = [], limit = 200) {
  const response = await listLibraryRows(driver, limit);
  const items = response.items ?? [];

  if (!expectedRows.length) {
    return (response.total ?? items.length) > 0;
  }

  const filenames = new Set(items.map((item) => item.filename));
  return expectedRows.every((rowText) => filenames.has(rowText));
}

async function ensureLibraryIndexed(driver, expectedRows = [], timeoutMs = 90000) {
  await clickButton(driver, "Library");
  await waitForText(driver, "Library");

  if (await backendLibraryHasRows(driver, expectedRows)) {
    return;
  }

  const started = await invokeTauriCommand(driver, "start_scan");
  if (!started.ok) {
    await dumpBodyText(driver, "scan-start-failed");
    throw new Error(`Could not start the installed Library scan: ${started.error}`);
  }

  const startedAt = Date.now();
  let lastStatus = started.response;

  while (Date.now() - startedAt < timeoutMs) {
    const status = await invokeTauriCommand(driver, "get_scan_status");
    if (!status.ok) {
      await dumpBodyText(driver, "scan-status-failed");
      throw new Error(`Could not read the installed Library scan state: ${status.error}`);
    }

    lastStatus = status.response;
    if (lastStatus?.state && lastStatus.state !== "running") {
      break;
    }

    await driver.sleep(500);
  }

  if (lastStatus?.state === "running") {
    await dumpBodyText(driver, "scan-timeout");
    throw new Error("Timed out waiting for the installed Library scan to finish.");
  }

  if (lastStatus?.state !== "succeeded") {
    await dumpBodyText(driver, "scan-failed");
    throw new Error(
      `The installed Library scan did not finish cleanly: ${lastStatus?.error ?? lastStatus?.state ?? "unknown state"}`,
    );
  }

  await driver.sleep(1200);
  const indexedAt = Date.now();
  while (Date.now() - indexedAt < 30000) {
    if (await backendLibraryHasRows(driver, expectedRows)) {
      await clickButton(driver, "Library");
      await waitForText(driver, "Library");
      await driver.sleep(1200);
      return;
    }
    await driver.sleep(300);
  }

  await dumpBodyText(driver, "library-indexed-but-ui-not-ready");
  throw new Error(
    `Timed out waiting for indexed Library rows: ${expectedRows.join(", ") || "any rows"}.`,
  );
}

async function fillInputByPlaceholder(driver, placeholder, value, timeoutMs = 30000) {
  const locator = By.css(`input[placeholder="${placeholder}"]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);
  const input = await driver.findElement(locator);
  await driver.wait(until.elementIsVisible(input), timeoutMs);
  await input.clear();
  await input.sendKeys(value);
}

async function selectFieldOptionByLabel(driver, labelText, optionText, timeoutMs = 30000) {
  const locator = By.xpath(
    `//label[.//span[normalize-space(.) = ${xpathString(labelText)}]]//select`,
  );
  await driver.wait(until.elementLocated(locator), timeoutMs);
  const select = await driver.findElement(locator);
  await driver.wait(until.elementIsVisible(select), timeoutMs);
  await driver.executeScript(
    "arguments[0].scrollIntoView({ block: 'center', inline: 'nearest' })",
    select,
  );
  try {
    await select.sendKeys(optionText);
  } catch {
    await select.click();
    const optionLocator = By.xpath(
      `//label[.//span[normalize-space(.) = ${xpathString(labelText)}]]//option[contains(normalize-space(.), ${xpathString(
        optionText,
      )})]`,
    );
    const option = await driver.findElement(optionLocator);
    await option.click();
  }
}

async function waitForQueueItem(driver, partialText, timeoutMs = 90000) {
  const startedAt = Date.now();
  let lastRetryAt = 0;

  while (Date.now() - startedAt < timeoutMs) {
    if (await hasVisibleText(driver, partialText)) {
      return;
    }

    if (
      Date.now() - startedAt > 15000 &&
      Date.now() - lastRetryAt > 12000 &&
      (await hasVisibleText(driver, "Check again"))
    ) {
      if (await tryClickVisibleButtonIfEnabled(driver, "Check again")) {
        lastRetryAt = Date.now();
      }
    }

    await driver.sleep(500);
  }

  await dumpBodyText(driver, `queue-timeout-${partialText}`);
  throw new Error(`Timed out waiting for inbox item "${partialText}".`);
}

async function clickSpecialQueueItem(driver) {
  const session = loadDriverSession();
  const namedItem =
    process.env.SIMSUITE_SMOKE_SPECIAL_ITEM_TEXT ??
    session?.fixture?.specialItem ??
    DEFAULT_SPECIAL_ITEM;
  if (namedItem) {
    try {
      await clickNamedQueueItemInLanes(driver, namedItem, SPECIAL_SETUP_LANE_LABELS);
      return;
    } catch {
    }
    try {
      await clickVisibleText(driver, namedItem);
      return;
    } catch {
    }
  }

  const locator = By.xpath(
    "//button[contains(normalize-space(.), 'Special setup') and contains(normalize-space(.), 'Ready')]",
  );
  await driver.wait(until.elementLocated(locator), 30000);
  const items = await driver.findElements(locator);
  for (const item of items) {
    if (await item.isDisplayed()) {
      await item.click();
      return;
    }
  }
  throw new Error("Could not find a visible special-mod queue item.");
}

async function clickBlockedQueueItem(driver) {
  const session = loadDriverSession();
  const namedItem =
    process.env.SIMSUITE_SMOKE_BLOCKED_ITEM_TEXT ??
    session?.fixture?.blockedItem ??
    DEFAULT_BLOCKED_ITEM;
  if (namedItem) {
    try {
      await clickNamedQueueItemInLanes(driver, namedItem, FOLLOW_UP_LANE_LABELS);
      return;
    } catch {
    }
    try {
      await clickVisibleText(driver, namedItem);
      return;
    } catch {
    }
  }

  const locator = By.xpath(
    "//button[contains(normalize-space(.), 'Blocked') and contains(normalize-space(.), 'Needs review')]",
  );
  await driver.wait(until.elementLocated(locator), 30000);
  const items = await driver.findElements(locator);
  for (const item of items) {
    if (await item.isDisplayed()) {
      await item.click();
      return;
    }
  }
  throw new Error("Could not find a visible blocked special-mod queue item.");
}

async function clickNamedQueueItem(driver, partialText, timeoutMs = 30000) {
  await clickQueueRow(driver, partialText, timeoutMs);
}

async function verifySameVersionItem(driver, partialText, laneLabels = DONE_LANE_LABELS) {
  try {
    await clickNamedQueueItemInLanes(driver, partialText, laneLabels);
    await waitForAnyText(driver, ["Installed and incoming match", "Already current"], 30000);
    await waitForAnyText(
      driver,
      [
        "Existing install",
        "Ready to install",
        "Open the calmer proof sheet",
        "Open proof",
      ],
      30000,
    );
    await waitForAnyText(driver, ["Reinstall guided copy", "Reinstall anyway"], 30000);
  } catch (error) {
    await dumpBodyText(driver, `same-version-failure-${partialText}`);
    throw error;
  }
}

async function verifyOlderVersionItem(driver, partialText, laneLabels = WAITING_ON_YOU_LANE_LABELS) {
  try {
    await clickNamedQueueItemInLanes(driver, partialText, laneLabels);
    await waitForAnyText(driver, ["Incoming pack looks older", "Older than installed"], 30000);
    await waitForAnyText(
      driver,
      [
        "Ignore",
        "Open proof",
        "Use this pack",
        "Use XML_Injector_Same_Test",
        "Use S4CL_Same_Test",
        "Use Lot51_Core_Same_Test",
        "Use Toolbox_Same_Test",
        "Use Smart_Core_Same_Test",
      ],
      30000,
    );
  } catch (error) {
    await dumpBodyText(driver, `older-version-failure-${partialText}`);
    throw error;
  }
}

async function verifyHomeWatchSummary(driver) {
  await clickButton(driver, "Home");
  await waitForText(driver, "Home");
  // In Desktop-First redesign, clicking Updates goes to Updates screen
  await clickButton(driver, "Updates");
  await waitForText(driver, "Updates", 30000);
  // Verify the Updates screen has the expected tabs
  await waitForAnyText(driver, ["Tracked", "Need source", "Needs review"], 30000);
}

async function verifyHomeWatchFocus(driver) {
  await clickButton(driver, "Home");
  await waitForText(driver, "Home");
  // Home rows now jump straight into the Updates setup lane when that module is visible.
  let launchedFromHome = true;
  try {
    await clickFirstVisibleButton(driver, ["Need source setup", "Pages to save", "Pages to set"], 6000);
  } catch {
    launchedFromHome = false;
    console.log("Home watch launch row is hidden in the current Home layout; opening Updates directly.");
    await clickButton(driver, "Updates");
  }

  await waitForText(driver, "Updates", 30000);
  if (!launchedFromHome) {
    await clickVisibleText(driver, "Need source");
  }

  await waitForAnyText(
    driver,
    [
      "Need source",
      "Source setup",
      "Nothing needs source setup right now.",
      "No files currently need watch setup.",
    ],
    30000,
  );
}

async function verifyUpdatesVersionWatch(driver) {
  try {
    // In Desktop-First redesign, watch features are in Updates screen
    await clickButton(driver, "Updates");
    await waitForText(driver, "Updates", 30000);
    
    // Click on Tracked tab
    await clickVisibleText(driver, "Tracked");
    await waitForText(driver, "Tracked", 30000);
    
    // Should see tracked items
    await waitForAnyText(driver, ["Needs attention", "Confirmed updates", "All tracked"], 30000);
  } catch (error) {
    await dumpBodyText(driver, "updates-version-watch-failure");
    throw error;
  }
}

async function verifyUpdatesWatchSaveClear(driver, genericWatchFile) {
  try {
    await ensureLibraryIndexed(driver, [genericWatchFile]);

    const setupRows = await invokeTauriCommand(driver, "list_library_watch_setup_items", {
      limit: 200,
    });
    if (!setupRows.ok || !setupRows.response) {
      throw new Error(
        `Could not load watch setup rows for Updates smoke: ${setupRows.error ?? "unknown error"}`,
      );
    }

    const setupItem =
      setupRows.response.items?.find((item) => item.filename === genericWatchFile) ?? null;
    if (!setupItem?.fileId) {
      throw new Error(
        `Smoke fixture did not produce a real Updates setup item for ${genericWatchFile}.`,
      );
    }

    // Navigate to Updates screen
    await clickButton(driver, "Updates");
    await waitForText(driver, "Updates", 30000);

    await clickButton(driver, "Need source");
    await waitForText(driver, genericWatchFile, 30000);
    await clickUpdatesRow(driver, genericWatchFile);
    await waitForAnyText(driver, ["Suggested source", "Set source"], 30000);

    await clickButton(driver, "Set source");
    await waitForText(driver, "Save source", 30000);
    await selectFieldOptionByLabel(driver, "Source type", "Creator page");
    await fillInputByPlaceholder(driver, "What page is this?", "Simstrouble");
    await fillInputByPlaceholder(driver, "https://...", "https://example.com/simstrouble");
    await clickButton(driver, "Save source");
    await waitForText(driver, "Source saved.", 30000);

    await clickButton(driver, "Needs review");
    await waitForText(driver, genericWatchFile, 30000);
    await clickUpdatesRow(driver, genericWatchFile);
    await waitForAnyText(
      driver,
      [
        "This creator page is saved as a reminder only.",
        "Reference only",
        "Reminder only",
      ],
      30000,
    );

    await clickButton(driver, "Edit source");
    await waitForAnyText(driver, ["Save source", "URL"], 30000);
    await clickButtonViaDom(driver, "Clear source");
    await waitForText(driver, "Source cleared.", 30000);

    await clickButton(driver, "Need source");
    await waitForText(driver, genericWatchFile, 30000);
    await ensureTextStaysHidden(driver, "This creator page is saved as a reminder only.", 5000);
  } catch (error) {
    await dumpBodyText(driver, "updates-watch-save-clear-failure");
    throw error;
  }
}

async function verifyLibraryWatchSaveClear(driver, genericWatchFile) {
  try {
    await ensureLibraryIndexed(driver, [genericWatchFile]);
    const libraryRows = await listLibraryRows(driver, 100);
    const genericRow =
      libraryRows.items?.find((item) => item.filename === genericWatchFile) ?? null;
    if (!genericRow?.id) {
      throw new Error(`Could not resolve a Library row id for ${genericWatchFile}.`);
    }
    const saveResult = await invokeTauriCommand(driver, "save_watch_source_for_file", {
      fileId: genericRow.id,
      sourceKind: "creator_page",
      sourceLabel: "Generic",
      sourceUrl: "https://example.com/creator-page",
    });
    if (!saveResult.ok || !saveResult.response) {
      throw new Error(
        `Could not save the generic watch source in desktop smoke: ${saveResult.error ?? "unknown error"}`,
      );
    }
    await waitForAnyText(driver, ["Review queue", "Watch review queue"], 30000);
    await waitForText(driver, genericWatchFile, 30000);
    await waitForAnyText(
      driver,
      [
        "This creator page is saved as a reminder only.",
        "This creator page is saved as a reminder only. Keep it if it helps, or replace it with an exact mod page.",
        "Reference only",
        "Reminder only",
      ],
      30000,
    );
    const clearResult = await invokeTauriCommand(driver, "clear_watch_source_for_file", {
      fileId: genericRow.id,
    });
    if (!clearResult.ok || !clearResult.response) {
      throw new Error(
        `Could not clear the generic watch source in desktop smoke: ${clearResult.error ?? "unknown error"}`,
      );
    }
    await waitForAnyText(driver, ["Ready to set up", "Setup suggestions"], 30000);
    await waitForText(driver, genericWatchFile, 30000);
    await ensureTextStaysHidden(driver, "This creator page is saved as a reminder only.", 5000);
  } catch (error) {
    await dumpBodyText(driver, "library-watch-save-clear-failure");
    throw error;
  }
}

async function run() {
  const appPath = resolveAppPath();
  const session = loadDriverSession();
  const fixtureSpecialItem = session?.fixture?.specialItem ?? DEFAULT_SPECIAL_ITEM;
  const fixtureBlockedItem = session?.fixture?.blockedItem ?? DEFAULT_BLOCKED_ITEM;
  const fixtureXmlSameItem = session?.fixture?.xmlSameItem ?? DEFAULT_XML_SAME_ITEM;
  const fixtureXmlOlderItem = session?.fixture?.xmlOlderItem ?? DEFAULT_XML_OLDER_ITEM;
  const fixtureS4clSameItem = session?.fixture?.s4clSameItem ?? DEFAULT_S4CL_SAME_ITEM;
  const fixtureS4clOlderItem = session?.fixture?.s4clOlderItem ?? DEFAULT_S4CL_OLDER_ITEM;
  const fixtureLot51SameItem = session?.fixture?.lot51SameItem ?? DEFAULT_LOT51_SAME_ITEM;
  const fixtureLot51OlderItem = session?.fixture?.lot51OlderItem ?? DEFAULT_LOT51_OLDER_ITEM;
  const fixtureToolboxSameItem = session?.fixture?.toolboxSameItem ?? DEFAULT_TOOLBOX_SAME_ITEM;
  const fixtureToolboxOlderItem = session?.fixture?.toolboxOlderItem ?? DEFAULT_TOOLBOX_OLDER_ITEM;
  const fixtureSmartCoreSameItem =
    session?.fixture?.smartCoreSameItem ?? DEFAULT_SMART_CORE_SAME_ITEM;
  const fixtureSmartCoreOlderItem =
    session?.fixture?.smartCoreOlderItem ?? DEFAULT_SMART_CORE_OLDER_ITEM;
  const fixtureGenericWatchFile =
    session?.fixture?.genericWatchFile ?? DEFAULT_GENERIC_WATCH_FILE;
  const capabilities = new Capabilities();
  capabilities.setBrowserName("wry");
  capabilities.set("tauri:options", {
    application: appPath,
  });

  const driver = await new Builder()
    .usingServer(WEBDRIVER_URL)
    .withCapabilities(capabilities)
    .build();

  try {
    await waitForAnyText(driver, ["HOME", "INBOX", "SETTINGS"], 60000);

    await clickButton(driver, "Inbox");
    await waitForText(driver, "Inbox");
    await waitForAnyText(
      driver,
      [
        "Downloads Inbox",
        "Checking your Downloads inbox...",
        "Inbox is the plumbob checkpoint before anything reaches Mods or Tray.",
      ],
      60000,
    );
    await waitForQueueItemInLane(driver, fixtureSpecialItem, SPECIAL_SETUP_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureBlockedItem, FOLLOW_UP_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureXmlSameItem, DONE_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureXmlOlderItem, WAITING_ON_YOU_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureS4clSameItem, DONE_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureS4clOlderItem, WAITING_ON_YOU_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureLot51SameItem, DONE_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureLot51OlderItem, WAITING_ON_YOU_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureToolboxSameItem, DONE_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureToolboxOlderItem, WAITING_ON_YOU_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureSmartCoreSameItem, DONE_LANE_LABELS, 90000);
    await waitForQueueItemInLane(driver, fixtureSmartCoreOlderItem, WAITING_ON_YOU_LANE_LABELS, 90000);

    await clickSpecialQueueItem(driver);
    await waitForAnyText(driver, ["Ready to install", "Guided update is ready"], 30000);
    await waitForAnyText(
      driver,
      ["Existing install", "Apply guided update", "Apply guided install", "Update safely"],
      30000,
    );

    for (const item of [
      fixtureXmlSameItem,
      fixtureS4clSameItem,
      fixtureLot51SameItem,
      fixtureToolboxSameItem,
      fixtureSmartCoreSameItem,
    ]) {
      await verifySameVersionItem(driver, item, DONE_LANE_LABELS);
    }

    for (const item of [
      fixtureXmlOlderItem,
      fixtureS4clOlderItem,
      fixtureLot51OlderItem,
      fixtureToolboxOlderItem,
      fixtureSmartCoreOlderItem,
    ]) {
      await verifyOlderVersionItem(driver, item, WAITING_ON_YOU_LANE_LABELS);
    }

    if (INCLUDE_APPLY) {
      if (process.env.SIMSUITE_ALLOW_APPLY_SMOKE !== "1") {
        throw new Error(
          "Apply smoke is blocked by default. Run it only against isolated test data with SIMSUITE_ALLOW_APPLY_SMOKE=1.",
        );
      }
      await clickSpecialQueueItem(driver);
      try {
        await waitForAnyText(driver, DEFAULT_APPLY_LABELS, 30000);
      } catch (error) {
        await dumpBodyText(driver, "apply-step-timeout");
        throw error;
      }
      const applyLabels = process.env.SIMSUITE_SMOKE_APPLY_LABEL
        ? [process.env.SIMSUITE_SMOKE_APPLY_LABEL]
        : DEFAULT_APPLY_LABELS;
      await clickFirstVisibleButton(driver, applyLabels);
      await acceptAlertIfPresent(driver);
      await waitForAnyText(
        driver,
        [
          "matches the version that is already installed",
          "a fuller mc command center pack from this family is already installed",
        ],
        60000,
      );
      await waitForQueueItemInLane(driver, fixtureBlockedItem, FOLLOW_UP_LANE_LABELS, 30000);
      await clickBlockedQueueItem(driver);
      await waitForAnyText(
        driver,
        [
          "A fuller MC Command Center pack from this family is already installed.",
          "Ignore this leftover MC Command Center download",
          "A fuller family pack is already installed",
        ],
        30000,
      );
      await ensureTextStaysHidden(
        driver,
        "A fuller MC Command Center pack is already in Inbox as",
        2000,
      );
    } else {
      await clickBlockedQueueItem(driver);
      await waitForAnyText(
        driver,
        ["Why this was blocked", "Blocked details are not ready yet", "Blocked"],
        30000,
      );

      await clickButton(driver, "Refresh");
      await waitForText(driver, "Inbox");
    }

    await verifyHomeWatchSummary(driver);
    await verifyHomeWatchFocus(driver);
    await verifyUpdatesVersionWatch(driver);
    await verifyUpdatesWatchSaveClear(driver, fixtureGenericWatchFile);

    console.log(`Desktop smoke passed against ${appPath}`);
  } finally {
    await driver.quit();
  }
}

run().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
