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
    expect(pdf).toMatch(/\(1000 pt {2}\[1000 -> \d+\.\d+\]\) Tj/)
    expect(pdf).not.toContain('(U+) Tj')
  })

  it('separates consecutive escaped characters so a CJK label stays readable', () => {
    const pdf = pdfText([{ ...entry, label: '永東國' }])
    expect(pdf).toContain('(U+6C38 U+6771 U+570B) Tj')
    expect(pdf).toContain('/ActualText <FEFF6C3867715' + '70B>')
    // A space already in the source is not doubled.
    expect(pdfText([{ ...entry, label: '中 A' }])).toContain('(U+4E2D A) Tj')
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

  it("names each row's point size in both outputs so a waterfall reads as one", () => {
    const html = exportProofHtml('Waterfall', [
      { ...entry, label: 'Pangram', size: 12 },
      { ...entry, label: 'Pangram', size: 48 },
    ])
    const doc = new DOMParser().parseFromString(html, 'text/html')
    expect(
      [...doc.querySelectorAll('section')].map((section) =>
        section.getAttribute('data-size'),
      ),
    ).toEqual(['12', '48'])
    expect(
      [...doc.querySelectorAll('.meta')].map((node) => node.textContent),
    ).toEqual(['12 pt', '48 pt'])
    // The heading stays the sample's own name, so a reader can still match rows.
    expect(doc.querySelector('h2')?.textContent).toBe('Pangram')
    const pdf = pdfText([{ ...entry, label: 'Pangram', size: 12 }])
    expect(pdf).toContain('(12 pt) Tj')
  })

  it('embeds the compiled WOFF2 and sets real text in it', () => {
    const html = exportProofHtml(
      'Specimen',
      [
        {
          ...entry,
          label: 'Pangram',
          text: 'The quick brown fox',
          direction: 'ltr',
          language: 'en',
          features: [
            ['kern', true],
            ['liga', false],
          ],
        },
      ],
      {
        woff2Base64: 'd09GMgABAAAA',
        familyName: 'Example Sans',
        styleName: 'Regular',
        unitsPerEm: 1000,
      },
    )
    const doc = new DOMParser().parseFromString(html, 'text/html')
    const style = doc.querySelector('style')!.textContent!
    expect(style).toContain(
      'src:url(data:font/woff2;base64,d09GMgABAAAA) format("woff2")',
    )
    expect(style).toContain('@font-face')
    const specimen = doc.querySelector('.specimen')!
    expect(specimen.textContent).toBe('The quick brown fox')
    expect(specimen.getAttribute('lang')).toBe('en')
    expect(specimen.getAttribute('dir')).toBe('ltr')
    expect(specimen.getAttribute('style')).toContain('font-size:48pt')
    expect(specimen.getAttribute('style')).toContain(
      "font-feature-settings:'kern' 1,'liga' 0",
    )
    // The embedded rendering replaces the outlines; nothing is loaded remotely.
    expect(doc.querySelectorAll('svg')).toHaveLength(0)
    expect(doc.querySelector('script,link,img')).toBeNull()
    expect(html).toContain('Example Sans Regular')
  })

  it('sets a vertical sample in vertical writing mode with its own direction', () => {
    const html = exportProofHtml(
      'Vertical',
      [
        { ...entry, text: '永東國', direction: 'ttb', language: 'zh-Hans' },
        { ...entry, text: 'العربية', direction: 'rtl', language: 'ar' },
      ],
      {
        woff2Base64: 'd09GMg==',
        familyName: 'Example',
        styleName: 'Regular',
        unitsPerEm: 1000,
      },
    )
    const doc = new DOMParser().parseFromString(html, 'text/html')
    const [vertical, rtl] = [...doc.querySelectorAll('.specimen')]
    expect(vertical?.classList.contains('vertical')).toBe(true)
    expect(rtl?.classList.contains('vertical')).toBe(false)
    expect(rtl?.getAttribute('dir')).toBe('rtl')
    expect(doc.querySelector('style')?.textContent).toContain(
      'writing-mode:vertical-rl',
    )
  })

  it('escapes embedded text and refuses unusable font bytes, tags and languages', () => {
    const font = {
      woff2Base64: 'd09GMg==',
      familyName: '</style><script>bad()</script>',
      styleName: 'Regular',
      unitsPerEm: 1000,
    }
    const html = exportProofHtml(
      'Specimen',
      [
        {
          ...entry,
          text: '<img onerror="bad">',
          language: 'not a tag',
          features: [
            // Four printable characters, but one of them would close the
            // CSS string the value is written into.
            ["ke'n", true],
            ['bad', true],
            ['liga', true],
          ],
        },
      ],
      font,
    )
    const doc = new DOMParser().parseFromString(html, 'text/html')
    expect(doc.querySelector('script,img')).toBeNull()
    expect(doc.querySelector('.specimen')?.textContent).toBe(
      '<img onerror="bad">',
    )
    expect(doc.querySelector('.specimen')?.hasAttribute('lang')).toBe(false)
    expect(doc.querySelector('.specimen')?.getAttribute('style')).toContain(
      "font-feature-settings:'liga' 1",
    )
    expect(doc.querySelector('.specimen')?.getAttribute('style')).not.toContain(
      'ke',
    )
    expect(() =>
      exportProofHtml('Specimen', [{ ...entry, text: 'a' }], {
        ...font,
        woff2Base64: 'not base64!',
      }),
    ).toThrow()
  })

  it('refuses an embedded-font sheet whose entry has no source text', () => {
    expect(() =>
      exportProofHtml('Specimen', [entry], {
        woff2Base64: 'd09GMg==',
        familyName: 'Example',
        styleName: 'Regular',
        unitsPerEm: 1000,
      }),
    ).toThrow('Invalid proof text')
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
