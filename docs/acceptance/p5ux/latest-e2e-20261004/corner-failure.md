# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: draw.spec.ts >> open corner >> opens a reflex corner and reconnects it from the Path menu
- Location: e2e/draw.spec.ts:155:3

# Error details

```
Error: expect(locator).toContainText(expected) failed

Locator: getByRole('region', { name: 'Info box' })
Expected substring: "2 of"
Received string:    "Node▾XYTypeCornerCurveOn-curveCorner radiusSource cornerRemove cornerPositive rounds the corner; negative cuts an ink trap; zero removes it. Bake from Layers → Layer effects."
Timeout: 5000ms

Call log:
  - Expect "toContainText" with timeout 5000ms
  - waiting for getByRole('region', { name: 'Info box' })
    14 × locator resolved to <section data-open="true" class="_box_cjn2v_3" aria-label="Info box">…</section>
       - unexpected value "Node▾XYTypeCornerCurveOn-curveCorner radiusSource cornerRemove cornerPositive rounds the corner; negative cuts an ink trap; zero removes it. Bake from Layers → Layer effects."

```

```yaml
- region "Info box":
  - heading "Node" [level=2]
  - button "Collapse info box" [expanded]: ▾
  - text: X
  - textbox "X": "696"
  - text: "Y"
  - textbox "Y": "231.3"
  - text: Type
  - button "Corner"
  - text: Curve On-curve Corner radius
  - textbox "Corner radius": "0"
  - text: Source corner
  - button "Remove corner" [disabled]
  - paragraph: Positive rounds the corner; negative cuts an ink trap; zero removes it. Bake from Layers → Layer effects.
```

# Test source

