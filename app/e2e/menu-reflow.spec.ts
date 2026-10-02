import { expect, type Locator } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import { sourceTest as test } from './sessionFixtures'
import { requestApi } from './support'

async function fitsViewport(element: Locator, width: number, height: number) {
  await expect(element).toBeVisible()
  await expect
    .poll(async () => {
      const box = await element.boundingBox()
      return (
        !!box &&
        box.x >= 0 &&
        box.y >= 0 &&
        box.x + box.width <= width + 1 &&
        box.y + box.height <= height + 1
      )
    })
    .toBe(true)
}

for (const language of ['en', 'zh-CN'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`keeps Help and its popup inside the zoomed menu bar: ${language}/${theme}`, async ({
      page,
      request,
    }) => {
      const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
      await requestApi(request, 'prefs.set', {
        prefs: { ...prefs, language, theme },
      })
      await page.setViewportSize({ width: 683, height: 384 })
      await page.goto('/')
      const bar = page.getByRole('menubar')
      const titles = bar.locator('[data-menu]')
      const help = bar.locator('[data-menu=help]')
      const file = bar.locator('[data-menu=file]')
      const helpName = language === 'en' ? 'Help' : '帮助'
      const fileName = language === 'en' ? 'File' : '文件'
      const shortcutsName =
        language === 'en' ? 'Keyboard Shortcuts' : '键盘快捷键'
      const returnTarget = page.getByRole('button', {
        name: language === 'en' ? 'Open Font…' : '打开字体…',
        exact: true,
      })
      await expect(titles).toHaveCount(11)

      // 683×384 is 1366×768 at 200% browser zoom. The narrower and ordinary
      // sizes also exercise wrapping and repositioning an already open menu.
      for (const [width, height] of [
        [683, 384],
        [512, 384],
        [1366, 768],
      ]) {
        await page.setViewportSize({ width, height })
        for (const title of await titles.all())
          await fitsViewport(title, width, height)

        await returnTarget.focus()
        await help.click()
        const popup = page.getByRole('menu', { name: helpName, exact: true })
        await fitsViewport(popup, width, height)
        const anchor = await help.boundingBox()
        const bounds = await popup.boundingBox()
        expect(bounds?.y).toBeCloseTo(
          (anchor?.y ?? 0) + (anchor?.height ?? 0),
          0,
        )
        await popup
          .getByRole('menuitem', { name: shortcutsName, exact: true })
          .click()
        const settings = page.getByRole('dialog', {
          name: shortcutsName,
          exact: true,
        })
        await expect(settings).toBeVisible()
        await page.keyboard.press('Escape')
        await expect(settings).toBeHidden()
        await expect(returnTarget).toBeFocused()

        await help.focus()
        await page.keyboard.press('Enter')
        await expect(popup.getByRole('menuitem').first()).toBeFocused()
        await page.keyboard.press('End')
        await expect(popup.getByRole('menuitem').last()).toBeFocused()
        await page.keyboard.press('ArrowRight')
        const filePopup = page.getByRole('menu', {
          name: fileName,
          exact: true,
        })
        await fitsViewport(filePopup, width, height)
        await page.keyboard.press('Escape')
        await expect(file).toBeFocused()
        await page.keyboard.press('ArrowLeft')
        await expect(help).toBeFocused()
        await page.keyboard.press('Enter')
        await fitsViewport(popup, width, height)
        await page.keyboard.press('Tab')
        await expect(popup).toBeHidden()
        await expect(help).not.toBeFocused()
      }

      await help.click()
      await page.setViewportSize({ width: 683, height: 384 })
      const popup = page.getByRole('menu', { name: helpName, exact: true })
      await fitsViewport(popup, 683, 384)
      await page.setViewportSize({ width: 512, height: 384 })
      await fitsViewport(popup, 512, 384)
      await popup.getByRole('menuitem').last().hover()
      await expect(popup).toBeVisible()
      await page.keyboard.press('Escape')
      await expect(popup).toBeHidden()
    })
  }
}
