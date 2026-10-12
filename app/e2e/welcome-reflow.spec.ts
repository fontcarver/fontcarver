import { expect } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import { sourceTest as test } from './sessionFixtures'
import { requestApi } from './support'

// The E2E project runs under Node types; only these browser layout fields are
// read inside evaluate, so it does not need DOM globals in its configuration.
type LayoutElement = {
  scrollWidth: number
  clientWidth: number
  textContent: string | null
  children: Iterable<LayoutElement>
  querySelectorAll(selector: string): Iterable<LayoutElement>
  getBoundingClientRect(): {
    left: number
    right: number
    top: number
    bottom: number
  }
}

// 1366×768 at 200% browser zoom has a 683×384 CSS viewport. These cases
// guard reflow; the separate agent-browser acceptance uses actual page zoom.
test.use({ viewport: { width: 683, height: 384 } })

const languages = [
  {
    language: 'en',
    open: 'Open Font…',
    next: [
      'New Font…',
      'Getting started',
      /^Command Palette…/,
      'Keyboard Shortcuts',
    ],
    palette: 'Commands',
    scope: 'Search in',
    shortcuts: 'Keyboard Shortcuts',
  },
  {
    language: 'zh-CN',
    open: '打开字体…',
    next: ['新建字体…', '入门', /^命令面板/, '键盘快捷键'],
    palette: '命令',
    scope: '搜索范围',
    shortcuts: '键盘快捷键',
  },
] as const

for (const labels of languages) {
  for (const theme of ['light', 'dark'] as const) {
    test(`keeps welcome and palette readable at 200% reflow: ${labels.language}/${theme}`, async ({
      page,
      request,
    }) => {
      const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
      await requestApi(request, 'prefs.set', {
        prefs: { ...prefs, language: labels.language, theme },
      })
      await page.goto('/')
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme)

      await page.getByRole('button', { name: labels.open, exact: true }).focus()
      for (const name of labels.next) {
        await page.keyboard.press('Tab')
        const button = page.getByRole('button', { name, exact: true })
        await expect(button).toBeFocused()
        // Fractional scroll positions can clip less than one CSS pixel.
        await expect(button).toBeInViewport({ ratio: 0.98 })
      }
      await page.keyboard.press('Shift+Tab')
      const opener = page.getByRole('button', {
        name: labels.next[2],
      })
      await expect(opener).toBeFocused()
      await page.keyboard.press('Enter')

      const dialog = page.getByRole('dialog', {
        name: labels.palette,
        exact: true,
      })
      const search = dialog.getByRole('combobox', {
        name: labels.palette,
        exact: true,
      })
      const scope = dialog.getByRole('combobox', {
        name: labels.scope,
        exact: true,
      })
      const list = dialog.getByRole('listbox')
      await expect(search).toBeFocused()
      await expect(dialog).toBeInViewport({ ratio: 0.98 })
      await expect
        .poll(() => list.getByRole('option').count())
        .toBeGreaterThan(100)

      // Check every rendered row, including long unavailable reasons and tool
      // gesture hints. Neither a horizontal scrollbar nor ellipsis may hide it.
      const overflow = await list.evaluate((element) => {
        const results = element as unknown as LayoutElement
        const failures: string[] = []
        if (results.scrollWidth > results.clientWidth + 1)
          failures.push('result list overflows horizontally')
        for (const row of results.querySelectorAll('button')) {
          const bounds = row.getBoundingClientRect()
          for (const part of row.children) {
            const rect = part.getBoundingClientRect()
            if (
              part.scrollWidth > part.clientWidth + 1 ||
              rect.left < bounds.left - 1 ||
              rect.right > bounds.right + 1 ||
              rect.top < bounds.top - 1 ||
              rect.bottom > bounds.bottom + 1
            )
              failures.push(part.textContent ?? 'unlabelled content')
          }
        }
        return failures
      })
      expect(overflow).toEqual([])

      // Wrapping changes row heights: keyboard selection must still scroll the
      // last/first result into view and keep the search and scope reachable.
      await page.keyboard.press('ArrowUp')
      await expect(list.getByRole('option').last()).toHaveAttribute(
        'aria-selected',
        'true',
      )
      await expect(list.getByRole('option').last()).toBeInViewport({
        ratio: 0.98,
      })
      await page.keyboard.press('ArrowDown')
      await expect(list.getByRole('option').first()).toHaveAttribute(
        'aria-selected',
        'true',
      )
      await expect(list.getByRole('option').first()).toBeInViewport({
        ratio: 0.98,
      })
      await page.keyboard.press('Tab')
      await expect(scope).toBeFocused()
      await page.keyboard.press('Tab')
      await expect(search).toBeFocused()
      await page.keyboard.press('Shift+Tab')
      await expect(scope).toBeFocused()
      await page.keyboard.press('Shift+Tab')
      await expect(search).toBeFocused()

      await search.fill(labels.shortcuts)
      await expect(list.getByRole('option')).toHaveCount(1)
      await page.keyboard.press('Enter')
      const settings = page.getByRole('dialog', {
        name: labels.shortcuts,
        exact: true,
      })
      await expect(settings).toBeVisible()
      await page.keyboard.press('Escape')
      await expect(settings).toBeHidden()
      await expect(opener).toBeFocused()
      await expect(opener).toBeInViewport({ ratio: 0.98 })
    })
  }
}
