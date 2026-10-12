# FontCarver website design

Status: approved design, not yet implemented. Date: 2026-09-24.

The public site for FontCarver at `https://fontcarver.com`: a static site in
`site/`, deployed to Cloudflare Pages by GitHub Actions. This document is
binding for that site the way `2026-09-03-fontcarver-design.md` is binding for
the product.

## 1. Purpose

FontCarver is a cross-platform desktop font editor, not a browser extension.
The site exists to do three things, in this order:

1. **Show what the editor is** to a type designer who has thirty seconds and
   has already used FontForge, Glyphs or FontLab.
2. **Get the right build onto their machine** — Linux, macOS, Windows — or,
   until there is a release, tell them the truth and show them how to build
   from source.
3. **Explain how to use it** well enough that the first session is not a
   bounce.

Audience, in priority order: independent type designers; font engineers
shipping OpenType layout; CJK font makers; free-software users looking for a
FontForge successor.

Success criteria:

- A visitor who reads only the first screen can state what the product is and
  who it is for.
- Every claim on the page is one the repository can currently back. No claimed
  release, no claimed support that `docs/STATUS.md` does not record.
- First screen renders in under one second on a cold cache over a normal
  connection, with no layout shift and no external network dependency.
- The site works with JavaScript disabled, at 320 px wide, in light and dark,
  and with a keyboard only.

## 2. Constraints

| Constraint | Consequence for the site |
|---|---|
| There is no release; packaging is untested (`docs/STATUS.md`, "Release" block not started) | The download section is built in full but ships in a **pending** state: real structure, honest labels, build-from-source path. No dead links presented as live downloads. |
| `docs/screenshots/*.png` still shows the old `FontWorkshop` branding | No product screenshots ship. Interface imagery is authored as inline SVG. A real screenshot gallery is designed as a slot to fill later. |
| The user base includes mainland China | No third-party CDN, no Google Fonts, no analytics, no external requests of any kind at runtime. Fonts are self-hosted, subset woff2. |
| `CONVENTIONS.md` licensing rules | Every bundled font and every extracted glyph outline is permissively licensed (OFL) and recorded in `THIRD_PARTY.md` and `fixtures/README.md` conventions. |
| Repository has no site toolchain | `site/` is hand-written HTML/CSS/JS with no build step, no `package.json`, no `node_modules`. CI verifies and uploads the directory as it stands. |
| "Do not publish yet" | The deploy job is gated: it runs only on `push` to `main` **and** only when the Cloudflare secrets exist. It is a no-op until both are true. |

## 3. Visual system

The site and the application are one product and share one visual language.
The site's `assets/css/tokens.css` is derived from `app/src/styles/tokens.css`
and `docs/design/editor-ui.md` §2, restated for the web rather than imported,
because the two ship separately.

**Colour.** The application's token names and values, verbatim where they
apply: `--color-app`, `--color-surface`, `--color-border`, `--color-text`,
`--color-text-muted`, `--color-accent` (`#3a6ea5` light / `#5f92cc` dark), and
the whole `--cv-*` canvas family, which the hero needs in order to look like
the real editor: `--cv-node-corner` (blue square), `--cv-node-smooth` (green
circle), `--cv-handle`, `--cv-metric`, `--cv-metric-label`, `--cv-guide-global`
(red). One accent. Semantic colours never decorative. Every colour has a light
and a dark value.

The site adds a small number of tokens the application has no use for: a
display type scale, a page measure, a section rhythm, and one very dark
`--color-stage` used only behind the hero and the format wall.

**Theme.** `data-theme` on `<html>`, mirroring the application. Default
follows `prefers-color-scheme`; a header toggle overrides and persists in
`localStorage` inside `try`/`catch`. The hero is dark in both themes — it is a
canvas, and the editor's canvas is where the product lives.

**Type.** Three self-hosted OFL families, subset to the characters the site
actually uses:

- Display: **Instrument Serif** — headlines only, large sizes, high contrast.
  It gives the page typographic authority, which is the one thing a font
  editor's site cannot be without.
