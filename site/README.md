# fontcarver.com

The public website. Hand-written static files: no build step, no
`package.json`, no `node_modules`. What is in this directory is what is
served.

The binding design is
[`docs/superpowers/specs/2026-09-24-website-design.md`](../docs/superpowers/specs/2026-09-24-website-design.md).

## Layout

```
index.html              English page
zh/index.html           Chinese page
404.html
assets/css/tokens.css   design tokens, light and dark
assets/css/site.css     layout and components
assets/js/theme.js      theme bootstrap, loaded synchronously in <head>
assets/js/specimens.js  generated outline data — do not edit
assets/js/hero.js       the live canvas
assets/js/site.js       theme toggle, platform promotion
assets/fonts/*.woff2    self-hosted subset faces
assets/img/             favicon and the social card
tools/                  development tools; never deployed
_headers _redirects     Cloudflare Pages configuration
```

## Working on it

Serve the directory and open it:

```sh
python3 -m http.server 8787 --directory site
```

Before committing:

```sh
python3 scripts/check_site.py
```

The checker fails on a broken internal link, a missing `alt`, missing page
metadata, an incomplete `hreflang` pair, a reference to any host other than
`github.com` or `fontcarver.com`, and a page or directory over its weight
budget. That last rule is what keeps the promise the site makes: it loads
nothing from anyone else, which is also why it works from everywhere.

## Rules that are easy to break

- **No third-party requests.** No CDN, no font service, no analytics. The
  content security policy in `_headers` forbids it and the checker catches it
  before the policy would.
- **No inline `<script>` or `style=` in markup.** The policy has no
  `unsafe-inline`. Dynamic geometry is applied through SVG attributes and the
  CSSOM.
- **`assets/css/tokens.css` is a copy** of the application's
  `app/src/styles/tokens.css` and `docs/design/editor-ui.md` §2. The site
  ships separately, so the values are restated rather than imported. When the
  application's tokens change, re-sync this file.
- **The hero figure is generated.** The markup between the `hero:begin` and
  `hero:end` markers in both pages, and `assets/js/specimens.js`, come from
  `tools/extract-outlines.py`. Edit the tool, not the output.

## Regenerating the hero figure

The figure draws two real glyphs, read out of OFL-licensed fonts:

```sh
python3 site/tools/extract-outlines.py \
    --latin site/assets/fonts/source-serif-4-variable.woff2 \
    --cjk /usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc \
    --cjk-font-number 2 \
    --inject site/index.html site/zh/index.html
```

It needs `fontTools` and `brotli`, which are development dependencies and are
not needed to build or serve the site. Both source fonts are recorded in
[`THIRD_PARTY.md`](../THIRD_PARTY.md).

## Deployment

`.github/workflows/site.yml` verifies every push and pull request that touches
`site/`, and deploys only when **all three** of these hold:

1. the event is not a pull request,
2. the ref is `refs/heads/main`,
3. the repository variable `CLOUDFLARE_PAGES_PROJECT` is set.

None of them hold today, so nothing is published. Publishing is a deliberate
act, not an accident of merging.

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
   that redirect is in `_redirects`, not in the dashboard.

### Going live with real downloads

The download section ships in a pending state. When the first release exists:

1. replace the `<span>` in each `.artifacts` row with the artefact's size and
   its SHA-256,
2. point each row at its release URL,
3. change `data-state="pending"` on `#downloads`,
4. delete the `#release-notice` block from both pages.
