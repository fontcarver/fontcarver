# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: p2a-kerning.spec.ts >> authors groups and pair exceptions, coalesces shortcuts, saves and exports matching layout
- Location: e2e/p2a-kerning.spec.ts:61:1

# Error details

```
Error: expect(locator).toHaveValue(expected) failed

Locator:  getByRole('combobox', { name: 'Kerning gap', exact: true })
Expected: "0"
Received: ""
Timeout:  5000ms

Call log:
  - Expect "toHaveValue" with timeout 5000ms
  - waiting for getByRole('combobox', { name: 'Kerning gap', exact: true })
    14 × locator resolved to <select aria-label="Kerning gap">…</select>
       - unexpected value ""

```

```yaml
- combobox "Kerning gap":
  - option "Choose a gap" [selected]
  - option "1. A / V"
```

# Test source

```ts
  144 |   const canvas = page.getByRole('img', { name: /^Glyph / })
  145 |   await canvas.press('t')
  146 |   const input = page.getByRole('textbox', { name: 'Text', exact: true })
  147 |   await expect(input).toBeFocused()
  148 |   await input.fill(text)
  149 |   await expect(page.locator('[data-text-mode]')).toHaveAttribute(
  150 |     'data-run-count',
  151 |     String(text.length),
  152 |   )
  153 |   return canvas
  154 | }
  155 | 
  156 | export function resolution(
  157 |   page: Page,
  158 |   doc: DocSummary,
  159 |   master = doc.masters[0].id,
  160 | ) {
  161 |   return api<KerningResolution>(page, 'kerning.resolve', {
  162 |     doc: doc.doc,
  163 |     master,
  164 |     left: 'A',
  165 |     right: 'V',
  166 |   })
  167 | }
  168 | 
  169 | export async function expectAdvance(
  170 |   page: Page,
  171 |   doc: DocSummary,
  172 |   expected: number,
  173 |   kern = true,
  174 |   master = doc.masters[0].id,
  175 | ) {
  176 |   await expect
  177 |     .poll(
  178 |       async () => {
  179 |         const run = await api<ShapedRun>(page, 'text.shape', {
  180 |           doc: doc.doc,
  181 |           master,
  182 |           text: 'AV',
  183 |           features: [['kern', kern]],
  184 |         })
  185 |         return run.stale
  186 |           ? null
  187 |           : run.glyphs.reduce((total, glyph) => total + glyph.x_advance, 0)
  188 |       },
  189 |       { timeout: 15_000 },
  190 |     )
  191 |     .toBe(expected)
  192 | }
  193 | 
  194 | export async function expectPreviewAdvance(page: Page, expected: number) {
  195 |   await expect
  196 |     .poll(() =>
  197 |       page.evaluate(() => {
  198 |         const run = (
  199 |           globalThis as unknown as {
  200 |             __fwTextRun?: { glyphs: { xAdvance: number }[] }
  201 |           }
  202 |         ).__fwTextRun
  203 |         return run?.glyphs.reduce((total, glyph) => total + glyph.xAdvance, 0)
  204 |       }),
  205 |     )
  206 |     .toBe(expected)
  207 | }
  208 | 
  209 | /** Hit the visible gap marker through the canvas's actual pointer handler. */
  210 | export async function selectFirstGap(page: Page, unitsPerEm: number) {
  211 |   const canvas = page.getByRole('img', { name: /^Glyph / })
  212 |   const point = await canvas.evaluate((element, upem) => {
  213 |     const globals = globalThis as unknown as {
  214 |       __fwTextRun?: {
  215 |         glyphs: { x: number; y: number }[]
  216 |         activeOrigin: { x: number; y: number }
  217 |       }
  218 |       __fwEditor?: {
  219 |         viewportForTest: {
  220 |           toScreen: (point: { x: number; y: number }) => {
  221 |             x: number
  222 |             y: number
  223 |           }
  224 |         }
  225 |       }
  226 |     }
  227 |     const run = globals.__fwTextRun
  228 |     const editor = globals.__fwEditor
  229 |     if (!run?.glyphs[1] || !editor) throw new Error('text gap is not ready')
  230 |     const at = editor.viewportForTest.toScreen({
  231 |       x: run.glyphs[1].x - run.activeOrigin.x,
  232 |       y: -upem * 0.12 - run.activeOrigin.y,
  233 |     })
  234 |     const bounds = (
  235 |       element as unknown as {
  236 |         getBoundingClientRect: () => { left: number; top: number }
  237 |       }
  238 |     ).getBoundingClientRect()
  239 |     return { x: bounds.left + at.x, y: bounds.top + at.y }
  240 |   }, unitsPerEm)
  241 |   await page.mouse.click(point.x, point.y)
  242 |   await expect(
  243 |     page.getByRole('combobox', { name: 'Kerning gap', exact: true }),
> 244 |   ).toHaveValue('0')
      |     ^ Error: expect(locator).toHaveValue(expected) failed
  245 |   await expect(page.locator('[data-text-mode]')).toHaveAttribute(
  246 |     'data-text-mode',
  247 |     'text',
  248 |   )
  249 | }
  250 | 
  251 | export async function saveAs(page: Page, path: string) {
  252 |   await menuItem(page, 'File', /^Save As/)
  253 |   const dialog = page.getByRole('dialog', { name: 'Save As' })
  254 |   await expect(dialog).toBeVisible()
  255 |   await dialog.getByRole('textbox', { name: 'Document path' }).fill(path)
  256 |   await dialog.getByRole('button', { name: 'Save', exact: true }).click()
  257 |   await expect(dialog).toBeHidden()
  258 | }
  259 | 
  260 | /** Pause only the next real kerning mutation, allowing a handover to race it. */
  261 | export async function holdNextKerning(page: Page, fail = false) {
  262 |   let release = () => {}
  263 |   const gate = new Promise<void>((resolve) => {
  264 |     release = resolve
  265 |   })
  266 |   let requested = false
  267 |   let captured = false
  268 |   let completed = () => {}
  269 |   const handled = new Promise<void>((resolve) => {
  270 |     completed = resolve
  271 |   })
  272 |   const handler = async (route: Route) => {
  273 |     const request = route.request()
  274 |     if (captured || !request.postData()?.includes('SetHorizontalKerning')) {
  275 |       await route.continue()
  276 |       return
  277 |     }
  278 |     captured = true
  279 |     requested = true
  280 |     await gate
  281 |     try {
  282 |       if (fail) {
  283 |         await route.fulfill({
  284 |           status: 400,
  285 |           contentType: 'application/json',
  286 |           body: JSON.stringify({
  287 |             error: {
  288 |               code: 'invalid_parameter',
  289 |               message: 'P2a delayed kerning failure',
  290 |             },
  291 |           }),
  292 |         })
  293 |       } else {
  294 |         await route.continue()
  295 |       }
  296 |     } finally {
  297 |       completed()
  298 |     }
  299 |   }
  300 |   await page.route('**/api/doc.*apply', handler)
  301 |   return {
  302 |     wait: () => expect.poll(() => requested).toBe(true),
  303 |     release,
  304 |     dispose: async () => {
  305 |       release()
  306 |       if (captured) await handled
  307 |       await page.unroute('**/api/doc.*apply', handler)
  308 |     },
  309 |   }
  310 | }
  311 | 
```