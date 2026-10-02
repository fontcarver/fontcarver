import { join } from 'node:path'
import { expect, type Page } from '@playwright/test'
import type { DocSummary } from '../src/api/generated/DocSummary'
import type { LayoutConversionPreview } from '../src/api/generated/LayoutConversionPreview'
import { api } from './support'
import { exportThroughDialog, glyphCell, outline } from './acceptanceSupport'
import { enterText } from './p2aSupport'
import { withUnsupportedTable } from './p2bFixtures'
import {
  applySource,
  editor,
  exportFont,
  featureState,
  openFeatures,
  openFont,
  replaceCode,
  revisions,
  saveNative,
  seedSource,
  shape,
  test,
} from './p2bSupport'

const LIGATURE = 'feature liga { sub f i by f_i; } liga;'

async function prepare(page: Page, preferLift = true) {
  await page.getByRole('button', { name: 'Lookups', exact: true }).click()
  const panel = page.getByRole('region', {
    name: 'Original lookups',
    exact: true,
  })
  await panel
    .getByRole('checkbox', {
      name: 'Use pair, group and anchor controls where equivalent',
      exact: true,
    })
    .setChecked(preferLift)
  const response = page.waitForResponse((response) =>
    response.url().endsWith('/api/layout.prepare_conversion'),
  )
  await panel
    .getByRole('button', { name: 'Check conversion', exact: true })
    .click()
  const result = (await (await response).json())
    .result as LayoutConversionPreview
  expect(result.token, JSON.stringify(result.blockers)).not.toBeNull()
  await expect(
    panel.getByRole('button', { name: 'Use editable source', exact: true }),
  ).toBeEnabled()
  return result
}

async function convert(page: Page, doc: DocSummary) {
  const [response] = await Promise.all([
    page.waitForResponse('**/api/layout.apply_conversion'),
    page
      .getByRole('button', { name: 'Use editable source', exact: true })
      .click(),
  ])
  expect(response.request().postDataJSON().doc).toBe(doc.doc)
  expect((await response.json()).error).toBeUndefined()
  await expect
    .poll(async () => (await featureState(page, doc.doc)).authority)
    .toBe('Fea')
}

async function revert(page: Page, doc: DocSummary) {
  await page.getByRole('button', { name: 'Lookups', exact: true }).click()
  await page
    .getByRole('button', { name: 'Restore original layout…', exact: true })
    .click()
  const [response] = await Promise.all([
    page.waitForResponse('**/api/layout.revert_original'),
    page
      .getByRole('button', { name: 'Restore original layout', exact: true })
      .click(),
  ])
  expect(response.request().postDataJSON().doc).toBe(doc.doc)
  expect((await response.json()).error).toBeUndefined()
  await expect
    .poll(async () => (await featureState(page, doc.doc)).authority)
    .toBe('ImportedTables')
}

