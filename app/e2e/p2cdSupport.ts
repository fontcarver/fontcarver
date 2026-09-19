import { expect, type Page } from '@playwright/test'
import type { ExportReport } from '../src/api/generated/ExportReport'
import { menuItem, type LayerOutline } from './acceptanceSupport'

export const LATIN_LAYOUT =
  'feature liga { sub f i by f_i; } liga;\nfeature kern { pos A V -80; } kern;'

export async function chooseExport(page: Page, label: string, path: string) {
  await menuItem(page, 'File', /^Export/)
  const dialog = page.getByRole('dialog', { name: 'Export', exact: true })
  await dialog.getByRole('tab', { name: label, exact: true }).click()
  await dialog.getByRole('textbox', { name: 'Destination path' }).fill(path)
  return dialog
}

/** Exercise the real dialog and return the backend report it displays. */
export async function finishExport(page: Page, path: string) {
  const dialog = page.getByRole('dialog', { name: 'Export', exact: true })
  const response = page.waitForResponse('**/api/session.export')
  await dialog.getByRole('button', { name: 'Export', exact: true }).click()
  const envelope = (await (await response).json()) as {
    result?: ExportReport
    error?: unknown
  }
  expect(envelope.error).toBeUndefined()
  expect(envelope.result?.path).toBe(path)
  expect(envelope.result?.bytes_written).toBeGreaterThan(0)
  await expect(dialog.getByText(path, { exact: true })).toBeVisible()
  await expect(
    dialog.getByRole('button', { name: 'Export again' }),
  ).toBeVisible()
  return envelope.result!
}

export async function closeExport(page: Page) {
  const dialog = page.getByRole('dialog', { name: 'Export', exact: true })
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  await expect(dialog).toBeHidden()
}

/** Straight fixture outlines may rotate their starting point during encoding. */
export function outlineGeometry(outline: LayerOutline) {
  return {
    width: outline.width,
    contours: outline.contours.map((contour) => ({
      closed: contour.closed,
      points: contour.points
        .map(({ x, y, kind }) => ({ x, y, kind }))
        .sort((a, b) => a.x - b.x || a.y - b.y),
    })),
  }
}

export async function openProblems(page: Page) {
  await menuItem(page, 'Font', /^Find problems/)
  return page.getByRole('dialog', { name: 'Find problems', exact: true })
}
