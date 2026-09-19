import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { expect } from '@playwright/test'
import {
  addPrefix,
  featureState,
  openFeatures,
  seedSource,
  shape,
  test,
} from './p2bSupport'
import {
  chooseExport,
  closeExport,
  LATIN_LAYOUT,
  openProblems,
} from './p2cdSupport'

test('all new export formats reject unapplied feature drafts and problem scans preserve active layout', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await seedSource(page, doc, LATIN_LAYOUT)
  const active = await shape(page, doc, 'AVfi')
  await openFeatures(page)
  await addPrefix(
    page,
    'Unfinished source',
    'feature liga { sub f i by ; } liga;',
  )
  await expect
    .poll(async () => (await featureState(page, doc)).has_unapplied_changes)
    .toBe(true)
  const before = await featureState(page, doc)
  for (const [label, extension] of [
    ['OTF (CFF)', 'otf'],
    ['Glyphs', 'glyphs'],
    ['SFD', 'sfd'],
    ['PFA', 'pfa'],
    ['PFB', 'pfb'],
  ]) {
    const output = join(source.directory, `Blocked.${extension}`)
    const dialog = await chooseExport(page, label, output)
    const response = page.waitForResponse('**/api/session.export')
    await dialog.getByRole('button', { name: 'Export', exact: true }).click()
    expect((await (await response).json()).error.code).toBe(
      'unapplied_feature_draft',
    )
    await expect(dialog.getByRole('alert')).toBeVisible()
    expect(existsSync(output)).toBe(false)
    await closeExport(page)
  }
  const problems = await openProblems(page)
  await expect(problems).toContainText('unapplied-feature-draft')
  await expect(problems.getByRole('list', { name: 'Problems' })).toContainText(
    'feature-',
  )
  const after = await featureState(page, doc)
  expect(after.generation).toBe(before.generation)
  expect(after.compile_revision).toBe(before.compile_revision)
  expect(after.has_unapplied_changes).toBe(true)
  expect(await shape(page, doc, 'AVfi')).toEqual(active)
})
