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

async function clickVisibleButton(driver, partialText, timeoutMs = 30000) {
  const locator = By.xpath(`//button[contains(normalize-space(.), ${xpathString(partialText)})]`);
  await driver.wait(until.elementLocated(locator), timeoutMs);

  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const buttons = await driver.findElements(locator);
    for (const button of buttons) {
      if ((await button.isDisplayed()) && (await button.isEnabled())) {
        try {
          await button.click();
        } catch {
          await driver.executeScript("arguments[0].click()", button);
        }
        return;
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
      if (await element.isDisplayed()) {
        return await element.getText();
      }
    }
    await sleep(driver, 250);
  }
  throw new Error(`Timed out waiting for visible selector ${selector}`);
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

async function ensureLibraryIndexed(driver) {
  await clickVisibleButton(driver, "Library");
  await waitForAnyText(driver, ["Library", "MOD OR FILE", "MOD OR FILES"], 30000);

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
    const items = await ensureLibraryIndexed(driver);
    const targets = pickTargets(items, session);
    summary.targets = {
      mccc: targets.mccc.filename,
      generic: targets.generic?.filename ?? null,
    };

    await clickVisibleButton(driver, "Library");
    await waitForHash(driver, "#library", 30000).catch(() => null);
    summary.mcccRowNeedle = await openRow(driver, targets.mccc);
    summary.mcccCompactText = await waitForVisibleCssText(driver, ".action-preflight-card", 30000);
    const selectedShot = path.join(runDir, "01-library-selected-mccc.png");
    await takeScreenshot(driver, selectedShot);
    summary.screenshots.push(selectedShot);

    await clickVisibleButton(driver, "Review cautions");
    summary.mcccDetailText = await waitForVisibleCssText(driver, ".action-preflight-detail-block", 30000);
    const detailShot = path.join(runDir, "02-library-preflight-detail-mccc.png");
    await takeScreenshot(driver, detailShot);
    summary.screenshots.push(detailShot);

    await clickVisibleButton(driver, "Open Needs Review");
    summary.reviewHash = await waitForHash(driver, "#review", 30000);
    summary.reviewBodyHasMccc = /MCCC|MCCommandCenter/i.test(await getBodyText(driver));
    const reviewShot = path.join(runDir, "03-review-route.png");
    await takeScreenshot(driver, reviewShot);
    summary.screenshots.push(reviewShot);

    summary.finishedAt = new Date().toISOString();
    summary.ok = true;
  } catch (error) {
    summary.finishedAt = new Date().toISOString();
    summary.ok = false;
    summary.error = error instanceof Error ? error.message : String(error);
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
