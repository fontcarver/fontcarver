import { expect, type Locator, type Page } from '@playwright/test'

/** Geometry only: E2E runs in Node, without adding DOM globals to its types. */
type Layout = {
  ownerDocument: { defaultView: { innerWidth: number } }
  querySelectorAll(selector: string): Iterable<Layout>
  getBoundingClientRect(): { left: number; right: number; width: number }
  textContent: string | null
  getAttribute(name: string): string | null
}

/** Long tables may scroll; ordinary controls must fit without sideways panning. */
export async function controlsFit(root: Locator) {
  const clipped = await root.evaluate((node) => {
    const element = node as unknown as Layout
    const width = element.ownerDocument.defaultView.innerWidth
    return [
      ...element.querySelectorAll('button, input, select, textarea, summary'),
    ]
      .filter((control) => {
        const rect = control.getBoundingClientRect()
        return rect.width > 0 && (rect.left < -1 || rect.right > width + 1)
      })
      .map(
        (control) => control.getAttribute('aria-label') ?? control.textContent,
      )
  })
  expect(clipped).toEqual([])
}

export async function footerReachable(dialog: Locator, button: Locator) {
  await expect(dialog).toBeInViewport({ ratio: 0.99 })
  await expect(button).toBeInViewport({ ratio: 0.99 })
}

/** Focusing/scanning a canvas must never pan the whole application sideways. */
export async function pageFits(page: Page) {
  const size = await page.evaluate(() => {
    const window = globalThis as unknown as {
      innerWidth: number
      scrollX: number
      document: { documentElement: { scrollWidth: number } }
    }
    return {
      width: window.innerWidth,
      content: window.document.documentElement.scrollWidth,
      x: window.scrollX,
    }
  })
  expect(size.content).toBeLessThanOrEqual(size.width + 1)
  expect(size.x).toBe(0)
}