test('renders original rules as glyphs, converts source, exports and restores the native original baseline', async ({
  page,
  source,
}) => {
  await seedSource(page, source.doc.doc, LIGATURE)
  const originalPath = join(source.directory, 'Original.ttf')
  await exportFont(page, source.doc.doc, originalPath)
  const imported = await openFont(page, originalPath)
  const baseline = await shape(page, imported.doc, 'fi')
  expect(baseline.map((glyph) => glyph.name)).toEqual(['f_i'])
  const initial = await featureState(page, imported.doc)
  await openFeatures(page)
  await expect(
    page.getByRole('button', { name: 'Add prefix', exact: true }),
  ).toBeDisabled()
  await page.getByRole('button', { name: 'Lookups', exact: true }).click()
  await page
    .getByRole('combobox', { name: 'Lookup', exact: true })
    .selectOption('GSUB:0')
  const rules = page.getByRole('list', {
    name: 'Original lookup rules',
    exact: true,
  })
  await expect(
    rules.getByRole('img', { name: 'Outline of f_i', exact: true }),
  ).toBeVisible()
  await expect(rules).toContainText('f_i')
  expect((await featureState(page, imported.doc)).generation).toBe(
    initial.generation,
  )
  await page.screenshot({
    path: test.info().outputPath('original-lookups.png'),
  })
  await prepare(page, false)
  await convert(page, imported)
  expect(await shape(page, imported.doc, 'fi')).toEqual(baseline)
  await page.getByRole('button', { name: 'Source', exact: true }).click()
  await expect(editor(page)).toHaveAttribute('contenteditable', 'true')
  await replaceCode(page, 'feature liga { sub f i by A; } liga;')
  await applySource(page, imported.doc)
  const a = await glyphCell(page, imported.doc, 'A')
  const layer = await outline(page, imported.doc, a.gid)
  await api(page, 'doc.apply', {
    doc: imported.doc,
    label: 'Retained width edit',
    command: { SetWidth: { glyph: 'A', layer: layer.layer, width: 650 } },
  })
  const edited = await shape(page, imported.doc, 'fi')
  expect(edited[0]).toMatchObject({ name: 'A', x_advance: 650 })
  const native = join(source.directory, 'Converted.fcf')
  const binary = join(source.directory, 'Edited.ttf')
  await saveNative(page, native)
  await exportThroughDialog(page, binary)
  const exported = await openFont(page, binary)
  expect(await shape(page, exported.doc, 'fi')).toEqual(edited)
  expect(
    (await featureState(page, exported.doc)).original_layout_available,
  ).toBe(false)
  const reopened = await openFont(page, native)
  expect(
    (await featureState(page, reopened.doc)).original_layout_available,
  ).toBe(true)
  await openFeatures(page)
  await revert(page, reopened)
  expect(await shape(page, reopened.doc, 'fi')).toEqual(baseline)
  const retainedA = await glyphCell(page, reopened.doc, 'A')
  expect((await outline(page, reopened.doc, retainedA.gid)).width).toBe(650)
  await page.keyboard.press('Control+z')
  await expect
    .poll(async () => (await featureState(page, reopened.doc)).authority)
    .toBe('Fea')
  expect(await shape(page, reopened.doc, 'fi')).toEqual(edited)
})

test('proves a kerning lift before exposing pair controls and preserves zero exceptions through revert undo', async ({
  page,
  source,
}) => {
  await api(page, 'doc.apply', {
    doc: source.doc.doc,
    label: 'Writer kerning fixture',
    command: {
      SetHorizontalKerning: {
        master: source.doc.masters[0].id,
        left: { glyph: 'A' },
        right: { glyph: 'V' },
        value: -80,
      },
    },
  })
  const path = join(source.directory, 'WriterKerning.ttf')
  await exportFont(page, source.doc.doc, path)
  const imported = await openFont(page, path)
  await openFeatures(page)
  const proven = await prepare(page)
  expect(proven.kerning_lifted, proven.messages.join('\n')).toBe(true)
  await convert(page, imported)
  expect(
    await api(page, 'kerning.status', { doc: imported.doc }),
  ).toMatchObject({ editable: true })
  for (const [side, name, members] of [
    ['side1', 'public.kern1.A', ['A', 'a']],
    ['side2', 'public.kern2.V', ['V', 'v']],
  ])
    await api(page, 'doc.apply', {
      doc: imported.doc,
      label: 'Imported group',
      command: { CreateKerningGroup: { side, name, members } },
    })
  await api(page, 'doc.apply', {
    doc: imported.doc,
    label: 'Imported group pair',
    command: {
      SetHorizontalKerning: {
        master: imported.masters[0].id,
        left: { group: 'public.kern1.A' },
        right: { group: 'public.kern2.V' },
        value: -100,
      },
    },
  })
  await page.getByRole('tab', { name: 'WriterKerning.ttf' }).click()
  const canvas = await enterText(page)
  await page
    .getByRole('combobox', { name: 'Kerning gap', exact: true })
    .selectOption('0')
  await page.getByTestId('text-kerning-value').fill('0')
  await page.getByTestId('text-kerning-value').press('Enter')
  await expect
    .poll(async () =>
      (await shape(page, imported.doc, 'AV')).reduce(
        (sum, glyph) => sum + glyph.x_advance,
        0,
      ),
    )
    .toBe(1200)
  await page.getByTestId('text-kerning-remove').click()
  await expect
    .poll(async () =>
      (await shape(page, imported.doc, 'AV')).reduce(
        (sum, glyph) => sum + glyph.x_advance,
        0,
      ),
    )
    .toBe(1100)
  await canvas.press('Control+z')
  await expect
    .poll(async () =>
      (await shape(page, imported.doc, 'AV')).reduce(
        (sum, glyph) => sum + glyph.x_advance,
        0,
      ),
    )
    .toBe(1200)
  await openFeatures(page)
  await revert(page, imported)
  expect(
    (await shape(page, imported.doc, 'AV')).reduce(
      (sum, glyph) => sum + glyph.x_advance,
      0,
    ),
  ).toBe(1120)
  await page.keyboard.press('Control+z')
  await expect
    .poll(async () => (await featureState(page, imported.doc)).authority)
    .toBe('Fea')
  expect(
    (await shape(page, imported.doc, 'AV')).reduce(
      (sum, glyph) => sum + glyph.x_advance,
      0,
    ),
  ).toBe(1200)
})