- Text and UI: **IBM Plex Sans** — body, navigation, buttons, cards.
- Mono: **IBM Plex Mono** — glyph names, code points, font units, format
  lists, shell commands, checksums.

Chinese pages use the system CJK stack (`"PingFang SC", "Microsoft YaHei",
"Noto Sans SC"`), as `docs/design/editor-ui.md` §2.2 prescribes for the
application; no CJK webfont is shipped. `font-display: swap`, `preload` for
the two faces used above the fold, `tabular-nums` on every number.

**Shape, elevation, motion.** Radii 4/6/3. Borders, not shadows, for
structure; the single `--shadow-popover` value only for genuinely floating
elements. Spacing on the 4/8/12/16/24/32 scale. Durations 80/120/180 ms with
`cubic-bezier(.2,.8,.2,1)`. Under `prefers-reduced-motion: reduce` every
transition and the hero's idle animation stop; the hero stays draggable.

**Grid.** A 12-column grid with a 1120 px maximum and a 16 px gutter at phone
width. Prose measure caps at 68 characters. A faint baseline-grid texture —
the editor's canvas ruler, at low contrast — backs the hero and the format
wall and nothing else.

**Icons.** Inline SVG, 1.5 px stroke, `currentColor`, 16 px in text and 20 px
in cards, drawn in the Lucide idiom. No emoji, no icon font.

## 4. Information architecture

One page per language, plus a fourth-level anchor structure. English at `/`,
Chinese at `/zh/`. Content is maintained in parallel; the Chinese page is a
translation of the same structure, not a subset.

1. **Header** — wordmark, anchor links (Features, Formats, Guide, Download),
   GitHub link, theme toggle, language switch. Sticky, 1 px bottom border,
   collapses to a disclosure button under 720 px.
2. **Hero** — the live glyph canvas (§5), the positioning line, two buttons
   (Download / View on GitHub), and a one-line honest status chip linking to
   `docs/STATUS.md`.
3. **Capabilities** — four cards, each a claim the repository backs:
   curves *and* pixels; Latin *and* CJK; the whole OpenType layout stack
   authored as FEA or through the UI; lossless preservation of everything the
   model did not touch. Each card carries a small SVG diagram, not a stock
   illustration.
4. **Format wall** — every supported format as a dense monospace grid of
   cells, deliberately echoing the application's glyph grid. Cells are grouped
   (outline, bitmap, source, layout, metrics) and carry a read/write marker.
   The grouping and markers come from `README.md` and the acceptance records;
   nothing is listed that is not recorded there.
5. **Guide** — "使用方法": four steps, each with a short instruction, one SVG
   figure and the relevant keyboard shortcuts as chips: open a font → edit
   outlines → kerning and OpenType features → export. A fifth panel covers
   the CLI. Steps are a tab set with anchor-linked, JS-free fallback (each
   panel is a section; the tab behaviour is progressive enhancement).
6. **Download** — three platform cards (§6).
7. **Roadmap and honesty** — the P0–P5 phase strip with current state, the
   known open defect, and a link to `docs/STATUS.md` and the acceptance
   records. This section is a feature, not an apology.
8. **Footer** — licence (`MIT OR Apache-2.0`), repository, documentation, the
   FontForge relationship statement from `README.md`, and the trademark note.

## 5. The hero: a live glyph canvas

The first screen is a working fragment of the editor, not a picture of one.

- An SVG canvas in **font units**, y-up, with the coordinate flip applied by a
  single `transform`, exactly as the application treats font space.
- Horizontal metric lines — descender, baseline, x-height, cap height,
  ascender — each labelled with its name and its value in font units, in mono,
  `--cv-metric` / `--cv-metric-label`. Vertical origin and advance lines with
  LSB/RSB markers.
- One glyph outline drawn with `--cv-ink`, its on-curve points as
  `--cv-node-corner` squares and `--cv-node-smooth` circles, off-curve points
  as `--cv-handle` dots on `--cv-handle` handle lines. Sizes and shapes follow
  `docs/design/editor-ui.md` §5.
