import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { expect } from '@playwright/test'
import type { GlyphDetail } from '../src/api/generated/GlyphDetail'
import { glyphCell, outline } from './acceptanceSupport'
import type { LayerOutline } from './acceptanceSupport'
import { openFont, seedSource, shape, test } from './p2bSupport'
import { api } from './support'
import {
  chooseExport,
  closeExport,
  finishExport,
  LATIN_LAYOUT,
  outlineGeometry,
} from './p2cdSupport'

for (const [label, extension] of [
  ['OTF (CFF)', 'otf'],
  ['Glyphs', 'glyphs'],
  ['SFD', 'sfd'],
]) {
  test(`${label} exports through the UI and reopens with Latin outlines, kerning and ligatures`, async ({
    page,
    source,
  }) => {
    const doc = source.doc.doc
    const a = await glyphCell(page, doc, 'A')
    const detail = await api<GlyphDetail>(page, 'glyphs.detail', {
      doc,
      gid: a.gid,
    })
    const bold = source.doc.masters[1]
    const boldLayer = detail.glyph.layers.find(
      (layer) => 'Master' in layer.kind && layer.kind.Master === bold.id,
    )!
    await api(page, 'doc.apply', {
      doc,
      label: 'Distinct Bold advance',
      command: { SetWidth: { glyph: 'A', layer: boldLayer.id, width: 720 } },
    })
    await seedSource(page, doc, LATIN_LAYOUT)
    const regularShape = await shape(page, doc, 'AVfi')
    const boldShape = await shape(page, doc, 'AVfi', undefined, bold.id)
    expect(regularShape[0].x_advance + regularShape[1].x_advance).toBe(1120)
    expect(boldShape[0].x_advance + boldShape[1].x_advance).toBe(1240)
    const before = extension === 'sfd' ? boldShape : regularShape
    expect(before.map((glyph) => glyph.name)).toEqual(['A', 'V', 'f_i'])
    const original = outlineGeometry(
      await api<LayerOutline>(page, 'glyphs.outline', {
        doc,
        gid: a.gid,
        layer: extension === 'sfd' ? boldLayer.id : null,
      }),
    )
    const output = join(source.directory, `Latin.${extension}`)
    const dialog = await chooseExport(page, label, output)
    if (extension === 'otf') {
      await expect(dialog).toContainText('cubic CFF outlines')
      await expect(
        dialog.getByRole('checkbox', { name: 'Keep TrueType instructions' }),
      ).toHaveCount(0)
    }
    if (extension === 'glyphs') {
      await expect(dialog.getByRole('combobox')).toHaveCount(0)
      await expect(dialog).toContainText('All masters')
    }
    if (extension === 'sfd')
      await dialog.getByRole('combobox').selectOption(bold.id)
    const report = await finishExport(page, output)
    expect(existsSync(output)).toBe(true)
    if (extension === 'otf') {
      expect(readFileSync(output).subarray(0, 4).toString('ascii')).toBe('OTTO')
      expect(report.tables.map(([tag]) => tag)).toContain('CFF ')
      expect(report.tables.map(([tag]) => tag)).not.toContain('glyf')
    }
    await closeExport(page)
    const reopened = await openFont(page, output)
    expect(reopened.glyph_count).toBe(source.doc.glyph_count)
    expect(reopened.units_per_em).toBe(source.doc.units_per_em)
    expect(reopened.masters).toHaveLength(extension === 'glyphs' ? 2 : 1)
    for (const master of reopened.masters)
      expect(
        await shape(page, reopened.doc, 'AVfi', undefined, master.id),
      ).toEqual(
        extension === 'glyphs' && master.name === bold.name
          ? boldShape
          : before,
      )
    expect(
      outlineGeometry(
        await outline(
          page,
          reopened.doc,
          (await glyphCell(page, reopened.doc, 'A')).gid,
        ),
      ),
    ).toEqual(original)
  })
}

for (const extension of ['pfa', 'pfb']) {
  test(`Type 1 ${extension.toUpperCase()} discloses loss before export and reopens static outlines`, async ({
    page,
    source,
  }) => {
    const doc = source.doc.doc
    await seedSource(page, doc, LATIN_LAYOUT)
    const original = outlineGeometry(
      await outline(page, doc, (await glyphCell(page, doc, 'A')).gid),
    )
    const output = join(source.directory, `Latin.${extension}`)
    const dialog = await chooseExport(page, extension.toUpperCase(), output)
    await expect(dialog.getByRole('note')).toContainText(
      'cannot preserve OpenType layout',
    )
    await expect(dialog.getByRole('note')).toContainText('retained hints')
    expect(existsSync(output)).toBe(false)
    await expect(dialog.getByRole('combobox')).toHaveCount(1)
    const report = await finishExport(page, output)
    expect(report.warnings.map((warning) => warning.code)).toEqual(
      expect.arrayContaining(['type1-layout-loss', 'type1-hinting-loss']),
    )
    await expect(dialog).toContainText('unhinted')
    const bytes = readFileSync(output)
    if (extension === 'pfa')
      expect(bytes.subarray(0, 2).toString('ascii')).toBe('%!')
    else expect([...bytes.subarray(0, 2)]).toEqual([0x80, 0x01])
    await closeExport(page)
    const reopened = await openFont(page, output)
    expect(reopened.masters).toHaveLength(1)
    const shaped = await shape(page, reopened.doc, 'AVfi')
    expect(shaped.map((glyph) => glyph.name)).toEqual(['A', 'V', 'f', 'i'])
    expect(shaped.map((glyph) => glyph.x_advance)).toEqual([600, 600, 600, 600])
    expect(
      outlineGeometry(
        await outline(
          page,
          reopened.doc,
          (await glyphCell(page, reopened.doc, 'A')).gid,
        ),
      ),
    ).toEqual(original)
  })
}
