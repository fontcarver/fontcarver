import { expect } from '@playwright/test'
import { sourceTest as test } from './sessionFixtures'

import { api } from './support'
import {
  glyphCell,
  openGlyphFromGrid,
  outline,
  type LayerOutline,
} from './acceptanceSupport'

const POINTS = 300
const WARM_SAMPLES = 20
const MEASURED_SAMPLES = 200

function circlePoints() {
  return Array.from({ length: POINTS }, (_, index) => {
    const angle = (index / POINTS) * Math.PI * 2
    return {
      x: 500 + Math.cos(angle) * 300,
      y: 400 + Math.sin(angle) * 300,
      kind: 'Line',
      smooth: false,
      name: null,
      tt_index: 'Unnumbered',
    }
  })
}

function percentile(sorted: readonly number[], percent: number): number {
  const rank = Math.ceil((sorted.length * percent) / 100)
  return sorted[Math.max(0, rank - 1)] ?? 0
}

test('300-point drag repaints synchronously within eight milliseconds', async ({
  page,
}) => {
  test.setTimeout(120_000)
  const family = `Pointer Performance ${Date.now()}`
  await page.goto('/')
  const created = await api<{ doc: number }>(page, 'session.new', {
    family,
    units_per_em: 1000,
  })
  try {
    const added = await api<{ added: { gid: number; name: string }[] }>(
      page,
      'glyphs.add',
      { doc: created.doc, recipe: 'A', master: null, dry_run: false },
    )
    const glyph = added.added[0]
    if (!glyph) throw new Error('performance glyph was not added')
    const empty = await outline(page, created.doc, glyph.gid)
    await api(page, 'doc.apply', {
      doc: created.doc,
      command: {
        Batch: [
          {
            AddContour: {
              glyph: 'A',
              layer: empty.layer,
              contour: {
                closed: true,
                points: circlePoints(),
                name: null,
                id: null,
              },
            },
          },
          {
            SetWidth: { glyph: 'A', layer: empty.layer, width: 1000 },
          },
        ],
      },
      label: 'Build performance glyph',
    })
    await page.reload()
    await page.getByRole('tab', { name: family }).click()
    await openGlyphFromGrid(page, 'A')
    const cell = await glyphCell(page, created.doc, 'A')
    const before = await outline(page, created.doc, cell.gid)
    expect(before.contours[0]?.points).toHaveLength(POINTS)
    const first = before.contours[0]?.points[0]
    if (!first) throw new Error('performance outline has no first point')

    // Pointer-up closes asynchronously after queued HTTP applies drain. Capture
    // that closure before the gesture; timing samples still measure only paint.
    const [closed, measurement] = await Promise.all([
      page.waitForResponse(
        (response) =>
          response.url().endsWith('/api/doc.edit_end') &&
          (response.request().postDataJSON() as { doc: number }).doc ===
            created.doc,
        // The test's 120-second deadline bounds both frame-paced sampling and
        // queue draining; do not expire this listener before pointer-up.
        { timeout: 0 },
      ),
      page.getByRole('img', { name: /^Glyph A/ }).evaluate(
        async (canvas, setup) => {
          const globals = globalThis as unknown as {
            __fwEditor?: {
              dragging: boolean
              viewportForTest: {
                scale: number
                toScreen: (point: { x: number; y: number }) => {
                  x: number
                  y: number
                }
              }
              frame: () => {
                contours: LayerOutline['contours'] | null
              }
            }
            __fwPaint?: {
              samples: number[]
              count: number
              reset: () => void
            }
          }
          const editor = globals.__fwEditor
          if (!editor) throw new Error('missing dev editor handle')
          const paintStats = globals.__fwPaint
          if (!paintStats) throw new Error('missing dev paint statistics')
          const first = setup.first
          const target = canvas as unknown as {
            getBoundingClientRect: () => {
              left: number
              top: number
            }
            dispatchEvent: (event: unknown) => boolean
          }
          const BrowserPointerEvent = (
            globalThis as unknown as {
              PointerEvent: new (
                type: string,
                init: Record<string, unknown>,
              ) => unknown
            }
          ).PointerEvent
          const nextAnimationFrame = (
            globalThis as unknown as {
              requestAnimationFrame: (callback: () => void) => number
            }
          ).requestAnimationFrame
          const rect = target.getBoundingClientRect()
          const origin = editor.viewportForTest.toScreen(first)
          const dispatch = (
            type: 'pointerdown' | 'pointermove' | 'pointerup',
            dx: number,
            dy: number,
          ) =>
            target.dispatchEvent(
              new BrowserPointerEvent(type, {
                bubbles: true,
                cancelable: true,
                pointerId: 41,
                pointerType: 'mouse',
                isPrimary: true,
                button: 0,
                buttons: type === 'pointerup' ? 0 : 1,
                ctrlKey: true,
                clientX: rect.left + origin.x + dx,
                clientY: rect.top + origin.y + dy,
              }),
            )

          dispatch('pointermove', 0, 0)
          const downStarted = performance.now()
          dispatch('pointerdown', 0, 0)
          const pointerDown = performance.now() - downStarted
          if (!editor.dragging)
            throw new Error('pointer-down did not start a drag')
          paintStats.reset()

          let previousX = first.x
          let previousY = first.y
          const move = async (sample: number): Promise<number> => {
            await new Promise<void>((resolve) =>
              nextAnimationFrame(() => resolve()),
            )
            const dx = 4 + sample * 0.03
            const dy = 2 + sample * 0.01
            const paintsBefore = paintStats.count
            const started = performance.now()
            dispatch('pointermove', dx, dy)
            const elapsed = performance.now() - started
            if (paintStats.count <= paintsBefore)
              throw new Error(`sample ${sample} did not repaint synchronously`)
            const current = editor.frame().contours?.[0]?.points[0]
            if (!current) throw new Error('drag lost its preview')
            if (current.x === previousX && current.y === previousY)
              throw new Error(
                `sample ${sample} did not change preview coordinates`,
              )
            previousX = current.x
            previousY = current.y
            return elapsed
          }
          for (let sample = 1; sample <= setup.warm; sample += 1)
            await move(sample)
          const samples: number[] = []
          for (let index = 0; index < setup.measured; index += 1)
            samples.push(await move(setup.warm + index + 1))
          const final = editor.frame().contours?.[0]?.points[0]
          const paint = [...paintStats.samples]
          const paintCount = paintStats.count
          dispatch(
            'pointerup',
            4 + (setup.warm + setup.measured) * 0.03,
            2 + (setup.warm + setup.measured) * 0.01,
          )
          return { pointerDown, samples, paint, paintCount, final }
        },
        { warm: WARM_SAMPLES, measured: MEASURED_SAMPLES, first },
      ),
    ])
    expect(closed.ok(), 'the gesture closed successfully').toBe(true)

    expect(
      measurement.final,
      'last pointer move produced a preview',
    ).toBeTruthy()
    const final = measurement.final as { x: number; y: number }
    await expect
      .poll(async () => {
        const persisted = await outline(page, created.doc, cell.gid)
        const point = persisted.contours[0]?.points[0]
        return point ? [point.x, point.y] : null
      })
      .toEqual([final.x, final.y])

    const sorted = [...measurement.samples].sort((a, b) => a - b)
    expect(measurement.samples).toHaveLength(MEASURED_SAMPLES)
    expect(measurement.paintCount).toBeGreaterThanOrEqual(
      WARM_SAMPLES + MEASURED_SAMPLES,
    )
    expect(measurement.paint).not.toHaveLength(0)
    const median = percentile(sorted, 50)
    const p95 = percentile(sorted, 95)
    const max = sorted.at(-1) ?? 0
    const overBudget = measurement.samples.filter((sample) => sample > 8).length
    const paint = [...measurement.paint].sort((a, b) => a - b)
    const rendererMax = paint.at(-1) ?? 0
    console.log(
      `pointer-to-paint 300 points/${MEASURED_SAMPLES} frame-paced samples: median=${median.toFixed(3)}ms p95=${p95.toFixed(3)}ms max=${max.toFixed(3)}ms over8=${overBudget} pointerDown=${measurement.pointerDown.toFixed(3)}ms rendererP95=${percentile(paint, 95).toFixed(3)}ms rendererMax=${rendererMax.toFixed(3)}ms`,
    )
    expect(measurement.pointerDown).toBeLessThanOrEqual(8)
    expect(p95).toBeLessThanOrEqual(8)
    expect(max).toBeLessThanOrEqual(8)
    expect(final.x).not.toBe(before.contours[0]?.points[0]?.x)
    expect(final.y).not.toBe(before.contours[0]?.points[0]?.y)

    await page.keyboard.press('Control+z')
    await expect
      .poll(async () => {
        const restored = await outline(page, created.doc, cell.gid)
        const point = restored.contours[0]?.points[0]
        return point ? [point.x, point.y] : null
      })
      .toEqual([
        before.contours[0]?.points[0]?.x,
        before.contours[0]?.points[0]?.y,
      ])
  } finally {
    await api(page, 'session.close', { doc: created.doc, force: true })
  }
})
