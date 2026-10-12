import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, type Page, type Route } from '@playwright/test'
import { sourceTest as base } from './sessionFixtures'

import type { DocSummary } from '../src/api/generated/DocSummary'
import type { KerningResolution } from '../src/api/generated/KerningResolution'
import type { ShapedRun } from '../src/api/generated/ShapedRun'
import {
  menuItem,
  openGlyphFromGrid,
  openPathThroughDialog,
} from './acceptanceSupport'
import { api, REPO_ROOT } from './support'

const plist = (body: string) =>
  `<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>${body}</dict></plist>`

/** An owned, visible Latin fixture derived from the project's licensed UFO. */
export function sourceFixture(twoMasters = false) {
  const directory = mkdtempSync(join(tmpdir(), 'fc-p2a-browser-'))
  const makeUfo = (name: string, style: string) => {
    const path = join(directory, name)
    cpSync(resolve(REPO_ROOT, 'fixtures/ufo/Minimal.ufo'), path, {
      recursive: true,
    })
    writeFileSync(join(path, 'features.fea'), '')
    writeFileSync(join(path, 'groups.plist'), plist(''))
    writeFileSync(join(path, 'kerning.plist'), plist(''))
    const info = readFileSync(join(path, 'fontinfo.plist'), 'utf8')
      .replace('<string>Minimal</string>', '<string>P2a Browser</string>')
      .replace('<string>Regular</string>', `<string>${style}</string>`)
    writeFileSync(join(path, 'fontinfo.plist'), info)
    const outline = readFileSync(join(path, 'glyphs/A_.glif'), 'utf8')
    const names = ['.notdef', 'A', 'V', 'a', 'v']
    for (const [index, name] of names.entries()) {
      const unicode =
        name === '.notdef'
          ? ''
          : `<unicode hex="${name.charCodeAt(0).toString(16).padStart(4, '0')}"/>`
      writeFileSync(
        join(path, 'glyphs', `glyph-${index}.glif`),
        outline
          .replace('name="A"', `name="${name}"`)
          .replace('<unicode hex="0041"/>', unicode),
      )
    }
    writeFileSync(
      join(path, 'glyphs/contents.plist'),
      plist(
        names
          .map(
            (name, index) =>
              `<key>${name}</key><string>glyph-${index}.glif</string>`,
          )
          .join(''),
      ),
    )
    return path
  }
  const first = makeUfo('Regular.ufo', 'Regular')
  let input = first
  if (twoMasters) {
    makeUfo('Bold.ufo', 'Bold')
    input = join(directory, 'P2a.designspace')
    writeFileSync(
      input,
      `<?xml version="1.0" encoding="UTF-8"?>
<designspace format="5.0"><axes><axis tag="wght" name="Weight" minimum="400" maximum="700" default="400"/></axes><sources>
<source filename="Regular.ufo" name="regular" familyname="P2a Browser" stylename="Regular"><location><dimension name="Weight" xvalue="400"/></location></source>
<source filename="Bold.ufo" name="bold" familyname="P2a Browser" stylename="Bold"><location><dimension name="Weight" xvalue="700"/></location></source>
</sources></designspace>`,
    )
  }
  return {
    directory,
    input,
    cleanup: () => rmSync(directory, { recursive: true, force: true }),
  }
}

export async function openSource(
  page: Page,
  input: string,
): Promise<DocSummary> {
  await openPathThroughDialog(page, input)
  const documents = await api<DocSummary[]>(page, 'session.list', {})
  const doc = documents.findLast((candidate) => candidate.path === input)
  if (!doc) throw new Error('source document did not open')
  return doc
}

type Source = ReturnType<typeof sourceFixture> & { doc: DocSummary }
export const test = base.extend<{ source: Source }>({
  source: async ({ page }, runTest) => {
    const files = sourceFixture(true)
    await page.goto('/')
    // Close only documents created by this source workflow. The outer session
    // fixture also cleans up when a test has already closed its page.
    const inherited = new Set(
      (await api<DocSummary[]>(page, 'session.list', {})).map(
        (document) => document.doc,
      ),
    )
    try {
      const doc = await openSource(page, files.input)
      await runTest({ ...files, doc })
    } finally {
      if (
        !page.isClosed() &&
        test.info().status !== test.info().expectedStatus
      ) {
        await page.screenshot({
          path: test.info().outputPath('before-cleanup.png'),
        })
        await test.info().attach('page-before-cleanup', {
          body: await page.locator('body').ariaSnapshot(),
          contentType: 'text/plain',
        })
      }
      try {
        if (!page.isClosed())
          for (const doc of await api<DocSummary[]>(page, 'session.list', {})) {
            if (!inherited.has(doc.doc))
              await api(page, 'session.close', { doc: doc.doc, force: true })
          }
      } finally {
        files.cleanup()
      }
    }
  },
})