- **The points are draggable.** Pointer and keyboard (arrow keys move the
  focused point by 1 unit, shift by 10). The path recomputes from the control
  points; smooth points keep their handles collinear, the way the editor does.
  A HUD in the corner shows the dragged point's coordinates in font units with
  `tabular-nums`. This is the entire interaction — no undo stack, no tools.
- A **script switch** — `g` / `永` — cross-fades the outline to a CJK glyph,
  whose components are outlined in `--cv-component` to show that components
  are a first-class part of the model.
- A **vector ↔ bitmap slider** rasterises the current glyph into an 8, 12 or
  16 px strike drawn as square cells over the outline, demonstrating that
  bitmap strikes live in the same document.

Outline data is real: paths are extracted at authoring time from OFL fonts
with `fontTools` by `site/tools/extract-outlines.py`, and committed as a small
static data file. The script is a development tool, is not deployed, and does
not run in CI. Source fonts and their licences are recorded in
`THIRD_PARTY.md`.

Without JavaScript the hero renders as a complete, correct static figure: the
outline, the points, the metric lines and the labels are all in the markup.
Only dragging, the script switch and the bitmap slider are enhancements. Total
hero JavaScript budget: 10 KB uncompressed.

## 6. Downloads

Three cards — Linux, macOS, Windows — each listing the artefacts the Tauri
bundler will produce: `.AppImage` / `.deb` / `.rpm`; `.dmg` for Apple silicon
and Intel; `.msi` / `.exe`. Each row has a slot for size and a SHA-256
checksum in mono.

The visitor's platform is detected from `navigator.userAgentData` with a
`navigator.platform` fallback and that card is promoted; detection failure
shows all three equally. Detection never hides a platform.

Until the first release, every card is in the **pending** state: the artefact
rows are visible but inert and marked "not yet released", the card's primary
action is "Build from source", and the section leads with one honest sentence
and a link to `docs/STATUS.md`. A build-from-source panel gives the exact
commands from `README.md` including the Debian/Ubuntu system dependencies.

Going live later is a content change in one place: the artefact rows read
their URLs from a single `downloads` data block at the top of the page script,
and the pending state is a single attribute on the section.

## 7. File layout

```
site/
  index.html                  English page
  zh/index.html               Chinese page
  404.html
  assets/
    css/tokens.css            design tokens, light and dark
    css/site.css              layout and components
    js/hero.js                the live canvas (progressive enhancement)
    js/site.js                theme toggle, platform detection, tabs, nav
    js/outlines.js            extracted outline data (generated, committed)
    fonts/*.woff2             subset OFL faces
    img/logo.svg, favicon.svg, og-image.png
  tools/extract-outlines.py   development only, not deployed
  _headers                    Cloudflare Pages: CSP, caching, security headers
  _redirects
  robots.txt
  sitemap.xml
  README.md                   how to work on the site, how to deploy it
```

No file in `site/` imports from `app/`. Tokens are duplicated deliberately and
`site/README.md` records that they must be re-synced when
`app/src/styles/tokens.css` changes.

`_headers` sets a strict `Content-Security-Policy` (`default-src 'self'`, no
`unsafe-inline`, so every style and script is a file and the hero applies its
dynamic geometry through SVG attributes and the CSSOM, never through a `style`
attribute in markup), `X-Content-Type-Options`,
`Referrer-Policy: strict-origin-when-cross-origin`, a one-year immutable cache
for `/assets/*` and a short cache for HTML.

## 8. Continuous integration and deployment

A second workflow, `.github/workflows/site.yml`, independent of `ci.yml`, so
site work never waits on the 45-minute Rust job.

```
on:
  push:        paths: ['site/**', '.github/workflows/site.yml']
  pull_request: paths: ['site/**', '.github/workflows/site.yml']
  workflow_dispatch:
```

**Job `verify`** — runs everywhere, no secrets, no npm:

