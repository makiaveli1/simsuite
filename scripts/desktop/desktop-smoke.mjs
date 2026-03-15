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
const DEFAULT_GENERIC_WATCH_FILE = "Generic_Watch_Mod_v1.0.package";
const DEFAULT_APP_PATHS = [
  path.resolve("src-tauri", "target", "debug", "simsuite.exe"),
  path.resolve("src-tauri", "target", "debug", "SimSuite.exe"),
  path.resolve("src-tauri", "target", "release", "simsuite.exe"),
  path.resolve("src-tauri", "target", "release", "SimSuite.exe"),
];
const INCLUDE_APPLY = process.argv.includes("--include-apply");
const DEFAULT_SESSION_FILE = path.resolve("output", "desktop", "tauri-driver-session.json");
const DEFAULT_APPLY_LABELS = ["Apply guided update", "Apply guided install", "Update safely", "Install safely"];

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
          await driver.executeScript("arguments[0].click()", row);
          return;
        }
      }
    }
    await driver.sleep(250);
  }

  throw new Error(`Could not find a visible Library row for "${filename}".`);
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

async function clickQueueRow(driver, partialText, timeoutMs = 30000) {
  const row = await findVisibleQueueRow(driver, partialText, timeoutMs);
  await driver.wait(until.elementIsVisible(row), timeoutMs);
  await driver.wait(until.elementIsEnabled(row), timeoutMs);
  await row.click();
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

async function ensureLibraryIndexed(driver, expectedRows = [], timeoutMs = 90000) {
  await clickButton(driver, "Library");
  await waitForText(driver, "Library");

  for (const rowText of expectedRows) {
    if (await hasVisibleText(driver, rowText)) {
      return;
    }
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
  await clickButton(driver, "Library");
  await waitForText(driver, "Library");

  if (expectedRows.length) {
    await waitForAnyText(driver, expectedRows, 30000);
  }
}

async function fillInputByPlaceholder(driver, placeholder, value, timeoutMs = 30000) {
  const locator = By.css(`input[placeholder="${placeholder}"]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);
  const input = await driver.findElement(locator);
  await driver.wait(until.elementIsVisible(input), timeoutMs);
  await input.clear();
  await input.sendKeys(value);
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
      await clickQueueRow(driver, namedItem);
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
      await clickQueueRow(driver, namedItem);
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

async function verifySameVersionItem(driver, partialText) {
  try {
    await clickNamedQueueItem(driver, partialText);
    await waitForText(driver, "Versions");
    await waitForAnyText(driver, ["Installed and incoming match", "Already current"], 30000);
    await waitForAnyText(
      driver,
      [
        "Inside the mod files",
        "Matching file fingerprint",
        "Matching file fingerprints confirmed the same version",
        "Download name",
        "Installed files",
      ],
      30000,
    );
    await waitForAnyText(driver, ["Reinstall guided copy", "Reinstall anyway"], 30000);
  } catch (error) {
    await dumpBodyText(driver, `same-version-failure-${partialText}`);
    throw error;
  }
}

async function verifyOlderVersionItem(driver, partialText) {
  try {
    await clickNamedQueueItem(driver, partialText);
    await waitForText(driver, "Versions");
    await waitForAnyText(driver, ["Incoming pack looks older", "Older than installed"], 30000);
    await waitForAnyText(driver, ["Installed", "Incoming", "Compare"], 30000);
  } catch (error) {
    await dumpBodyText(driver, `older-version-failure-${partialText}`);
    throw error;
  }
}

async function verifyHomeWatchSummary(driver) {
  await clickButton(driver, "Home");
  await waitForText(driver, "Home");
  await waitForAnyText(driver, ["Exact updates", "Updates ready"], 30000);
  await waitForAnyText(driver, ["Possible updates", "Watch review"], 30000);
}

async function verifyLibraryVersionWatch(driver) {
  await ensureLibraryIndexed(driver, ["S4CL.ts4script", "mc_cmd_center.ts4script"]);
  await waitForText(driver, "Needs attention");
  await clickButton(driver, "All tracked");
  await waitForAnyText(driver, ["mc_cmd_center.ts4script", "S4CL.ts4script"], 30000);
  try {
    await clickLibraryRow(driver, "S4CL.ts4script", 30000);
  } catch {
    await clickLibraryRow(driver, "mc_cmd_center.ts4script", 30000);
  }
  await waitForAnyText(driver, ["Installed version", "Version and updates"], 30000);
  await waitForText(driver, "Confidence");
  await waitForText(driver, "Watch status");
  await clickButton(driver, "Check now");
  await waitForAnyText(
    driver,
    ["Watch result refreshed.", "Looks current", "Exact update available"],
    30000,
  );
}

async function verifyLibraryWatchSaveClear(driver, genericWatchFile) {
  await ensureLibraryIndexed(driver, [genericWatchFile]);
  await clickLibraryRow(driver, genericWatchFile, 30000);
  await waitForAnyText(driver, ["Installed version", "Version and updates"], 30000);
  await waitForText(driver, "Watch status");
  await clickButton(driver, "Add watch source");
  await fillInputByPlaceholder(driver, "https://example.com/mod-page", "https://example.com/watch-test");
  await clickButton(driver, "Save watch");
  await waitForAnyText(
    driver,
    ["Watch source saved.", "This page is saved as a reference, but SimSuite cannot check it automatically yet."],
    30000,
  );
  await waitForText(driver, "Exact mod page");
  await waitForText(
    driver,
    "This page is saved as a reference, but SimSuite cannot check it automatically yet.",
  );
  await clickFirstVisibleButton(driver, ["Clear watch source", "Stop watching"], 30000);
  await waitForAnyText(
    driver,
    ["Watch source cleared.", "No approved watch source is saved for this installed content yet."],
    30000,
  );
  await waitForText(driver, "Add watch source");
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
    await waitForQueueItem(driver, fixtureSpecialItem, 90000);
    await waitForQueueItem(driver, fixtureBlockedItem, 90000);
    await waitForQueueItem(driver, fixtureXmlSameItem, 90000);
    await waitForQueueItem(driver, fixtureXmlOlderItem, 90000);
    await waitForQueueItem(driver, fixtureS4clSameItem, 90000);
    await waitForQueueItem(driver, fixtureS4clOlderItem, 90000);
    await waitForQueueItem(driver, fixtureLot51SameItem, 90000);
    await waitForQueueItem(driver, fixtureLot51OlderItem, 90000);
    await waitForQueueItem(driver, fixtureToolboxSameItem, 90000);
    await waitForQueueItem(driver, fixtureToolboxOlderItem, 90000);
    await waitForQueueItem(driver, fixtureSmartCoreSameItem, 90000);
    await waitForQueueItem(driver, fixtureSmartCoreOlderItem, 90000);

    await clickSpecialQueueItem(driver);
    await waitForText(driver, "Versions");
    await waitForText(driver, "Installed");
    await waitForText(driver, "Incoming");
    await waitForText(driver, "Compare");
    await waitForAnyText(driver, ["Incoming evidence", "Main check"], 30000);

    for (const item of [
      fixtureXmlSameItem,
      fixtureS4clSameItem,
      fixtureLot51SameItem,
      fixtureToolboxSameItem,
      fixtureSmartCoreSameItem,
    ]) {
      await verifySameVersionItem(driver, item);
    }

    for (const item of [
      fixtureXmlOlderItem,
      fixtureS4clOlderItem,
      fixtureLot51OlderItem,
      fixtureToolboxOlderItem,
      fixtureSmartCoreOlderItem,
    ]) {
      await verifyOlderVersionItem(driver, item);
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
      await waitForQueueItem(driver, fixtureBlockedItem, 30000);
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
    await verifyLibraryVersionWatch(driver);
    await verifyLibraryWatchSaveClear(driver, fixtureGenericWatchFile);

    console.log(`Desktop smoke passed against ${appPath}`);
  } finally {
    await driver.quit();
  }
}

run().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
