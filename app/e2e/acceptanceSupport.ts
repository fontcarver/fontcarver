import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { basename, resolve } from 'node:path'

import { expect, type Locator, type Page } from '@playwright/test'

import { api, REPO_ROOT } from './support'

export interface DocSummary {
  doc: number
  path: string | null
  family_name: string
  glyph_count: number
}

export interface GlyphCell {
  gid: number
  name: string
}

export interface OutlinePoint {
  x: number
  y: number
  kind: string
}

export interface LayerOutline {
  layer: string
  width: number
  contours: {
    closed: boolean
    points: OutlinePoint[]
  }[]
}

export interface PointLocation {
  contour: number
  point: number
  x: number
  y: number
}

const RELEASE_CLI = resolve(REPO_ROOT, 'target', 'release', 'fontcarver')

/**
 * Runs the `fontcarver` CLI over an emitted font and returns its output.
 *
 * The release binary is used when the checkout has one, because these specs
 * read whole fonts back; the suite only promises a built `fc-devserver`, so a
 * checkout without it falls back to Cargo rather than failing with `ENOENT`.
 */
function runCli(args: readonly string[]): string {
  const options = {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  } as const
  return existsSync(RELEASE_CLI)
    ? execFileSync(RELEASE_CLI, args, options)
    : execFileSync(
        'cargo',
        ['run', '-q', '-p', 'fc-cli', '--bin', 'fontcarver', '--', ...args],
        options,
      )
}

export function releaseInspect(path: string): {
  summary: { glyph_count: number; family_name: string; units_per_em: number }
} {
  return JSON.parse(runCli(['inspect', '--json', path])) as ReturnType<
    typeof releaseInspect
  >
}

export function releaseGlyphs(path: string): string {
  return runCli(['glyphs', path, '--limit', '16'])
}

export async function menuItem(
  page: Page,
  menu: string,
  item: RegExp,
): Promise<void> {
  await page.getByRole('menuitem', { name: menu, exact: true }).click()
  await page
    .getByRole('menu', { name: menu })
    .getByRole('menuitem', { name: item })
    .click()
}

export async function docs(page: Page): Promise<DocSummary[]> {
  return api<DocSummary[]>(page, 'session.list', {})
}

export async function openPathThroughDialog(
  page: Page,
  path: string,
): Promise<DocSummary> {
  const before = new Set((await docs(page)).map((doc) => doc.doc))
  await menuItem(page, 'File', /^Open Font/)
  const dialog = page.getByRole('dialog', { name: 'Open Font' })
  await expect(dialog).toBeVisible()
  await dialog.getByRole('textbox', { name: 'File path' }).fill(path)
  await dialog.getByRole('button', { name: 'Open', exact: true }).click()
  await expect(dialog).toBeHidden()

  let opened: DocSummary | undefined
  await expect
    .poll(
      async () => {
        opened = (await docs(page)).find(
          (doc) => !before.has(doc.doc) && doc.path === path,
        )
        return opened?.doc
      },
      // Opening a 6,000-glyph font takes the debug kernel several seconds on a
      // busy machine, well past the five-second default.
      { timeout: 30_000 },
    )
    .toBeTruthy()
  // The pinned Overview tab is labelled "Overview · <file>", so match the file
  // name rather than requiring it to be the whole accessible name.
  const name = basename(path).replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')
  await expect(
    page.getByRole('tab', { name: new RegExp(`(?:^|· )${name}$`, 'u') }).last(),
  ).toHaveAttribute('aria-selected', 'true')
  return opened as DocSummary
}

export async function exportThroughDialog(
  page: Page,
  path: string,
): Promise<void> {
  await menuItem(page, 'File', /^Export/)
  const dialog = page.getByRole('dialog', { name: 'Export' })
  await expect(dialog).toBeVisible()
  // Bitmap-embedding formats add "CBDT (TTF)" and "sbix (TTF)" tabs, so the
  // plain TTF tab has to be matched exactly.
  await dialog.getByRole('tab', { name: 'TTF', exact: true }).click()
  await dialog.getByLabel('Destination path').fill(path)
  await dialog.getByRole('button', { name: 'Export', exact: true }).click()
  await expect(dialog.getByText(path)).toBeVisible({ timeout: 120_000 })
  await expect(
    dialog.getByRole('button', { name: 'Export again' }),
  ).toBeVisible()
  await dialog.getByRole('button', { name: 'Close' }).click()
  await expect(dialog).toBeHidden()
}

export async function glyphCell(
  page: Page,
  doc: number,
  name: string,
): Promise<GlyphCell> {
  const cells = await api<GlyphCell[]>(page, 'glyphs.search', {
    doc,
    query: name,
    limit: 8,
  })
  const cell = cells.find((candidate) => candidate.name === name)
  if (!cell) throw new Error(`missing glyph ${name}`)
  return cell
}

export async function outline(
  page: Page,
  doc: number,
  gid: number,
): Promise<LayerOutline> {
  return api<LayerOutline>(page, 'glyphs.outline', {
    doc,
    gid,
    layer: null,
  })
}

export async function openGlyphFromGrid(
  page: Page,
  name: string,
): Promise<void> {
  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const option = grid.getByRole('option', { name, exact: true })
  await expect(option).toBeVisible()
  await option.dblclick()
  await expect(
    page.getByRole('img', { name: new RegExp(`^Glyph ${name}`) }),
  ).toBeVisible()
}

export function firstOnCurve(value: LayerOutline): PointLocation {
  for (const [contour, entry] of value.contours.entries()) {
    const point = entry.points.findIndex(
      (candidate) => candidate.kind !== 'OffCurve',
    )
    if (point >= 0) {
      const found = entry.points[point] as OutlinePoint
      return { contour, point, x: found.x, y: found.y }
    }
  }
  throw new Error('outline has no on-curve point')
}

export async function movePointWithPointer(
  page: Page,
  doc: number,
  gid: number,
  location: PointLocation,
  delta: { x: number; y: number },
): Promise<PointLocation> {
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
    if (!editor) throw new Error('missing dev editor handle')
    const at = editor.viewportForTest.toScreen({ x, y })
    return { ...at, scale: editor.viewportForTest.scale }
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

  let moved: OutlinePoint | undefined
  await expect
    .poll(async () => {
      moved = (await outline(page, doc, gid)).contours[location.contour]
        ?.points[location.point]
      return moved ? [moved.x, moved.y] : null
    })
    .toEqual([location.x + delta.x, location.y + delta.y])
  return { ...location, x: moved?.x ?? 0, y: moved?.y ?? 0 }
}

export async function closePenContourWithPointer(
  page: Page,
  canvas: Locator,
): Promise<void> {
  const box = await canvas.boundingBox()
  if (!box) throw new Error('glyph canvas has no bounds')
  const first = await page.evaluate(() => {
    const editor = (
      globalThis as unknown as {
        __fwEditor?: {
          viewportForTest: {
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
          frame: () => { tool: { marks: { x: number; y: number }[] } }
        }
      }
    ).__fwEditor
    const point = editor?.frame().tool.marks[0]
    if (!editor || !point) throw new Error('pen has no first node')
    return editor.viewportForTest.toScreen(point)
  })
  await page.mouse.click(box.x + first.x, box.y + first.y)
}