- `python3 scripts/check_site.py`, a dependency-free checker that fails on: a
  broken internal link or asset reference; an `<img>` without `alt`; a missing
  `<title>`, `description`, canonical or Open Graph tag; a page whose
  `hreflang` pair is incomplete; a stylesheet or script referenced but absent;
  a file over its size budget; any absolute `http(s)://` reference to a domain
  other than `github.com` or `fontcarver.com` (this is what enforces "no third
  party at runtime").
- A total-page-weight budget assertion: first screen under 200 KB, whole page
  under 500 KB including fonts.

**Job `deploy`** — `needs: verify`, and gated twice:

```
if: github.event_name != 'pull_request'
    && github.ref == 'refs/heads/main'
    && vars.CLOUDFLARE_PAGES_PROJECT != ''
```

with `secrets.CLOUDFLARE_API_TOKEN` and `secrets.CLOUDFLARE_ACCOUNT_ID`, using
`cloudflare/wrangler-action` to run `pages deploy site --project-name
${{ vars.CLOUDFLARE_PAGES_PROJECT }}`. Because the repository variable does not
exist and the branch is not `main`, **nothing deploys until someone
deliberately sets them** — which is the "build it, do not publish it"
requirement, enforced by the workflow rather than by memory.

`site/README.md` documents the one-time Cloudflare setup as a checklist:
create the Pages project, mint a scoped API token, add the two secrets and the
one variable, add `fontcarver.com` and `www.fontcarver.com` as custom domains,
point DNS, and verify the redirect from `www`.

## 9. Verification

- `python3 scripts/check_site.py` passes.
- The site is opened in a browser at 1440, 768 and 320 px, in light and dark,
  and screenshots are recorded in the implementation report.
- Keyboard-only pass: every control reachable, focus visible, hero points
  movable with arrow keys.
- JavaScript disabled: hero figure, all copy, all links and the download
  section remain correct and complete.
- `prefers-reduced-motion: reduce`: no animation runs.
- Contrast: body and muted text at or above 4.5:1 in both themes, checked with
  the computed values.
- Every factual claim on the page is traced to `README.md`, `docs/STATUS.md`
  or an acceptance record; the implementation report lists the traces.

## 10. Out of scope

Blog, changelog, documentation hosting, search, newsletter, analytics,
telemetry, a downloads API, release automation, and regenerating the product
screenshots. Each is a separate piece of work; the roadmap section links to
the repository for what it does not host.

---

## 11. What the implementation changed, and why

The site was built from this specification on 2026-09-24. Six things ended up
different from §5 to §9, all decided while building and recorded here so the
document and the site agree.

1. **The strike is rasterised in the browser, not extracted.** §5 had the
   extraction tool precompute bitmap strikes. It did — and then dragging a
   point changed the outline while the pixels stayed put, which contradicted
   the one claim the figure exists to make. The scanline fill moved into
   `hero.js`, so the strike is rasterised from whatever outline is on screen.
   `specimens.js` got smaller as a result.
2. **Implied points are first-class.** The Latin specimen is quadratic and has
   38 implied on-curve points beside its 35 explicit ones. They are drawn in
   the application's own `--cv-node-implied` colour, they are not draggable,
   and they follow their two controls — which is what they are.
3. **No component outlining in the CJK specimen.** §5 said components would be
   drawn in `--cv-component`. `永` in Noto Serif CJK SC is a single outline
   with no components, so there was nothing true to draw. The claim stayed in
   the capability card, where the repository backs it; the figure does not
   illustrate it.
4. **No read/write markers on the format wall.** §4 said markers would come
   from the acceptance records. They are not recorded format by format, so
   marking them would have meant inventing them. The wall groups the formats
   and the footnote states the limitation in the README's own terms.
5. **No screenshot-gallery slot.** §2 kept a slot for real screenshots later.
   An empty slot is a hole in a page; when screenshots exist, a section is
   easy to add. Nothing was left half-built.
6. **The figure is injected into the markup, not written as standalone SVG
   files.** §5 wrote `site/assets/svg/hero-<key>.svg`. An external SVG cannot
   inherit the page's theme or be edited in place, so the tool writes the
   figure between markers in both pages instead.

Two numbers were set to what the site actually weighs rather than to round
guesses: first load is capped at 200 KiB and the whole directory at 320 KiB,
against 257 KiB served today.