test('offers editable source while leaving non-equivalent model kerning source-owned', async ({
  page,
  source,
}) => {
  await seedSource(
    page,
    source.doc.doc,
    'lookup First { pos A V -40; } First; lookup Second { pos A V -40; } Second; feature kern { lookup First; lookup Second; } kern;',
  )
  const path = join(source.directory, 'AdditiveKerning.ttf')
  await exportFont(page, source.doc.doc, path)
  const imported = await openFont(page, path)
  const baseline = await shape(page, imported.doc, 'AV')
  expect(baseline.reduce((sum, glyph) => sum + glyph.x_advance, 0)).toBe(1120)
  await openFeatures(page)
  const proven = await prepare(page)
  expect(proven.kerning_lifted).toBe(false)
  await convert(page, imported)
  expect(await shape(page, imported.doc, 'AV')).toEqual(baseline)
  expect(
    await api(page, 'kerning.status', { doc: imported.doc }),
  ).toMatchObject({ editable: false, reason: 'source_owned_kerning' })
  await page.getByRole('button', { name: 'Source', exact: true }).click()
  await expect(editor(page)).toHaveAttribute('contenteditable', 'true')
})

test('reports an unsupported competing table without changing the imported document', async ({
  page,
  source,
}) => {
  await seedSource(page, source.doc.doc, LIGATURE)
  const original = join(source.directory, 'Simple.ttf')
  const blockedPath = join(source.directory, 'Blocked.ttf')
  await exportFont(page, source.doc.doc, original)
  withUnsupportedTable(original, blockedPath)
  const blocked = await openFont(page, blockedPath)
  const before = await featureState(page, blocked.doc)
  await openFeatures(page)
  await page.getByRole('button', { name: 'Lookups', exact: true }).click()
  await expect(
    page.getByRole('list', { name: 'Conversion blockers', exact: true }),
  ).toContainText('JSTF')
  await expect(
    page.getByRole('button', { name: 'Check conversion', exact: true }),
  ).toBeDisabled()
  const preview = await api<LayoutConversionPreview>(
    page,
    'layout.prepare_conversion',
    revisions(before),
  )
  expect(preview.token).toBeNull()
  expect(preview.blockers.join('\n')).toContain('JSTF')
  expect(await featureState(page, blocked.doc)).toEqual(before)
  expect(await api(page, 'doc.summary', { doc: blocked.doc })).toMatchObject({
    dirty: false,
    undo_label: null,
  })
})
