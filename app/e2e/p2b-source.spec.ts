import { copyFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { expect } from '@playwright/test'
import { api } from './support'
import { menuItem, glyphCell } from './acceptanceSupport'
import {
  addPrefix,
  applySource,
  editor,
  exportFont,
  featureState,
  holdApi,
  openFeatures,
  openFont,
  replaceCode,
  revisions,
  shape,
  test,
} from './p2bSupport'

const INVALID =
  '# 草稿 😀 — preserved exactly\nfeature liga { sub f i by ; } liga;'
const VALID =
  '# Accepted source\nfeature liga { sub f i by f_i; } liga;\nfeature kern { pos A V -80; } kern;'

test('retains invalid Unicode drafts through native save and separates validation from activation', async ({
  page,
  source,
}) => {
  const before = await featureState(page, source.doc.doc)
  await openFeatures(page)
  expect((await featureState(page, source.doc.doc)).generation).toBe(
    before.generation,
  )
  await addPrefix(page, 'Latin layout', INVALID)
  const native = join(source.directory, 'InvalidDraft.fcf')
  await editor(page).press('Control+s')
  const save = page.getByRole('dialog', { name: 'Save As' })
  await save.getByRole('textbox', { name: 'Document path' }).fill(native)
  await save.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(save).toBeHidden()
  const saved = await featureState(page, source.doc.doc)
  expect(saved.draft?.source.prefixes[0].code).toBe(INVALID)
  expect(saved.compile_revision).toBe(before.compile_revision)
  expect(saved.has_unapplied_changes).toBe(true)
  // The shell focuses a path it already has open instead of opening it twice,
  // so the saved bytes are reopened through a copy: the draft still has to come
  // back from disk rather than from the document in memory.
  const copy = join(source.directory, 'InvalidDraftCopy.fcf')
  // The dialog closes as soon as the save is accepted; wait for the bytes.
  await expect.poll(() => existsSync(native), { timeout: 20_000 }).toBe(true)
  copyFileSync(native, copy)
  const reopened = await openFont(page, copy)
  await openFeatures(page)
  expect(
    (await featureState(page, reopened.doc)).draft?.source.prefixes[0].code,
  ).toBe(INVALID)
  await expect(editor(page)).toContainText('草稿 😀')

  await page.getByRole('button', { name: 'Check source', exact: true }).click()
  const diagnostics = page.getByRole('region', {
    name: 'Diagnostics',
    exact: true,
  })
  await expect(diagnostics.getByRole('button').first()).toBeEnabled()
  await diagnostics.getByRole('button').first().click()
  await expect(editor(page)).toBeFocused()
  const rejectedPath = join(source.directory, 'Unapplied.ttf')
  const rejected = await page.request.post('/api/session.export', {
    data: { doc: reopened.doc, path: rejectedPath, options: { format: 'Ttf' } },
  })
  expect((await rejected.json()).error.code).toBe('unapplied_feature_draft')
  expect(existsSync(rejectedPath)).toBe(false)

  await replaceCode(page, VALID)
  await page.getByRole('button', { name: 'Check source', exact: true }).click()
  await expect(diagnostics).toContainText('Source compiles in 2 masters.')
  expect(
    (await shape(page, reopened.doc, 'fi')).map((glyph) => glyph.name),
  ).toEqual(['f', 'i'])
  expect((await featureState(page, reopened.doc)).has_unapplied_changes).toBe(
    true,
  )
  await applySource(page, reopened.doc)
  const active = await shape(page, reopened.doc, 'AVfi')
  expect(active.map((glyph) => glyph.name)).toEqual(['A', 'V', 'f_i'])
  expect(
    active.slice(0, 2).reduce((sum, glyph) => sum + glyph.x_advance, 0),
  ).toBe(1120)
  await editor(page).press('Control+z')
  await expect
    .poll(
      async () =>
        (await featureState(page, reopened.doc)).has_unapplied_changes,
    )
    .toBe(true)
  expect(
    (await shape(page, reopened.doc, 'fi')).map((glyph) => glyph.name),
  ).toEqual(['f', 'i'])
  await editor(page).press('Control+Shift+z')
  await expect
    .poll(
      async () =>
        (await featureState(page, reopened.doc)).has_unapplied_changes,
    )
    .toBe(false)
  for (const [format, extension] of [
    ['Ufo', 'ufo'],
    ['Ttf', 'ttf'],
    ['Woff', 'woff'],
    ['Woff2', 'woff2'],
  ]) {
    const output = join(source.directory, `Applied.${extension}`)
    await exportFont(page, reopened.doc, output, format)
    const roundtrip = await openFont(page, output)
    expect(await shape(page, roundtrip.doc, 'AVfi')).toEqual(active)
  }
})

test('generates and freezes an opt-in recipe without changing active layout during draft edits', async ({
  page,
  source,
}) => {
  await openFeatures(page)
  const recipe = page.getByRole('checkbox', {
    name: 'Small capitals (.sc → smcp)',
    exact: true,
  })
  await recipe.check()
  expect(
    (await shape(page, source.doc.doc, 'A', [['smcp', true]]))[0].name,
  ).toBe('A')
  await applySource(page, source.doc.doc)
  expect(
    (await shape(page, source.doc.doc, 'A', [['smcp', true]]))[0].name,
  ).toBe('A.sc')
  const generated = await featureState(page, source.doc.doc)
  expect(generated.active_settings.generated).toHaveLength(1)
  await page
    .getByRole('complementary', { name: 'Automatic features', exact: true })
    .getByRole('button', { name: 'Freeze into editable source', exact: true })
    .click()
  await expect(recipe).not.toBeChecked()
  await expect
    .poll(
      async () =>
        (await featureState(page, source.doc.doc)).has_unapplied_changes,
    )
    .toBe(true)
  expect(
    (await featureState(page, source.doc.doc)).active_settings.small_caps,
  ).toBe(true)
  await applySource(page, source.doc.doc)
  const frozen = await featureState(page, source.doc.doc)
  expect(frozen.active_settings.small_caps).toBe(false)
  expect(frozen.active_settings.generated).toEqual([])
  expect(
    frozen.active_source.features.some(
      (block) => block.tag === 'smcp' && !block.automatic,
    ),
  ).toBe(true)
  expect(
    (await shape(page, source.doc.doc, 'A', [['smcp', true]]))[0].name,
  ).toBe('A.sc')
})

test('rejects stale source activation and ignores delayed diagnostics after glyph deletion', async ({
  page,
  source,
}) => {
  await openFeatures(page)
  await addPrefix(page, 'Ligature', 'feature liga { sub f i by f_i; } liga;')
  const held = await holdApi(page, 'features.validate', { response: true })
  try {
    await page
      .getByRole('button', { name: 'Check source', exact: true })
      .click()
    await held.wait()
    const old = await featureState(page, source.doc.doc)
    await api(page, 'doc.apply', {
      doc: source.doc.doc,
      label: 'Delete referenced glyph',
      command: { RemoveGlyphs: { names: ['f_i'] } },
    })
    const stale = await page.request.post('/api/features.apply', {
      data: revisions(old),
    })
    expect((await stale.json()).error.code).toBe('stale_features')
    held.release()
    await expect(
      page.getByRole('button', { name: 'Check source', exact: true }),
    ).toBeEnabled()
    await expect(
      page.getByRole('region', { name: 'Diagnostics', exact: true }),
    ).not.toContainText('Source compiles in')
    await page
      .getByRole('button', { name: 'Check source', exact: true })
      .click()
    await expect(
      page
        .getByRole('region', { name: 'Diagnostics', exact: true })
        .getByRole('button')
        .first(),
    ).toBeVisible()
    await menuItem(page, 'Edit', /^Undo/)
    await expect
      .poll(async () => (await glyphCell(page, source.doc.doc, 'f_i')).name)
      .toBe('f_i')
    await page
      .getByRole('button', { name: 'Check source', exact: true })
      .click()
    await expect(
      page.getByRole('region', { name: 'Diagnostics', exact: true }),
    ).toContainText('Source compiles in 2 masters.')
  } finally {
    await held.dispose()
  }
})
