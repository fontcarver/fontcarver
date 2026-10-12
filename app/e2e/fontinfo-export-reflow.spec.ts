import { expect } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import en from '../src/i18n/en.json' with { type: 'json' }
import zh from '../src/i18n/zh-CN.json' with { type: 'json' }
import { controlsFit, footerReachable, pageFits } from './reflowSupport'
import { test } from './sessionFixtures'
import { fixture, inspect, requestApi } from './support'

test.use({ fixtureFont: fixture('DejaVuSansMin.woff2') })

for (const [language, labels] of [
  ['en', en],
  ['zh-CN', zh],
] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`reflows font information and exports a usable font: ${language}/${theme}`, async ({
      page,
      request,
      isolatedSession,
    }, testInfo) => {
      test.setTimeout(90_000)
      const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
      await requestApi(request, 'prefs.set', {
        prefs: { ...prefs, language, theme },
      })
      await page.setViewportSize({ width: 683, height: 384 })
      await page.goto('/')
      await expect(page.getByRole('tab').first()).toBeVisible()
      await page.keyboard.press('Control+i')
      const toolbar = page.getByRole('toolbar', {
        name: labels.fontInfo.toolbar,
      })

      for (const width of [683, 512, 1366]) {
        await page.setViewportSize({
          width,
          height: width === 1366 ? 768 : 384,
        })
        for (const section of Object.values(labels.fontInfo.section)) {
          const tab = toolbar.getByRole('tab', { name: section, exact: true })
          await expect(tab).toBeInViewport({ ratio: 0.99 })
          await tab.click()
          await expect(tab).toHaveAttribute('aria-selected', 'true')
          await controlsFit(page.getByRole('main'))
        }
        const search = toolbar.getByRole('searchbox')
        await expect(search).toBeInViewport({ ratio: 0.99 })
        await search.fill('DejaVu')
        await expect(
          page
            .getByRole('main')
            .getByText('DejaVu Sans', { exact: true })
            .first(),
        ).toBeVisible()
        await search.fill('')

        const metadataOpener = toolbar.getByRole('button', {
          name: labels.fontMetadata.edit,
        })
        await metadataOpener.click()
        const metadata = page.getByRole('dialog', {
          name: labels.fontMetadata.title,
        })
        await expect(metadata.getByRole('textbox').first()).toBeVisible()
        await controlsFit(metadata)
        const closeMetadata = metadata.getByRole('button', {
          name: labels.common.close,
          exact: true,
        })
        await metadata.getByRole('textbox').last().focus()
        await expect(metadata.getByRole('textbox').last()).toBeInViewport({
          ratio: 0.99,
        })
        await footerReachable(metadata, closeMetadata)
        const note = metadata.getByRole('textbox', {
          name: labels.fontInfo.field.note,
          exact: true,
        })
        await note.fill(`Zoom verification ${width}`)
        const apply = metadata.getByRole('button', {
          name: labels.variation.apply,
          exact: true,
        })
        await apply.click()
        await expect(apply).toBeDisabled()
        await expect(closeMetadata).toBeEnabled()
        await expect(closeMetadata).toBeFocused()
        await page.keyboard.press('Escape')
        await expect(metadata).toBeHidden()
        await expect(metadataOpener).toBeFocused()
        await metadataOpener.click()
        await expect(note).toHaveValue(`Zoom verification ${width}`)
        await closeMetadata.click()

        await page.keyboard.press('Control+e')
        const dialog = page.getByRole('dialog', {
          name: labels.dialog.export.title,
          exact: true,
        })
        const run = dialog.getByRole('button', {
          name: labels.dialog.export.run,
          exact: true,
        })
        await footerReachable(dialog, run)
        await controlsFit(dialog)
        // Every format is reachable through the body while its footer stays put.
        for (const format of [
          'Ttf',
          'Otf',
          'Woff2',
          'Eps',
          'Ufo',
          'Fcf',
        ] as const) {
          await dialog
            .getByRole('tab', {
              name: labels.dialog.export.formatName[format],
              exact: true,
            })
            .click()
          await controlsFit(dialog)
          await footerReachable(dialog, run)
        }
        await dialog.getByRole('tab', { name: 'TTF', exact: true }).click()
        const path = testInfo.outputPath(`zoom-${width}.ttf`)
        await dialog
          .getByRole('textbox', { name: labels.dialog.export.destinationLabel })
          .fill(path)
        await run.click()
        await expect(
          dialog.getByRole('button', {
            name: labels.dialog.export.again,
            exact: true,
          }),
        ).toBeEnabled({ timeout: 30_000 })
        expect(inspect(path).summary.glyph_count).toBe(
          isolatedSession!.glyph_count,
        )
        await controlsFit(dialog)
        await dialog
          .getByRole('button', { name: labels.common.close, exact: true })
          .click()
        await expect(dialog).toBeHidden()
        await pageFits(page)
      }
    })
  }
}
