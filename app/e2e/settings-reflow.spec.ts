import { expect, type Locator } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import { sourceTest as test } from './sessionFixtures'
import { requestApi } from './support'

// Browser layout fields, without adding DOM globals to the Node E2E project.
type LayoutElement = {
  tagName: string
  className: string
  textContent: string | null
  scrollWidth: number
  clientWidth: number
  querySelectorAll(selector: string): Iterable<LayoutElement>
}

async function noHorizontalOverflow(dialog: Locator) {
  const overflow = await dialog.evaluate((element) => {
    const root = element as unknown as LayoutElement
    return [...root.querySelectorAll('*')]
      .filter(
        (part) =>
          !['INPUT', 'SELECT', 'OPTION'].includes(part.tagName) &&
          part.clientWidth > 0 &&
          part.scrollWidth > part.clientWidth + 1,
      )
      .map((part) => ({
        className: part.className,
        text: part.textContent?.slice(0, 80),
        width: part.clientWidth,
        contentWidth: part.scrollWidth,
      }))
  })
  expect(overflow).toEqual([])
}

const languages = [
  {
    language: 'en',
    open: 'Open Font…',
    preferences: 'Preferences',
    categories: ['General', 'Editor', 'Snapping'],
    done: 'Done',
    reset: 'Reset to defaults',
    shortcuts: 'Keyboard Shortcuts',
    apply: 'Apply',
    cancel: 'Cancel',
    restore: 'Restore Open Font…',
  },
  {
    language: 'zh-CN',
    open: '打开字体…',
    preferences: '偏好设置',
    categories: ['通用', '编辑器', '吸附'],
    done: '完成',
    reset: '恢复默认设置',
    shortcuts: '键盘快捷键',
    apply: '应用',
    cancel: '取消',
    restore: '恢复打开字体…',
  },
] as const

for (const labels of languages) {
  for (const theme of ['light', 'dark'] as const) {
    test(`keeps preferences and shortcut settings usable at 200% reflow: ${labels.language}/${theme}`, async ({
      page,
      request,
    }) => {
      const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
      await requestApi(request, 'prefs.set', {
        prefs: { ...prefs, language: labels.language, theme },
      })
      await page.goto('/')
      const opener = page.getByRole('button', {
        name: labels.open,
        exact: true,
      })

      // Equivalent CSS sizes for actual 200% zoom, a narrower window and 100%.
      for (const [width, height] of [
        [683, 384],
        [512, 384],
        [1366, 768],
      ]) {
        await page.setViewportSize({ width, height })
        await opener.focus()
        await page.keyboard.press('Control+,')
        const preferences = page.getByRole('dialog', {
          name: labels.preferences,
          exact: true,
        })
        const first = preferences.getByRole('tab').first()
        const done = preferences.getByRole('button', {
          name: labels.done,
          exact: true,
        })
        await expect(first).toBeFocused()
        await page.keyboard.press('Shift+Tab')
        await expect(done).toBeFocused()
        await page.keyboard.press('Tab')
        await expect(first).toBeFocused()
        for (const category of labels.categories) {
          await preferences.getByRole('tab', { name: category }).click()
          await noHorizontalOverflow(preferences)
          await expect(preferences).toBeInViewport({ ratio: 0.99 })
          await expect(done).toBeInViewport({ ratio: 0.99 })
          await expect(
            preferences.getByRole('button', { name: labels.reset }),
          ).toBeInViewport({ ratio: 0.99 })
          // Native focus scrolling must expose the last field above the footer.
          const last = preferences.getByRole('spinbutton').last()
          await last.focus()
          await expect(last).toBeInViewport({ ratio: 0.99 })
          const field = await last.boundingBox()
          const footer = await done.boundingBox()
          expect((field?.y ?? 0) + (field?.height ?? 0)).toBeLessThanOrEqual(
            footer?.y ?? 0,
          )
        }
        await page.keyboard.press('Escape')
        await expect(preferences).toBeHidden()
        await expect(opener).toBeFocused()

        const shortcutOpener = page.getByRole('button', {
          name: labels.shortcuts,
          exact: true,
        })
        await shortcutOpener.click()
        const shortcuts = page.getByRole('dialog', {
          name: labels.shortcuts,
          exact: true,
        })
        const preset = shortcuts.getByRole('combobox')
        const apply = shortcuts.getByRole('button', {
          name: labels.apply,
          exact: true,
        })
        const cancel = shortcuts.getByRole('button', {
          name: labels.cancel,
          exact: true,
        })
        await expect(preset).toBeFocused()
        await page.keyboard.press('Shift+Tab')
        await expect(apply).toBeFocused()
        await page.keyboard.press('Tab')
        await expect(preset).toBeFocused()
        await noHorizontalOverflow(shortcuts)
        const bindings = shortcuts.getByRole('group')
        await expect(bindings.getByRole('textbox')).toHaveCount(80)
        await bindings.getByRole('textbox').last().focus()
        await page.keyboard.press('Tab')
        await expect(bindings.getByRole('button').last()).toBeFocused()
        await expect(bindings.getByRole('button').last()).toBeInViewport({
          ratio: 0.99,
        })
        await page.keyboard.press('Tab')
        await expect(cancel).toBeFocused()

        const search = shortcuts.getByRole('searchbox')
        await search.fill('file.open')
        const binding = bindings.locator('[id="binding-file.open"]')
        await expect(binding).toHaveValue('Mod+O')
        await binding.fill('Mod+S')
        await expect(apply).toBeDisabled()
        await expect(shortcuts.getByRole('alert')).toBeVisible()
        await noHorizontalOverflow(shortcuts)
        await expect(cancel).toBeInViewport({ ratio: 0.99 })
        await expect(apply).toBeInViewport({ ratio: 0.99 })
        await bindings.getByRole('button', { name: labels.restore }).click()
        await expect(apply).toBeEnabled()
        await expect(binding).toHaveValue('Mod+O')
        await binding.fill('Mod+Alt+Shift+9')
        await page.keyboard.press('Escape')
        await expect(shortcuts).toBeHidden()
        await expect(shortcutOpener).toBeFocused()
        await page.keyboard.press('Enter')
        await shortcuts.getByRole('searchbox').fill('file.open')
        await expect(binding).toHaveValue('Mod+O')
        await shortcuts.getByRole('button', { name: labels.cancel }).click()
      }
    })
  }
}
