/*
 * Draw the social card from the built site, so the card cannot drift from it.
 *
 * It reads the real stylesheet and the real drawing out of `dist`, lays them
 * out at 1200x630, and writes `public/assets/img/og-image.png`. Nothing here
 * is a mock-up of the page: the type, the colours and the glyph are the ones
 * the page itself serves.
 *
 *     cd site && bun run build && node tools/build-og-image.mjs
 *
 * Needs Playwright's Chromium, which the repository already has for its
 * end-to-end tests. Development tool: CI never runs it, and the card it
 * writes is committed.
 */

import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const site = new URL('..', import.meta.url);
const dist = new URL('dist/', site);
const playwright = new URL(
  '../node_modules/.pnpm/playwright@1.62.1/node_modules/playwright/index.mjs',
  site,
);

const page = await readFile(new URL('index.html', dist), 'utf8');
const stylesheet = page.match(/href="(\/_astro\/[^"]+\.css)"/)?.[1];
if (!stylesheet) throw new Error('no stylesheet in dist/index.html; build the site first');
const css = await readFile(new URL(`.${stylesheet}`, dist), 'utf8');
const drawing = page.match(/<svg[^>]*class="plate__canvas"[\s\S]*?<\/svg>/)?.[0];
if (!drawing) throw new Error('no drawing in dist/index.html');
const logo = await readFile(new URL('public/assets/img/favicon.svg', site), 'utf8');

const { release } = await import(new URL('src/data/release.ts', site).href).catch(() => ({
  release: null,
}));
const version = release?.version ?? JSON.parse(
  (await readFile(new URL('src/data/release.ts', site), 'utf8')).replace(
    /^[\s\S]*?export const release: Release = /,
    '',
  ).replace(/;\s*$/, ''),
).version;

const card = `<!doctype html>
<html lang="en" data-theme="light">
  <head>
    <meta charset="utf-8" />
    <style>${css}</style>
    <style>
      body { width: 1200px; height: 630px; overflow: hidden; display: grid;
             grid-template-columns: 1fr 1fr; align-items: center; }
      .card { padding: 0 0 0 64px; display: grid; gap: 22px; }
      .card h1 { font-size: 64px; font-weight: 860; letter-spacing: -0.03em; line-height: 0.98; }
      .card p { font-size: 22px; color: var(--ink-2); max-width: 22ch; }
      .card .wordmark { font-size: 24px; gap: 12px; }
      .card .wordmark svg { width: 34px; height: 34px; }
      .stage { height: 630px; border-left: 1px solid var(--rule); background: var(--plane);
               display: flex; align-items: center; }
      .stage svg { width: 100%; max-height: 560px; }
    </style>
  </head>
  <body>
    <div class="card">
      <span class="wordmark">${logo}FontCarver</span>
      <h1>Draw outlines,<br />edit pixels,<br />ship OpenType.</h1>
      <p>An open-source desktop font editor with a Rust kernel.</p>
      <p class="mono" style="color: var(--blue)">fontcarver.com &nbsp; ${version}</p>
    </div>
    <div class="stage">${drawing}</div>
  </body>
</html>
`;

const directory = await mkdtemp(join(tmpdir(), 'fc-og-'));
const file = join(directory, 'card.html');
await writeFile(file, card, 'utf8');

const { chromium } = await import(playwright.href);
const browser = await chromium.launch();
const tab = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 });
await tab.goto(pathToFileURL(file).href, { waitUntil: 'networkidle' });
await tab.waitForTimeout(300);
const target = new URL('public/assets/img/og-image.png', site);
await tab.screenshot({ path: target.pathname });
await browser.close();
console.log(`wrote ${target.pathname}`);
