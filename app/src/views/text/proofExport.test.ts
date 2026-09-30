import { describe, expect, it } from 'vitest'
import { exportProofHtml, exportProofPdf, type ProofEntry } from './proofExport'

const entry: ProofEntry = {
  label: 'Regular 48',
  unitsPerEm: 1000,
  size: 48,
  glyphs: [
    {
      path: 'M0 0L500 0Q600 350 500 700L0 700Z',
      x: -30,
      y: 20,
      xAdvance: 600,
      yAdvance: 0,
    },
    {
      path: 'M0 0C100 100 200 100 300 0Z',
      x: 530,
      y: -40,
      xAdvance: 400,
      yAdvance: 0,
    },
  ],
}
const pdfText = (entries: readonly ProofEntry[] = [entry], title = 'Proof') =>
  new TextDecoder().decode(exportProofPdf(title, entries))

describe('standalone proof export', () => {
  it('escapes labels and title while retaining Unicode and positioned outlines', () => {
    const html = exportProofHtml('<script>中 & "</script>', [
      { ...entry, label: '<img onerror="bad"> العربية' },
    ])
    const doc = new DOMParser().parseFromString(html, 'text/html')
    expect(doc.title).toBe('<script>中 & "</script>')
    expect(doc.querySelector('h2')?.textContent).toBe(
      '<img onerror="bad"> العربية',
    )
    expect(doc.querySelector('script,img,link')).toBeNull()
    expect(doc.querySelectorAll('path')).toHaveLength(2)
    expect(doc.querySelector('path')?.getAttribute('transform')).toBe(
      'translate(-30 20)',
    )
    expect(doc.querySelector('svg')?.getAttribute('viewBox')).toBe(
      '-30 -800 960 1000',
    )
  })

  it('uses filled vector paths and exact quadratic-to-cubic conversion in PDF', () => {
    const pdf = pdfText()
    expect(pdf.startsWith('%PDF-1.7')).toBe(true)
    expect(pdf).toContain(
      '0 0 m\n500 0 l\n566.666667 233.333333 566.666667 466.666667 500 700 c',
    )
    expect(pdf).toContain('1 0 0 1 -30 20 cm')
    expect(pdf).toContain('1 0 0 1 530 -40 cm')
    expect(pdf).toContain('0.048 0 0 0.048')
    expect(pdf).not.toContain('/Subtype /Image')
  })

  it('records Unicode title and labels with visible portable label escapes', () => {
    const pdf = pdfText([{ ...entry, label: '中 (A) \\' }], '校样')
    expect(pdf).toContain('/Title <FEFF68216837>')
    expect(pdf).toContain('/ActualText <FEFF4E2D00200028004100290020005C>')
    expect(pdf).toContain('(U+4E2D \\(A\\) \\\\) Tj')
  })

  it('paginates many rows and writes valid byte offsets and stream lengths', () => {
    const pdf = pdfText(Array.from({ length: 25 }, () => entry))
    const count = Number(pdf.match(/\/Type \/Pages \/Count (\d+)/)?.[1])
    expect(count).toBeGreaterThan(1)
    const start = Number(pdf.match(/startxref\n(\d+)/)?.[1])
    expect(pdf.slice(start, start + 4)).toBe('xref')
    const offsets = [...pdf.matchAll(/^(\d{10}) 00000 n /gm)]
    offsets.forEach((match, i) =>
      expect(pdf.slice(Number(match[1]))).toMatch(
        new RegExp(`^${i + 1} 0 obj\\n`),
      ),
    )
    for (const match of pdf.matchAll(
      /\/Length (\d+) >>\nstream\n([\s\S]*?)endstream/g,
    )) {
      expect(match[2]?.length).toBe(Number(match[1]))
    }
  })

  it('retains vertical advances and negative side bearings in the exported bounds', () => {
    const html = exportProofHtml('Vertical', [
      {
        ...entry,
        glyphs: [
          {
            ...entry.glyphs[0]!,
            x: -300,
            y: -1200,
            xAdvance: 0,
            yAdvance: -1000,
          },
        ],
      },
    ])
    expect(html).toContain('viewBox="-300 -800 600 3000"')
    expect(html).toContain('translate(-300 -1200)')
  })

  it('exports an empty sheet and whitespace-only glyphs', () => {
    expect(pdfText([])).toContain('/Type /Pages /Count 1')
    expect(exportProofHtml('Empty', [])).toContain('<h1>Empty</h1>')
    expect(
      pdfText([{ ...entry, glyphs: [{ ...entry.glyphs[0]!, path: '' }] }]),
    ).not.toContain('NaN')
  })

  it('labels the actual fitted size and keeps portable Unicode escapes intact', () => {
    const pdf = pdfText([
      { ...entry, size: 1000, label: `${'W'.repeat(47)}中` },
    ])
    expect(pdf).toContain('(U+4E2D) Tj')
    expect(pdf).toMatch(/\(\[1000 -> \d+\.\d+\]\) Tj/)
    expect(pdf).not.toContain('(U+) Tj')
  })

  it('paginates exceptionally long labels without duplicating their ActualText', () => {
    const label = '中'.repeat(500)
    const pdf = pdfText([{ ...entry, label }])
    expect(
      Number(pdf.match(/\/Type \/Pages \/Count (\d+)/)?.[1]),
    ).toBeGreaterThan(1)
    expect(
      pdf.match(new RegExp(`FEFF${'4E2D'.repeat(500)}`, 'g')),
    ).toHaveLength(1)
  })

  it.each([
    { unitsPerEm: 0 },
    { unitsPerEm: Infinity },
    { size: -1 },
    { size: NaN },
    { glyphs: [{ ...entry.glyphs[0]!, x: Infinity }] },
    { glyphs: [{ ...entry.glyphs[0]!, yAdvance: NaN }] },
    ...[
      'M0 0L2',
      'M0 0L1e999 2',
      'M0 0" onload="bad',
      'L0 0',
      'M0 0A5 5 0 0 0 0 0',
      'M0 0Z2 3',
      'M,0 0',
      'M0,,0',
      'M0 0,',
      ',',
      'M0 0,L1 1',
    ].map((path) => ({ glyphs: [{ ...entry.glyphs[0]!, path }] })),
  ])('rejects invalid geometry before exporting: %j', (invalid) => {
    expect(() => exportProofHtml('Proof', [{ ...entry, ...invalid }])).toThrow()
    expect(() => exportProofPdf('Proof', [{ ...entry, ...invalid }])).toThrow()
  })
})