export async function enterText(page: Page, text = 'AV') {
  await openGlyphFromGrid(page, 'A')
  const canvas = page.getByRole('img', { name: /^Glyph / })
  await canvas.press('t')
  const input = page.getByRole('textbox', { name: 'Text', exact: true })
  await expect(input).toBeFocused()
  await input.fill(text)
  await expect(page.locator('[data-text-mode]')).toHaveAttribute(
    'data-run-count',
    String(text.length),
  )
  return canvas
}

export function resolution(
  page: Page,
  doc: DocSummary,
  master = doc.masters[0].id,
) {
  return api<KerningResolution>(page, 'kerning.resolve', {
    doc: doc.doc,
    master,
    left: 'A',
    right: 'V',
  })
}

export async function expectAdvance(
  page: Page,
  doc: DocSummary,
  expected: number,
  kern = true,
  master = doc.masters[0].id,
) {
  await expect
    .poll(
      async () => {
        const run = await api<ShapedRun>(page, 'text.shape', {
          doc: doc.doc,
          master,
          text: 'AV',
          features: [['kern', kern]],
        })
        return run.stale
          ? null
          : run.glyphs.reduce((total, glyph) => total + glyph.x_advance, 0)
      },
      { timeout: 15_000 },
    )
    .toBe(expected)
}

export async function expectPreviewAdvance(page: Page, expected: number) {
  await expect
    .poll(() =>
      page.evaluate(() => {
        const run = (
          globalThis as unknown as {
            __fwTextRun?: { glyphs: { xAdvance: number }[] }
          }
        ).__fwTextRun
        return run?.glyphs.reduce((total, glyph) => total + glyph.xAdvance, 0)
      }),
    )
    .toBe(expected)
}

/** Hit the visible gap marker through the canvas's actual pointer handler. */
export async function selectFirstGap(page: Page, unitsPerEm: number) {
  const canvas = page.getByRole('img', { name: /^Glyph / })
  const point = await canvas.evaluate((element, upem) => {
    const globals = globalThis as unknown as {
      __fwTextRun?: {
        glyphs: { x: number; y: number }[]
        activeOrigin: { x: number; y: number }
      }
      __fwEditor?: {
        viewportForTest: {
          toScreen: (point: { x: number; y: number }) => {
            x: number
            y: number
          }
        }
      }
    }
    const run = globals.__fwTextRun
    const editor = globals.__fwEditor
    if (!run?.glyphs[1] || !editor) throw new Error('text gap is not ready')
    const at = editor.viewportForTest.toScreen({
      x: run.glyphs[1].x - run.activeOrigin.x,
      y: -upem * 0.12 - run.activeOrigin.y,
    })
    const bounds = (
      element as unknown as {
        getBoundingClientRect: () => { left: number; top: number }
      }
    ).getBoundingClientRect()
    return { x: bounds.left + at.x, y: bounds.top + at.y }
  }, unitsPerEm)
  await page.mouse.click(point.x, point.y)
  await expect(
    page.getByRole('combobox', { name: 'Kerning gap', exact: true }),
  ).toHaveValue('0')
  await expect(page.locator('[data-text-mode]')).toHaveAttribute(
    'data-text-mode',
    'text',
  )
}

export async function saveAs(page: Page, path: string) {
  await menuItem(page, 'File', /^Save As/)
  const dialog = page.getByRole('dialog', { name: 'Save As' })
  await expect(dialog).toBeVisible()
  await dialog.getByRole('textbox', { name: 'Document path' }).fill(path)
  await dialog.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(dialog).toBeHidden()
}

/** Pause only the next real kerning mutation, allowing a handover to race it. */
export async function holdNextKerning(page: Page, fail = false) {
  let release = () => {}
  const gate = new Promise<void>((resolve) => {
    release = resolve
  })
  let requested = false
  let captured = false
  let completed = () => {}
  const handled = new Promise<void>((resolve) => {
    completed = resolve
  })
  const handler = async (route: Route) => {
    const request = route.request()
    if (captured || !request.postData()?.includes('SetHorizontalKerning')) {
      await route.continue()
      return
    }
    captured = true
    requested = true
    await gate
    try {
      if (fail) {
        await route.fulfill({
          status: 400,
          contentType: 'application/json',
          body: JSON.stringify({
            error: {
              code: 'invalid_parameter',
              message: 'P2a delayed kerning failure',
            },
          }),
        })
      } else {
        await route.continue()
      }
    } finally {
      completed()
    }
  }
  await page.route('**/api/doc.*apply', handler)
  return {
    wait: () => expect.poll(() => requested).toBe(true),
    release,
    dispose: async () => {
      release()
      if (captured) await handled
      await page.unroute('**/api/doc.*apply', handler)
    },
  }
}
