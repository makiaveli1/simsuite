import { chromium } from '/mnt/c/Users/likwi/OneDrive/Desktop/PROJS/SimSort/node_modules/playwright/index.mjs';

const browser = await chromium.launch({ headless: true, args: ['--no-sandbox'] });
const page = await browser.newPage();
await page.goto('http://127.0.0.1:1420', { waitUntil: 'networkidle', timeout: 15000 }).catch(() => {});
await page.waitForTimeout(3000);

const title = await page.title();
const html = await page.content();
const hasFolderToggle = html.includes('folder') || html.includes('Folder');
const hasLooseFiles = html.includes('loose') || html.includes('Loose');

console.log('Title:', title);
console.log('Has folder content:', hasFolderToggle);
console.log('Has loose-files content:', hasLooseFiles);

await browser.close();
