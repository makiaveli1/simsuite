import { Builder, By, Capabilities, until } from "selenium-webdriver";
import fs from "node:fs";
import path from "node:path";

const WEBDRIVER_URL = process.env.SIMSUITE_WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const DEFAULT_APP_PATHS = [
  path.resolve("src-tauri", "target", "debug", "simsuite.exe"),
  path.resolve("src-tauri", "target", "debug", "SimSuite.exe"),
  path.resolve("src-tauri", "target", "release", "simsuite.exe"),
  path.resolve("src-tauri", "target", "release", "SimSuite.exe"),
];
const INCLUDE_APPLY = process.argv.includes("--include-apply");

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

async function waitForText(driver, text, timeoutMs = 30000) {
  const locator = By.xpath(`//*[contains(normalize-space(.), ${xpathString(text)})]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);
}

async function waitForAnyText(driver, texts, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    for (const text of texts) {
      const elements = await driver.findElements(
        By.xpath(`//*[contains(normalize-space(.), ${xpathString(text)})]`),
      );
      for (const element of elements) {
        if (await element.isDisplayed()) {
          return text;
        }
      }
    }
    await driver.sleep(300);
  }
  throw new Error(`Timed out waiting for one of: ${texts.join(", ")}`);
}

async function clickSpecialQueueItem(driver) {
  const namedItem = process.env.SIMSUITE_SMOKE_SPECIAL_ITEM_TEXT;
  if (namedItem) {
    await clickButton(driver, namedItem);
    return;
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
  const namedItem = process.env.SIMSUITE_SMOKE_BLOCKED_ITEM_TEXT;
  if (namedItem) {
    await clickButton(driver, namedItem);
    return;
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
    await driver.wait(until.titleIs("SimSuite"), 30000);

    await clickButton(driver, "Inbox");
    await waitForText(driver, "Inbox");
    await waitForText(driver, "Inbox station");

    await clickSpecialQueueItem(driver);
    await waitForText(driver, "Version check");
    await waitForText(driver, "Installed");
    await waitForText(driver, "Incoming");
    await waitForText(driver, "Compare");
    await waitForAnyText(driver, ["Incoming evidence", "Main check"], 30000);

    await clickBlockedQueueItem(driver);
    await waitForAnyText(
      driver,
      ["Why this was blocked", "Blocked details are not ready yet", "Blocked"],
      30000,
    );

    await clickButton(driver, "Refresh");
    await waitForText(driver, "Inbox");

    if (INCLUDE_APPLY) {
      if (process.env.SIMSUITE_ALLOW_APPLY_SMOKE !== "1") {
        throw new Error(
          "Apply smoke is blocked by default. Run it only against isolated test data with SIMSUITE_ALLOW_APPLY_SMOKE=1.",
        );
      }
      await clickSpecialQueueItem(driver);
      await waitForAnyText(driver, ["Update safely", "Install safely"], 30000);
      const applyLabel = process.env.SIMSUITE_SMOKE_APPLY_LABEL ?? "Update safely";
      await clickButton(driver, applyLabel);
      await waitForAnyText(driver, ["Done", "Installed safely", "Updated safely"], 30000);
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