```ts
  171 |     await dialog.getByRole('radio', { name: 'None', exact: true }).check()
  172 |     await dialog.getByRole('button', { name: 'Create' }).click()
  173 |     // The new document's tab has to be the active one before the grid below is
  174 |     // this font's grid; selecting it explicitly keeps the spec independent of
  175 |     // whether creating a font also focuses it.
  176 |     const tab = page.getByRole('tab', { name: family })
  177 |     await expect(tab).toBeVisible()
  178 |     await tab.click()
  179 |     await expect(tab).toHaveAttribute('aria-selected', 'true')
  180 | 
  181 |     const doc = await findDoc(page, family)
  182 |     const added = await api<{ added: { gid: number; name: string }[] }>(
  183 |       page,
  184 |       'glyphs.add',
  185 |       { doc, recipe: 'A', master: null, dry_run: false },
  186 |     )
  187 |     const gid = added.added[0]?.gid as number
  188 |     const points = async () => {
  189 |       const outline = await api<Outline>(page, 'glyphs.outline', {
  190 |         doc,
  191 |         gid,
  192 |         layer: null,
  193 |       })
  194 |       return outline.contours[0]?.points.length ?? 0
  195 |     }
  196 | 
  197 |     try {
  198 |       const grid = page.getByRole('listbox', { name: 'Glyphs' })
  199 |       const option = grid.getByRole('option', { name: 'A', exact: true })
  200 |       await expect(option).toBeVisible()
  201 |       await option.dblclick()
  202 |       const canvas = page.getByRole('img', { name: /^Glyph A/ })
  203 |       await expect(canvas).toBeVisible()
  204 |       const bounds = await canvas.boundingBox()
  205 |       if (!bounds) throw new Error('canvas has no bounds')
  206 | 
  207 |       // The Info box floats over the canvas's lower left, and at this window
  208 |       // size it covers a good half of a canvas that is only as tall as its
  209 |       // minimum. Every position below is therefore measured from the box's
  210 |       // right edge, so the chevron is drawn on canvas the user can actually
  211 |       // reach; a click on a covered point would otherwise wait out the test.
  212 |       const info = page.getByRole('region', { name: 'Info box' })
  213 |       const infoBounds = await info.boundingBox()
  214 |       const left = infoBounds
  215 |         ? infoBounds.x + infoBounds.width - bounds.x + 16
  216 |         : 0
  217 |       const span = bounds.width - left - 16
  218 |       if (span < 120) throw new Error('no drawable canvas beside the Info box')
  219 | 
  220 |       // A chevron whose middle node points back into the ink: the only kind of
  221 |       // corner a single contour can actually cross itself at.
  222 |       const at = (x: number, y: number) => ({
  223 |         x: left + span * x,
  224 |         y: bounds.height * y,
  225 |       })
  226 |       const outline = [
  227 |         at(0, 0.3),
  228 |         at(1, 0.3),
  229 |         at(0.45, 0.55),
  230 |         at(1, 0.8),
  231 |         at(0, 0.8),
  232 |       ]
  233 |       const reflex = outline[2] as { x: number; y: number }
  234 | 
  235 |       // A press that lands in a field would type the tool key instead of using
  236 |       // it, so the focus click goes to clear canvas too.
  237 |       await canvas.click({ position: at(0.5, 0.55), timeout: CLICK_TIMEOUT })
  238 |       await page.keyboard.press('p')
  239 |       await expect(
  240 |         page.getByRole('button', { name: 'Pen', exact: true }),
  241 |       ).toHaveAttribute('aria-pressed', 'true')
  242 |       for (const corner of outline)
  243 |         await canvas.click({ position: corner, timeout: CLICK_TIMEOUT })
  244 |       await canvas.click({
  245 |         position: outline[0] as { x: number; y: number },
  246 |         timeout: CLICK_TIMEOUT,
  247 |       })
  248 |       await expect.poll(points).toBe(5)
  249 | 
  250 |       // Select the corner with the Select tool and open it.
  251 |       await page.keyboard.press('v')
  252 |       await canvas.click({ position: reflex, timeout: CLICK_TIMEOUT })
  253 |       // By role and exact name: "Collapse info box" also contains an X.
  254 |       await expect(
  255 |         info.getByRole('textbox', { name: 'X', exact: true }),
  256 |       ).toBeVisible()
  257 | 
  258 |       const menubar = page.getByRole('menubar')
  259 |       await menubar.getByRole('menuitem', { name: 'Path', exact: true }).click()
  260 |       await page.getByRole('menuitem', { name: /^Open Corner/ }).click()
  261 |       await expect.poll(points).toBe(6)
  262 |       await expect(page.getByRole('contentinfo')).toContainText('Open corner')
  263 | 
  264 |       // Marquee the mouth: both of its nodes sit within a few pixels of where
  265 |       // the corner was, and the neighbouring nodes are a third of the canvas
  266 |       // away. The drag starts inside the ink, which is not hit-testable.
  267 |       await page.mouse.move(reflex.x + bounds.x - 30, reflex.y + bounds.y - 22)
  268 |       await page.mouse.down()
  269 |       await page.mouse.move(reflex.x + bounds.x + 18, reflex.y + bounds.y + 22)
  270 |       await page.mouse.up()
> 271 |       await expect(info).toContainText('2 of')
      |                          ^ Error: expect(locator).toContainText(expected) failed
  272 | 
  273 |       await menubar.getByRole('menuitem', { name: 'Path', exact: true }).click()
  274 |       await page.getByRole('menuitem', { name: /^Reconnect Nodes/ }).click()
  275 |       await expect.poll(points).toBe(5)
  276 |       await expect(page.getByRole('contentinfo')).toContainText(
  277 |         'Reconnect nodes',
  278 |       )
  279 | 
  280 |       // Two gestures, two undo entries.
  281 |       await page.keyboard.press('Control+z')
  282 |       await expect.poll(points).toBe(6)
  283 |       await page.keyboard.press('Control+z')
  284 |       await expect.poll(points).toBe(5)
  285 |     } finally {
  286 |       await api(page, 'session.close', { doc, force: true })
  287 |     }
  288 |   })
  289 | })
  290 | 
```