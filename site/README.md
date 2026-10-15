# fontcarver.com

The public website: Astro 7 with Svelte 5 islands, built to static files. What
is deployed is `site/dist`, which is generated; nothing in it is written by
hand.

The binding design is
[`docs/superpowers/specs/2026-09-24-website-design.md`](../docs/superpowers/specs/2026-09-24-website-design.md),
superseded in presentation by the current build but not in its rules: one
origin, no third-party request, and a weight budget the checker enforces.

## Layout

```
astro.config.mjs          build configuration, including the no-inline rules
src/pages/index.astro     English page
src/pages/zh/index.astro  Chinese page
src/pages/404.astro
src/components/           Home, Headline, Terminal, GlyphDrawing (Svelte)
src/layouts/Base.astro    head, masthead, footer
src/data/content.ts       the shape both languages fill
src/data/content.en.ts    English copy
src/data/content.zh.ts    Chinese copy, written by a separate translation pass
src/data/specimens.ts     generated outline data — do not edit
src/data/release.ts       generated release data — do not edit
src/lib/glyph.ts          glyph geometry: points, path, rasteriser
src/lib/enhance.ts        the one bundled module the pages load
src/styles/site.css       the whole design system
public/                   copied verbatim into dist
public/assets/fonts/      self-hosted subset faces
public/assets/js/theme.js the pre-paint theme bootstrap
public/_headers           Cloudflare Pages headers, including the policy
tools/                    development tools; never deployed
```

## Working on it

```sh
cd site
bun install
bun run dev            # http://localhost:4321
bun run build          # writes dist/
bun run verify         # build, then run the site checker against dist/
```

Before committing, from the repository root:

```sh
cd site && bun run build && cd ..
python3 scripts/check_site.py --root site/dist
```

The checker fails on a broken internal link, a missing `alt`, missing page
metadata, an incomplete `hreflang` pair, a reference to any host other than
`github.com`, `fontcarver.com` or `fontpixel.com`, an inline script or style,
and a page or directory over its weight budget. That last rule is what keeps
the promise the site makes: it loads nothing from anyone else, which is also
why it works from everywhere.

## Rules that are easy to break

- **No third-party requests.** No CDN, no font service, no analytics. The
  content security policy in `public/_headers` forbids it and the checker
  catches it first.
- **No inline `<script>` or `style=`.** The policy has no `unsafe-inline`.
  This is why the Svelte drawing is not an Astro island: a `client:` directive
  emits an inline hydration loader. Astro renders the component statically and
  `src/lib/enhance.ts` mounts a live copy over it, which keeps every script in
  a file with a `src`. Set geometry through SVG attributes, never `style=`.
- **The theme bootstrap is deliberately not bundled.** A module script is
  deferred, and a deferred theme is a flash of the wrong one.
- **Generated data is generated.** `src/data/specimens.ts` comes from
  `tools/extract-outlines.py`, `src/data/release.ts` from
  `scripts/release-data.mjs`. Edit the tool, not the output.
- **The Chinese page is translated separately.** Whoever writes the English
  copy does not write `content.zh.ts`; see `CONVENTIONS.md`.

## Regenerating the generated files

The drawing reads two real glyphs out of OFL-licensed fonts:

```sh
python3 site/tools/extract-outlines.py \
    --latin site/public/assets/fonts/source-serif-4-variable.woff2 \
    --cjk /usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc \
    --cjk-font-number 2 \
    --out site/src/data/specimens.ts
```

The download section reads the newest published release:

```sh
cd site && node scripts/release-data.mjs
```

The Han faces are subset to exactly the characters each page sets. Run this
after changing Chinese copy, or the page falls back to whatever Song serif the
reader's system has:

```sh
python3 site/tools/build-han-subset.py
```

All three need development dependencies (`fontTools` with Brotli, a network,
Source Han Serif) that the built site does not.

## Deployment

`.github/workflows/site.yml` builds and verifies every push and pull request
that touches `site/`, and deploys `site/dist` only when **all three** of these
hold:

1. the event is not a pull request,
2. the ref is `refs/heads/main`,
3. the repository variable `CLOUDFLARE_PAGES_PROJECT` is set.

None of them hold together today, so nothing is published. Publishing is a
deliberate act, not an accident of merging.

### One-time Cloudflare setup

1. In the Cloudflare dashboard, create a **Pages** project with **Direct
   Upload** (not a Git integration — GitHub Actions does the upload). Note the
   project name.
2. Create an API token with the **Cloudflare Pages: Edit** permission on that
   account, and copy the account ID from the dashboard sidebar.
3. In the GitHub repository, add:
   - secret `CLOUDFLARE_API_TOKEN`
   - secret `CLOUDFLARE_ACCOUNT_ID`
   - variable `CLOUDFLARE_PAGES_PROJECT` — the project name from step 1
4. Push to `main`, or run the workflow by hand from the Actions tab. The first
   deployment creates the `production` environment; approvals can be required
   on it afterwards.
5. In the Pages project, add the custom domains `fontcarver.com` and
   `www.fontcarver.com`, and point the domain's DNS at Cloudflare.
6. Check that `https://www.fontcarver.com/` redirects to the apex domain —
   that redirect is in `public/_redirects`, not in the dashboard.
