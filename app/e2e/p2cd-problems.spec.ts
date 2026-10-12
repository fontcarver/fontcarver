import { expect } from '@playwright/test'
import { api } from './support'
import { featureState, holdApi, test } from './p2bSupport'
import { openProblems } from './p2cdSupport'

test('Find problems is read-only, navigates to a glyph, and refreshes an invalidated report', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await api(page, 'doc.apply', {
    doc,
    label: 'Duplicate Latin Unicode',
    command: { SetUnicodes: { glyph: 'V', unicodes: [0x41] } },
  })
  const before = await featureState(page, doc)
  let dialog = await openProblems(page)
  const issues = dialog.getByRole('list', { name: 'Problems', exact: true })
  await expect(issues).toContainText('duplicate-unicode')
  expect((await featureState(page, doc)).generation).toBe(before.generation)
  await issues
    .getByRole('button', { name: 'Open glyph V', exact: true })
    .click()
  await expect(dialog).toBeHidden()
  await expect(page.getByRole('img', { name: /^Glyph V/ })).toBeVisible()
  dialog = await openProblems(page)
  await expect(
    dialog.getByRole('button', { name: 'Open glyph V' }),
  ).toBeEnabled()
  await api(page, 'doc.apply', {
    doc,
    label: 'Restore Latin Unicode',
    command: { SetUnicodes: { glyph: 'V', unicodes: [0x56] } },
  })
  await expect(dialog).toContainText('The document changed. Refresh')
  await expect(
    dialog.getByRole('button', { name: 'Open glyph V' }),
  ).toBeDisabled()
  await dialog.getByRole('button', { name: 'Refresh', exact: true }).click()
  await expect(dialog).toContainText('No problems found.')
  await expect(dialog).not.toContainText('duplicate-unicode')
  await expect(dialog).not.toContainText('The document changed. Refresh')
})

test('a delayed real scan cannot make an old glyph location actionable after deletion', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await api(page, 'doc.apply', {
    doc,
    label: 'Duplicate Latin Unicode',
    command: { SetUnicodes: { glyph: 'V', unicodes: [0x41] } },
  })
  const held = await holdApi(page, 'doc.find_problems', { response: true })
  try {
    const dialog = await openProblems(page)
    await held.wait()
    await api(page, 'doc.apply', {
      doc,
      label: 'Delete reported glyph',
      command: { RemoveGlyphs: { names: ['V'] } },
    })
    held.release()
    await expect(dialog).toContainText('The document changed. Refresh')
    await expect(
      dialog.getByRole('button', { name: 'Open glyph V' }),
    ).toBeDisabled()
    await dialog.getByRole('button', { name: 'Refresh', exact: true }).click()
    await expect(dialog).toContainText('No problems found.')
    await expect(
      dialog.getByRole('button', { name: 'Open glyph V' }),
    ).toHaveCount(0)
  } finally {
    await held.dispose()
  }
})
