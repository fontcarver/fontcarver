import { expect } from '@playwright/test'
import { test } from './sessionFixtures'

import { resolve } from 'node:path'

import { api, REPO_ROOT } from './support'
import {
  docs,
  firstOnCurve,
  openGlyphFromGrid,
  outline,
} from './acceptanceSupport'

const TEXT = 'Hamburgefonstiv'

interface RunGlyph {
  documentGid: number | null
  name: string | null
  x: number
  y: number
  xAdvance: number
  yAdvance: number
}

interface ShapedRun {
  glyphs: {
    gid: number
    x_advance: number
    y_advance: number
    x_offset: number
    y_offset: number
    cluster: number
  }[]
}

async function runGlyphs(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const run = (
      globalThis as unknown as {
        __fwTextRun?: { glyphs: RunGlyph[]; activeIndex: number | null } | null
      }
    ).__fwTextRun
    if (!run) throw new Error('missing text-run dev handle')
    return { glyphs: run.glyphs, activeIndex: run.activeIndex }
  })
}

async function editorState(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const editor = (
      globalThis as unknown as {
        __fwEditor?: {
          currentSelection: { size: number }
          viewportForTest: {
            scale: number
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
    ).__fwEditor
    if (!editor) throw new Error('missing editor dev handle')
    return {
      selectionSize: editor.currentSelection.size,
      scale: editor.viewportForTest.scale,
    }
  })
}

async function moveLocalPoint(
  page: import('@playwright/test').Page,
  doc: number,
  gid: number,
  location: { contour: number; point: number; x: number; y: number },
  delta: { x: number; y: number },
) {
  const canvas = page.getByRole('img', { name: /^Glyph / })
  const box = await canvas.boundingBox()
  if (!box) throw new Error('glyph canvas has no bounds')
  const screen = await page.evaluate(({ x, y }) => {
    const editor = (
      globalThis as unknown as {
        __fwEditor?: {
          viewportForTest: {
            scale: number
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
    ).__fwEditor
    if (!editor) throw new Error('missing editor dev handle')
    return {
      ...editor.viewportForTest.toScreen({ x, y }),
      scale: editor.viewportForTest.scale,
    }
  }, location)
  const start = { x: box.x + screen.x, y: box.y + screen.y }
  const end = {
    x: start.x + delta.x * screen.scale,
    y: start.y - delta.y * screen.scale,
  }
  await page.mouse.move(start.x, start.y)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.move(end.x, end.y)
  await page.mouse.up()
  await page.keyboard.up('Control')

  let moved = { x: Number.NaN, y: Number.NaN }
  await expect
    .poll(async () => {
      const point = (await outline(page, doc, gid)).contours[location.contour]
        ?.points[location.point]
      if (point) moved = point
      return (
        !!point &&
        Math.abs(point.x - location.x - delta.x) < 0.01 &&
        Math.abs(point.y - location.y - delta.y) < 0.01
      )
    })
    .toBe(true)
  return { ...location, ...moved }
}

test('shapes text and edits the third glyph in local coordinates', async ({
  page,
}) => {
  test.setTimeout(120_000)
  await page.goto('/')
  const [document] = await docs(page)
  if (!document) throw new Error('DejaVu fixture was not opened by the server')

  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const a = grid.getByRole('option', { name: 'A', exact: true })
  await expect(a).toBeVisible()
  await a.dblclick()
  const canvas = page.getByRole('img', { name: /^Glyph A/ })
  await expect(canvas).toBeVisible()

  await canvas.press('t')
  const view = page.locator('[data-text-mode]')
  await expect(view).toHaveAttribute('data-text-mode', 'text')
  const input = page.getByRole('textbox', { name: 'Text', exact: true })
  await expect(input).toBeFocused()
  await input.fill(TEXT)
  // The first `text.shape` of a session compiles the whole fixture for the live
  // shaper: measured at 3.9-4.3 s on a debug kernel here, against 79 ms once the
  // compile is warm. Until it answers the view reports `data-text-shaping` with
  // a run count of zero, which is not the same as "this text has no glyphs", so
  // the wait is explicit and generous rather than the default five seconds.
  await expect(view).toHaveAttribute('data-text-shaping', 'false', {
    timeout: 60_000,
  })
  await expect(view).toHaveAttribute('data-run-count', String(TEXT.length))
  await expect
    .poll(async () => (await runGlyphs(page)).glyphs.length)
    .toBe(TEXT.length)

  const shaped = await api<ShapedRun>(page, 'text.shape', {
    doc: document.doc,
    text: TEXT,
    master: null,
    features: [
      ['kern', true],
      ['liga', true],
      ['calt', true],
    ],
    script: null,
    language: null,
    direction: 'ltr',
    size: null,
  })
  const positioned = (await runGlyphs(page)).glyphs
  const first = positioned[0]
  const second = positioned[1]
  const firstShaped = shaped.glyphs[0]
  const secondShaped = shaped.glyphs[1]
  if (!first || !second || !firstShaped || !secondShaped)
    throw new Error('shaping did not produce the first two glyphs')
  expect(first.x).toBe(firstShaped.x_offset)
  expect(first.y).toBe(firstShaped.y_offset)
  expect(first.xAdvance).toBe(firstShaped.x_advance)
  expect(first.yAdvance).toBe(firstShaped.y_advance)
  expect(second.x).toBe(firstShaped.x_advance + secondShaped.x_offset)
  expect(second.y).toBe(firstShaped.y_advance + secondShaped.y_offset)
  expect(second.x).toBe(first.x + first.xAdvance)

  const target = await canvas.evaluate((element) => {
    const globals = globalThis as unknown as {
      __fwTextRun?: { glyphs: RunGlyph[] } | null
      __fwEditor?: {
        viewportForTest: {
          toScreen: (point: { x: number; y: number }) => {
            x: number
            y: number
          }
        }
      }
    }
    const glyph = globals.__fwTextRun?.glyphs[2]
    const editor = globals.__fwEditor
    if (!glyph || !editor) throw new Error('third shaped glyph is unavailable')
    const rect = (
      element as unknown as {
        getBoundingClientRect: () => { left: number; top: number }
      }
    ).getBoundingClientRect()
    const at = editor.viewportForTest.toScreen({
      x: glyph.x + glyph.xAdvance / 2,
      y: glyph.y + 300,
    })
    return { x: rect.left + at.x, y: rect.top + at.y }
  })
  await page.mouse.click(target.x, target.y)
  await expect(view).toHaveAttribute('data-text-mode', 'edit')
  await expect.poll(async () => (await runGlyphs(page)).activeIndex).toBe(2)
  const third = (await runGlyphs(page)).glyphs[2]
  if (!third?.name || third.documentGid === null)
    throw new Error('third glyph did not resolve to a document glyph')
  expect(third.name).toBe('m')
  await expect(
    page.getByRole('img', { name: new RegExp(`^Glyph ${third.name}`) }),
  ).toBeVisible()

  const originalOutline = await outline(page, document.doc, third.documentGid)
  const originalPoint = firstOnCurve(originalOutline)
  const movedPoint = await moveLocalPoint(
    page,
    document.doc,
    third.documentGid,
    originalPoint,
    { x: 8, y: 5 },
  )
  expect(movedPoint.x).toBeCloseTo(originalPoint.x + 8, 2)
  expect(movedPoint.y).toBeCloseTo(originalPoint.y + 5, 2)
  expect((await editorState(page)).selectionSize).toBeGreaterThan(0)

  const activeCanvas = page.getByRole('img', { name: /^Glyph m/ })
  await activeCanvas.press('Control+ArrowRight')
  await activeCanvas.press('Control+ArrowRight')
  await activeCanvas.press('Control+ArrowRight')
  await page.waitForTimeout(650)
  await expect
    .poll(
      async () => (await outline(page, document.doc, third.documentGid!)).width,
    )
    .toBe(originalOutline.width + 3)

  await activeCanvas.press('Control+z')
  await expect
    .poll(
      async () => (await outline(page, document.doc, third.documentGid!)).width,
    )
    .toBe(originalOutline.width)
  const afterMetricsUndo = await outline(page, document.doc, third.documentGid)
  const stillMoved =
    afterMetricsUndo.contours[movedPoint.contour]?.points[movedPoint.point]
  expect(stillMoved?.x).toBeCloseTo(movedPoint.x, 5)
  expect(stillMoved?.y).toBeCloseTo(movedPoint.y, 5)
  await activeCanvas.press('Control+z')
  await expect
    .poll(async () => {
      const restored = (await outline(page, document.doc, third.documentGid!))
        .contours[originalPoint.contour]?.points[originalPoint.point]
      return (
        !!restored &&
        Math.abs(restored.x - originalPoint.x) < 0.01 &&
        Math.abs(restored.y - originalPoint.y) < 0.01
      )
    })
    .toBe(true)

  const initialScale = (await editorState(page)).scale
  const size = page.getByRole('textbox', {
    name: 'Preview size',
    exact: true,
  })
  await expect(size).toHaveValue('128')
  await size.fill('256')
  await size.press('Enter')
  await expect
    .poll(async () => (await editorState(page)).scale)
    .toBeCloseTo(256 / 2048, 6)
  expect((await editorState(page)).scale).not.toBe(initialScale)

  await activeCanvas.press('Escape')
  await expect.poll(async () => (await editorState(page)).selectionSize).toBe(0)
  await expect(view).toHaveAttribute('data-text-mode', 'edit')
  await activeCanvas.press('Escape')
  await expect(view).toHaveAttribute('data-text-mode', 'text')
  await expect(input).toBeFocused()
})

/** Opens the Text tab on one glyph and waits for its first shaped run. */
async function openTextTab(
  page: import('@playwright/test').Page,
  text: string,
) {
  await page.goto('/')
  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const letter = grid.getByRole('option', { name: 'A', exact: true })
  await expect(letter).toBeVisible()
  await letter.dblclick()
  await expect(page.getByRole('img', { name: /^Glyph A/ })).toBeVisible()
  await page.getByRole('img', { name: /^Glyph A/ }).press('t')
  const view = page.locator('[data-text-mode]')
  await expect(view).toHaveAttribute('data-text-mode', 'text')
  const input = page.getByRole('textbox', { name: 'Text', exact: true })
  await expect(input).toBeFocused()
  await input.fill(text)
  // See the note on the first case: a cold kernel compiles the whole fixture.
  await expect(view).toHaveAttribute('data-text-shaping', 'false', {
    timeout: 60_000,
  })
  await expect(view).toHaveAttribute('data-run-count', String(text.length))
  return { view, input }
}

test('configures the waterfall as a list and as a range with a step', async ({
  page,
}) => {
  test.setTimeout(180_000)
  await openTextTab(page, 'Hamburg')
  const strip = page.locator('[data-preview-theme]')
  await expect(strip).toHaveAttribute('data-preview-sizes', '12, 24, 48')
  await strip.getByRole('button', { name: 'Preview settings' }).click()
  const sizes = strip.getByRole('textbox', { name: 'Waterfall sizes (px)' })
  await sizes.fill('8-32/8')
  await strip.getByRole('button', { name: 'Apply sizes' }).click()
  await expect(strip).toHaveAttribute('data-preview-sizes', '8, 16, 24, 32')
  await expect(sizes).toHaveValue('8, 16, 24, 32')
  for (const size of [8, 16, 24, 32])
    await expect(
      strip.getByRole('img', { name: `${size} px preview` }),
    ).toBeVisible()

  await sizes.fill('10, 60, 120')
  await strip.getByRole('button', { name: 'Apply sizes' }).click()
  await expect(strip).toHaveAttribute('data-preview-sizes', '10, 60, 120')
  await expect(strip.getByRole('img')).toHaveCount(3)

  // A rejected draft keeps the rendered waterfall and says why.
  await sizes.fill('4, 999')
  await strip.getByRole('button', { name: 'Apply sizes' }).click()
  await expect(strip.getByRole('alert')).toContainText('between 6 and 256')
  await expect(strip).toHaveAttribute('data-preview-sizes', '10, 60, 120')

  // The single-glyph waterfall shows one glyph at every step.
  await strip
    .getByRole('combobox', { name: 'Preview content' })
    .selectOption('single')
  await expect(strip).toHaveAttribute('data-preview-mode', 'single')
  await expect(strip.getByRole('img')).toHaveCount(3)
})

test('adopts the built-in samples and steps them from the field and the canvas', async ({
  page,
}) => {
  test.setTimeout(180_000)
  const { view, input } = await openTextTab(page, 'Hamburg')
  const [document] = await docs(page)
  if (!document) throw new Error('the fixture is not open')
  // The library is project data, so restore the original metadata at the end.
  const before = await api<{ info: unknown }>(page, 'doc.font_info', {
    doc: document.doc,
  })
  await page.getByText('Proof library').click()
  await page.getByRole('button', { name: 'Add built-in samples' }).click()
  const chooser = page.getByRole('combobox', { name: 'Choose a saved string' })
  await expect(chooser).toBeEnabled({ timeout: 30_000 })
  await expect(chooser.locator('option')).not.toHaveCount(1)

  await input.focus()
  await input.press('ArrowDown')
  await expect
    .poll(async () => input.inputValue(), { timeout: 30_000 })
    .not.toBe('Hamburg')
  const first = await input.inputValue()
  await input.press('ArrowDown')
  await expect.poll(async () => input.inputValue()).not.toBe(first)

  // Spacing happens with the canvas focused and nothing selected: the same keys
  // must step there too, which is what row 95 asks for.
  const canvas = page.getByRole('img', { name: /^Glyph / })
  await canvas.click({ position: { x: 5, y: 5 } })
  const beforeCanvasStep = await input.inputValue()
  await canvas.press('ArrowDown')
  await expect.poll(async () => input.inputValue()).not.toBe(beforeCanvasStep)
  await expect(view).toHaveAttribute('data-text-mode', 'text')

  // Script filtering narrows what the keys step through.
  await page
    .getByRole('combobox', { name: 'Filter by script' })
    .selectOption('Hani')
  await input.focus()
  await input.press('ArrowDown')
  await expect.poll(async () => input.inputValue()).toMatch(/[㐀-鿿]/u)

  await api(page, 'doc.apply', {
    doc: document.doc,
    command: { SetFontInfo: { info: before.info } },
    label: 'Restore proof library',
  })
  await expect(page.getByText(/saved strings$/)).toHaveText('0 saved strings')
})

test('exports a self-contained HTML proof with the compiled font embedded', async ({
  page,
}, testInfo) => {
  test.setTimeout(240_000)
  await openTextTab(page, 'Hamburgefonstiv')
  await page.getByText('Proof library').click()
  const group = page.getByRole('group', { name: 'Proof sheet' })
  await group
    .getByRole('combobox', { name: 'HTML rendering' })
    .selectOption('font')
  await group.getByRole('textbox', { name: 'Proof sizes (pt)' }).fill('18, 48')
  const download = page.waitForEvent('download', { timeout: 180_000 })
  await group.getByRole('button', { name: 'Export HTML' }).click()
  const file = testInfo.outputPath('proof-embedded.html')
  await (await download).saveAs(file)

  const html = await import('node:fs/promises').then((fs) =>
    fs.readFile(file, 'utf8'),
  )
  expect(html).toContain('@font-face')
  expect(html).toMatch(
    /src:url\(data:font\/woff2;base64,[A-Za-z0-9+/=]{1000,}\)/,
  )
  expect(html).not.toMatch(/<svg/)
  expect(html).toContain('18 pt')
  expect(html).toContain('48 pt')

  // Load the exported file in the browser: the page must set the text in the
  // embedded face, which only works if the WOFF2 inside it is a real font.
  const sheet = await page.context().newPage()
  await sheet.goto(`file://${file}`)
  const measured = await sheet.evaluate(async () => {
    // The e2e project has no DOM library, so the page's own objects are reached
    // through explicit structural casts, as the other cases here do.
    const win = globalThis as unknown as {
      document: {
        fonts: { ready: Promise<unknown>; check: (font: string) => boolean }
        querySelectorAll: (selector: string) => ArrayLike<{
          textContent: string | null
          getBoundingClientRect: () => { height: number }
        }>
      }
      getComputedStyle: (node: unknown) => {
        fontFamily: string
        fontSize: string
      }
    }
    await win.document.fonts.ready
    const specimens = Array.from(win.document.querySelectorAll('.specimen'))
    return {
      count: specimens.length,
      family: win.getComputedStyle(specimens[0]).fontFamily,
      loaded: win.document.fonts.check('48pt FCProofFont'),
      sizes: specimens.map((node) => win.getComputedStyle(node).fontSize),
      heights: specimens.map((node) => node.getBoundingClientRect().height),
      text: specimens[0]?.textContent ?? '',
    }
  })
  await sheet.close()
  expect(measured.count).toBe(2)
  expect(measured.family).toContain('FCProofFont')
  expect(measured.loaded).toBe(true)
  expect(measured.text).toBe('Hamburgefonstiv')
  // The 48 pt row really is set at 48 pt, so the requested sizes apply.
  expect(measured.sizes).toEqual(['24px', '64px'])
  expect(measured.heights[1]).toBeGreaterThan(measured.heights[0]!)
})

test('exports a vector PDF proof and a vertical outline sheet', async ({
  page,
}, testInfo) => {
  test.setTimeout(240_000)
  const { input } = await openTextTab(page, 'Hamburgefonstiv')
  await page.getByText('Proof library').click()
  const group = page.getByRole('group', { name: 'Proof sheet' })
  await group.getByRole('textbox', { name: 'Proof sizes (pt)' }).fill('24, 48')
  const pdfDownload = page.waitForEvent('download', { timeout: 120_000 })
  await group.getByRole('button', { name: 'Export PDF' }).click()
  const pdfPath = testInfo.outputPath('proof.pdf')
  await (await pdfDownload).saveAs(pdfPath)
  const fs = await import('node:fs/promises')
  const pdf = await fs.readFile(pdfPath)
  expect(pdf.subarray(0, 8).toString('latin1')).toBe('%PDF-1.7')
  const text = pdf.toString('latin1')
  expect(text).toContain('/Type /Pages /Count')
  expect(text).toContain('24 pt')
  expect(text).toContain('48 pt')
  expect(text).not.toContain('/Subtype /Image')
  expect(text.endsWith('%%EOF\n')).toBe(true)

  // Vertical mode must reach the outline sheet as well as the strip.
  await page
    .getByRole('button', { name: 'Vertical text', exact: true })
    .first()
    .click()
  await input.fill('ABCD')
  await expect(page.locator('[data-text-mode]')).toHaveAttribute(
    'data-text-shaping',
    'false',
    { timeout: 60_000 },
  )
  await group
    .getByRole('combobox', { name: 'HTML rendering' })
    .selectOption('outlines')
  const verticalDownload = page.waitForEvent('download', { timeout: 120_000 })
  await group.getByRole('button', { name: 'Export HTML' }).click()
  const verticalPath = testInfo.outputPath('proof-vertical.html')
  await (await verticalDownload).saveAs(verticalPath)
  const vertical = await fs.readFile(verticalPath, 'utf8')
  const viewBox = /viewBox="(-?[\d.]+) (-?[\d.]+) ([\d.]+) ([\d.]+)"/.exec(
    vertical,
  )
  if (!viewBox) throw new Error('vertical sheet has no outline viewBox')
  // A four-glyph vertical run is taller than it is wide.
  expect(Number(viewBox[4])).toBeGreaterThan(Number(viewBox[3]))
})

interface VariationState {
  generation: number
  masters: { id: string; name: string; location: Record<string, number> }[]
}

test('previews a named instance and a second master from the strip', async ({
  page,
}) => {
  test.setTimeout(240_000)
  await page.goto('/')
  // A committed variable fixture does not exist, so the design space is authored
  // through the same API the design-space editor uses, on the small UFO.
  const opened = await api<{ doc: number; family_name: string }>(
    page,
    'session.open',
    { path: resolve(REPO_ROOT, 'fixtures/ufo/Minimal.ufo'), face_index: null },
  )
  try {
    const state = await api<VariationState>(page, 'variation.state', {
      doc: opened.doc,
    })
    const regular = state.masters[0]
    if (!regular) throw new Error('the fixture has no master')
    await api(page, 'variation.apply', {
      doc: opened.doc,
      expected_generation: state.generation,
      command: {
        Set: {
          axis_renames: {},
          axes: [
            {
              tag: [119, 103, 104, 116],
              name: 'Weight',
              min: 100,
              default: 100,
              max: 900,
              map: [],
              hidden: false,
            },
          ],
          masters: [
            { ...regular, location: { wght: 100 } },
            {
              ...regular,
              id: '0123456789abcdef0123456789abcdef',
              name: 'Bold',
              location: { wght: 900 },
            },
          ],
          instances: [
            {
              name: 'SemiBold',
              location: { wght: 600 },
              style_name: 'SemiBold',
              family_name: null,
              active: true,
              is_variable_named_instance: true,
              user_data: null,
            },
          ],
          copy_from: regular.id,
        },
      },
      label: 'Add a weight axis',
    })
    // Added masters are given fresh identities by the kernel, so the ids the
    // picker will offer are read back rather than assumed.
    const applied = await api<VariationState>(page, 'variation.state', {
      doc: opened.doc,
    })
    const bold = applied.masters.find((master) => master.name === 'Bold')
    if (!bold) throw new Error('the second master was not added')
    await page.reload()
    await page.getByRole('tab', { name: opened.family_name }).first().click()
    await openGlyphFromGrid(page, 'A')
    await page.getByRole('img', { name: /^Glyph A/ }).press('t')
    const view = page.locator('[data-text-mode]')
    await expect(view).toHaveAttribute('data-text-mode', 'text')
    const input = page.getByRole('textbox', { name: 'Text', exact: true })
    await input.fill('AB')
    await expect(view).toHaveAttribute('data-text-shaping', 'false', {
      timeout: 60_000,
    })

    const strip = page.locator('[data-preview-theme]').first()
    await strip.getByRole('button', { name: 'Preview settings' }).click()
    const picker = strip.getByRole('combobox', { name: 'Preview instance' })
    expect(
      await picker.locator('optgroup').evaluateAll((nodes) =>
        nodes.map((node) =>
          (
            node as unknown as {
              getAttribute: (name: string) => string | null
            }
          ).getAttribute('label'),
        ),
      ),
    ).toEqual(['Masters', 'Named instances'])
    expect(
      await picker
        .locator('option')
        .evaluateAll((nodes) =>
          nodes.map(
            (node) =>
              (node as unknown as { textContent: string | null }).textContent,
          ),
        ),
    ).toEqual(['Regular', 'Bold', 'SemiBold'])

    // A named instance renders interpolated outlines without moving the editor.
    await picker.selectOption({ label: 'SemiBold' })
    await expect(strip.getByText('Preview unavailable')).toBeHidden()
    await expect
      .poll(async () => strip.getByRole('img').count(), { timeout: 30_000 })
      .toBe(3)
    const masterField = page
      .getByRole('toolbar', { name: 'Text controls' })
      .getByRole('combobox', { name: 'Master', exact: true })
    await expect(masterField).toHaveValue(regular.id)

    // A master, by contrast, moves the whole run — the editor follows it.
    await picker.selectOption({ label: 'Bold' })
    await expect(masterField).toHaveValue(bold.id)
    await expect(view).toHaveAttribute('data-text-shaping', 'false', {
      timeout: 60_000,
    })
    await expect(strip.getByText('Preview unavailable')).toBeHidden()
  } finally {
    // Close only the document this case opened; the suite shares the rest.
    await api(page, 'session.close', { doc: opened.doc })
  }
})
