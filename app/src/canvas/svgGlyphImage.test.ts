import { expect, it } from 'vitest'
import { defaultSvgGlyphBounds, serializeSvgGlyph } from './svgGlyphImage'

const NS = 'http://www.w3.org/2000/svg'
const svg = (children: string, attrs = '') =>
  `<svg xmlns="${NS}" ${attrs}>${children}</svg>`

function parsed(source: string, id = 'art', unitsPerEm = 1000) {
  const result = serializeSvgGlyph(source, id, unitsPerEm)
  expect(result).not.toBeNull()
  return new DOMParser().parseFromString(result!, 'image/svg+xml')
}

it('frames SVG y-down coordinates around the font baseline', () => {
  expect(defaultSvgGlyphBounds(1000)).toEqual({
    x: -150,
    y: -1000,
    width: 1300,
    height: 1300,
  })
  const doc = parsed(svg('<path id="art" d="M100 0 L300 -700 L500 0Z"/>'))
  expect(doc.documentElement.getAttribute('viewBox')).toBe(
    '-150 -1000 1300 1300',
  )
  expect(doc.documentElement.firstElementChild?.getAttribute('width')).toBe(
    '1000',
  )
  expect(doc.documentElement.firstElementChild?.getAttribute('height')).toBe(
    '1000',
  )
  expect(doc.documentElement.firstElementChild?.getAttribute('overflow')).toBe(
    'visible',
  )
})

it('retains selected nested ancestors and isolates unrelated glyphs in definitions', () => {
  const doc = parsed(
    svg(`
    <g id="parent" transform="translate(10 -20)" fill="blue" opacity="0.5">
      <g id="other"><path d="M0 0H100V100Z"/></g>
      <g id="nested" transform="scale(2)" style="stroke:red">
        <path id="art" d="M0 0H100V100Z"/>
      </g>
    </g>
    <path id="sibling" d="M0 0H900V900Z"/>
  `),
  )
  const target = doc.getElementById('art')!
  expect(target.parentElement?.id).toBe('nested')
  expect(target.parentElement?.getAttribute('transform')).toBe('scale(2)')
  expect(target.parentElement?.getAttribute('style')).toBe('stroke:red')
  expect(target.parentElement?.parentElement?.id).toBe('parent')
  expect(doc.getElementById('parent')?.getAttribute('fill')).toBe('blue')
  expect(doc.getElementById('parent')?.getAttribute('opacity')).toBe('0.5')
  expect(doc.getElementById('parent')?.getAttribute('transform')).toBe(
    'translate(10 -20)',
  )
  expect(doc.getElementById('other')?.parentElement?.localName).toBe('defs')
  expect(doc.getElementById('sibling')?.parentElement?.localName).toBe('defs')
  expect(target.closest('defs')).toBeNull()
})

it('retains shared paint definitions and references to unrelated outline glyphs', () => {
  const doc = parsed(
    svg(`
    <defs><linearGradient id="paint"><stop stop-color="red"/></linearGradient></defs>
    <path id="outline" d="M0 0H100V100Z"/>
    <g id="art" fill="url(#paint)"><use href="#outline"/></g>
  `),
  )
  expect(doc.getElementById('paint')?.parentElement?.localName).toBe('defs')
  expect(doc.getElementById('outline')?.parentElement?.localName).toBe('defs')
  expect(doc.getElementById('art')?.getAttribute('fill')).toBe('url(#paint)')
  expect(
    doc.getElementById('art')?.firstElementChild?.getAttribute('href'),
  ).toBe('#outline')
})

it('renders a glyph bound inside definitions with its presentation ancestors', () => {
  const doc = parsed(
    svg(`
    <defs id="definitions" fill="green">
      <g transform="translate(20 30)"><path id="art" d="M0 0H100V100Z"/></g>
      <path id="other" d="M0 0H900V900Z"/>
    </defs>
  `),
  )
  const target = doc.getElementById('art')!
  expect(target.closest('defs')).toBeNull()
  expect(target.parentElement?.getAttribute('transform')).toBe(
    'translate(20 30)',
  )
  expect(target.parentElement?.parentElement?.getAttribute('fill')).toBe(
    'green',
  )
  expect(doc.getElementById('other')?.parentElement?.localName).toBe('defs')
})

