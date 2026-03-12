import { Builder, By, Capabilities, until } from "selenium-webdriver";
import fs from "node:fs";
import path from "node:path";

const WEBDRIVER_URL = process.env.SIMSUITE_WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const DEFAULT_SPECIAL_ITEM = "MCCC_Update_Test";
const DEFAULT_BLOCKED_ITEM = "MCCC_Partial_Blocked_Test";
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
  await button.click();
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
  await element.click();
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

async function run() {
  const appPath = resolveAppPath();
  const session = loadDriverSession();
  const fixtureSpecialItem = session?.fixture?.specialItem ?? DEFAULT_SPECIAL_ITEM;
  const fixtureBlockedItem = session?.fixture?.blockedItem ?? DEFAULT_BLOCKED_ITEM;
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

    await clickSpecialQueueItem(driver);
    await waitForText(driver, "Versions");
    await waitForText(driver, "Installed");
    await waitForText(driver, "Incoming");
    await waitForText(driver, "Compare");
    await waitForAnyText(driver, ["Incoming evidence", "Main check"], 30000);

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
        ["already installed", "Ignore this leftover", "no longer needs to lead the install"],
        30000,
      );
      await ensureTextStaysHidden(driver, "already in Inbox as", 2000);
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

    console.log(`Desktop smoke passed against ${appPath}`);
  } finally {
    await driver.quit();
  }
}

run().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
