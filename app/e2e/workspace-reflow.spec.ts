import { expect } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import en from '../src/i18n/en.json' with { type: 'json' }
import zh from '../src/i18n/zh-CN.json' with { type: 'json' }
import { firstOnCurve, glyphCell, outline } from './acceptanceSupport'
import { controlsFit, pageFits } from './reflowSupport'
import { test } from './sessionFixtures'
import { fixture, requestApi } from './support'

test.use({ fixtureFont: fixture('DejaVuSansMin.woff2') })

for (const [language, labels] of [
  ['en', en],
  ['zh-CN', zh],
] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`keeps the workspace and font info reachable at 200%: ${language}/${theme}`, async ({
      page,
      request,
      isolatedSession,
    }) => {
      test.setTimeout(60_000)
      const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
      await requestApi(request, 'prefs.set', {
        prefs: { ...prefs, language, theme },
      })
      await page.setViewportSize({ width: 683, height: 384 })
      await page.goto('/')
      await expect(page.getByRole('tab').first()).toBeVisible()
      // Side panels must not leave only 135 CSS pixels for the editor.
      expect(
        (await page.getByRole('main').boundingBox())?.width,
      ).toBeGreaterThanOrEqual(680)
      const search = page.getByRole('searchbox', {
        name: labels.fontView.search,
      })
      await search.fill('A')
      await page
        .getByRole('listbox', { name: labels.fontView.gridLabel })
        .getByRole('option', { name: 'A', exact: true })
        .dblclick()
      const canvas = page.getByRole('img', {
        name: language === 'en' ? /^Glyph A,/ : /^字形 A，/,
      })
      await expect(canvas).toBeVisible()
      const doc = isolatedSession!.doc
      const { gid } = await glyphCell(page, doc, 'A')
      const before = await outline(page, doc, gid)

      for (const width of [683, 512]) {
        await page.setViewportSize({ width, height: 384 })
        expect((await page.getByRole('main').boundingBox())?.width).toBe(width)
        await controlsFit(
          page.getByRole('toolbar', { name: labels.glyphView.toolbar }),
        )
        await canvas.scrollIntoViewIfNeeded()
        await expect(canvas).toBeInViewport({ ratio: 0.99 })
        expect((await canvas.boundingBox())?.height).toBeGreaterThanOrEqual(240)
        await canvas.focus()
        await page.keyboard.press('Control+a')
        await page.keyboard.press('ArrowRight')
        await expect
          .poll(
            async () => (await outline(page, doc, gid)).contours[0].points[0].x,
          )
          .not.toBe(before.contours[0].points[0].x)
        await page.keyboard.press('Control+z')
        await expect.poll(() => outline(page, doc, gid)).toEqual(before)
        await pageFits(page)

        // One selected node has more fields than a multi-selection. Its header
        // and collapse control must stay inside the short canvas as well.
        const info = page.getByRole('region', {
          name: labels.infoBox.label,
          exact: true,
        })
        const toggleInfo = info.getByRole('button', {
          name: labels.infoBox.toggle,
        })
        if ((await toggleInfo.getAttribute('aria-expanded')) === 'true')
          await toggleInfo.click()
        await canvas.scrollIntoViewIfNeeded()
        const point = firstOnCurve(before)
        const position = await page.evaluate((point) => {
          const editor = (
            globalThis as unknown as {
              __fwEditor: {
                viewportForTest: {
                  toScreen(p: { x: number; y: number }): {
                    x: number
                    y: number
                  }
                }
              }
            }
          ).__fwEditor
          return editor.viewportForTest.toScreen(point)
        }, point)
        const box = (await canvas.boundingBox())!
        await page.mouse.click(box.x + box.width - 20, box.y + box.height - 20)
        await page.mouse.click(box.x + position.x, box.y + position.y)
        await toggleInfo.click()
        await expect(
          info.getByRole('heading', {
            name: labels.infoBox.title.node,
            exact: true,
          }),
        ).toBeInViewport({ ratio: 0.99 })
        await expect(info).toBeInViewport({ ratio: 0.99 })
        await info.getByRole('textbox').last().focus()
        await expect(info.getByRole('textbox').last()).toBeInViewport({
          ratio: 0.99,
        })

        const tools = page.getByRole('toolbar', {
          name: labels.tool.palette,
          exact: true,
        })
        await tools.getByRole('button').first().focus()
        await page.keyboard.press('End')
        await expect(tools.getByRole('button').last()).toBeFocused()
        await expect(tools.getByRole('button').last()).toBeInViewport({
          ratio: 0.99,
        })

        for (const [toggle, title] of [
          [labels.a11y.toggleSidebar, labels.a11y.sidebar],
          [labels.a11y.toggleInspector, labels.a11y.inspector],
        ]) {
          const opener = page.getByRole('button', { name: toggle, exact: true })
          await expect(opener).toBeInViewport({ ratio: 0.99 })
          await opener.click()
          const panel = page.getByRole('dialog', { name: title, exact: true })
          await expect(panel).toBeVisible()
          await expect(opener).toHaveAttribute('aria-pressed', 'true')
          await controlsFit(panel)
          const close = panel.getByRole('button', {
            name: labels.common.close,
            exact: true,
          })
          await expect(close).toBeInViewport({ ratio: 0.99 })
          await close.focus()
          await page.keyboard.press('Tab')
          await expect(panel.getByRole('button').first()).toBeFocused()
          await page.keyboard.press('Shift+Tab')
          await expect(close).toBeFocused()
          await page.keyboard.press('Escape')
          await expect(panel).toBeHidden()
          await expect(opener).toBeFocused()
        }
      }

      // Crossing the breakpoint restores both saved docks and their widths.
      await page.setViewportSize({ width: 1366, height: 768 })
      await expect(
        page.getByRole('separator', { name: labels.a11y.sidebarResize }),
      ).toHaveAttribute('aria-valuenow', '240')
      await expect(
        page.getByRole('separator', { name: labels.a11y.inspectorResize }),
      ).toHaveAttribute('aria-valuenow', '300')
      await page.setViewportSize({ width: 683, height: 384 })
      await page.keyboard.press('Control+i')
      const toolbar = page.getByRole('toolbar', {
        name: labels.fontInfo.toolbar,
      })
      for (const section of Object.values(labels.fontInfo.section)) {
        await expect(
          toolbar.getByRole('tab', { name: section, exact: true }),
        ).toBeInViewport({ ratio: 0.99 })
      }
      await expect(toolbar.getByRole('searchbox')).toBeInViewport({
        ratio: 0.99,
      })
    })
  }
}