it('keeps an explicitly selected SVG root, its viewport and complete artwork', () => {
  const doc = parsed(
    svg(
      '<path id="one"/><path id="two"/>',
      'id="art" width="900" height="800" viewBox="0 -800 900 800"',
    ),
  )
  const sourceRoot = doc.getElementById('art')!
  expect(sourceRoot.getAttribute('width')).toBe('900')
  expect(sourceRoot.getAttribute('height')).toBe('800')
  expect(sourceRoot.getAttribute('viewBox')).toBe('0 -800 900 800')
  expect(sourceRoot.children).toHaveLength(2)
})

it('accepts an explicit frame without changing source geometry', () => {
  const serialized = serializeSvgGlyph(svg('<path id="art"/>'), 'art', 1000, {
    x: -100,
    y: -1200,
    width: 1800,
    height: 1500,
  })!
  const doc = new DOMParser().parseFromString(serialized, 'image/svg+xml')
  expect(doc.documentElement.getAttribute('viewBox')).toBe(
    '-100 -1200 1800 1500',
  )
})

it('rejects malformed XML, non-SVG roots, doctypes, duplicate IDs and missing targets', () => {
  for (const source of [
    '<svg',
    '<svg><path id="art"/></svg>',
    `<g xmlns="${NS}"><path id="art"/></g>`,
    `<!DOCTYPE svg [<!ENTITY value "test">]>${svg('<path id="art"/>')}`,
    svg('<path id="art"/><path id="art"/>'),
    svg('<path id="other"/>'),
  ])
    expect(serializeSvgGlyph(source, 'art', 1000)).toBeNull()
})

it('rejects invalid units and nonfinite or empty bounds', () => {
  for (const units of [0, -1, NaN, Infinity])
    expect(serializeSvgGlyph(svg('<path id="art"/>'), 'art', units)).toBeNull()
  for (const bounds of [
    { x: NaN, y: 0, width: 10, height: 10 },
    { x: 0, y: Infinity, width: 10, height: 10 },
    { x: 0, y: 0, width: 0, height: 10 },
    { x: 0, y: 0, width: 10, height: -1 },
  ])
    expect(
      serializeSvgGlyph(svg('<path id="art"/>'), 'art', 1000, bounds),
    ).toBeNull()
})

it('enforces the UTF-8 byte limit before parsing multibyte sources', () => {
  const source = svg(`<desc>${'字'.repeat(700_000)}</desc><path id="art"/>`)
  expect(source.length).toBeLessThan(2 * 1024 * 1024)
  expect(serializeSvgGlyph(source, 'art', 1000)).toBeNull()
})

it('does not insert source nodes into the application document', () => {
  parsed(svg('<path id="art" fill="var(--color0, red)"/>'))
  expect(document.getElementById('art')).toBeNull()
})

it('supplies CPAL variables and foreground while preserving source fallbacks', () => {
  const serialized = serializeSvgGlyph(
    svg('<path id="art" fill="var(--color1, red)" stroke="currentColor"/>'),
    'art',
    1000,
    undefined,
    ['#ff0000', '#008000'],
    '#ffffff',
  )!
  const doc = new DOMParser().parseFromString(serialized, 'image/svg+xml')
  const wrapper = doc.documentElement as unknown as SVGSVGElement
  expect(wrapper.style.getPropertyValue('--color0')).toBe('#ff0000')
  expect(wrapper.style.getPropertyValue('--color1')).toBe('#008000')
  expect(wrapper.style.color).toBe('rgb(255, 255, 255)')
  expect(doc.getElementById('art')?.getAttribute('fill')).toBe(
    'var(--color1, red)',
  )
  expect(doc.getElementById('art')?.getAttribute('stroke')).toBe('currentColor')
})

it('bounds the decoded image dimensions without shrinking the font-space frame', () => {
  const doc = parsed(svg('<path id="art"/>'), 'art', 16384)
  expect(doc.documentElement.getAttribute('width')).toBe('512')
  expect(doc.documentElement.getAttribute('height')).toBe('512')
  expect(doc.documentElement.getAttribute('viewBox')).toBe(
    '-2457.6 -16384 21299.2 21299.2',
  )
})
