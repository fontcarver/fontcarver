const NS = 'http://www.w3.org/2000/svg'
const MAX_SOURCE_BYTES = 2 * 1024 * 1024
const MAX_IMAGE_DIMENSION = 512
const DEFINITION_ELEMENTS = new Set([
  'defs',
  'linearGradient',
  'radialGradient',
  'clipPath',
  'mask',
  'pattern',
  'title',
  'desc',
])

/** SVG y-down font coordinates, with the font baseline at y = 0. */
export interface SvgGlyphBounds {
  x: number
  y: number
  width: number
  height: number
}

/** The baseline-aligned frame shared with the SVG authoring preview. */
export function defaultSvgGlyphBounds(unitsPerEm: number): SvgGlyphBounds {
  return {
    x: -unitsPerEm * 0.15,
    y: -unitsPerEm,
    width: unitsPerEm * 1.3,
    height: unitsPerEm * 1.3,
  }
}

/**
 * Serialize a backend-validated SVG glyph for loading exclusively in image mode.
 * Never insert the returned source into the application DOM. Browser image mode
 * isolates scripts and external subresources; this helper is not an HTML sanitizer.
 * The caller owns the Blob URL and image lifecycle and draws into `bounds`.
 */
export function serializeSvgGlyph(
  source: string,
  id: string,
  unitsPerEm: number,
  bounds: SvgGlyphBounds = defaultSvgGlyphBounds(unitsPerEm),
  palette: readonly string[] = [],
  foreground = 'black',
): string | null {
  if (
    !id ||
    !Number.isFinite(unitsPerEm) ||
    unitsPerEm <= 0 ||
    !Object.values(bounds).every(Number.isFinite) ||
    bounds.width <= 0 ||
    bounds.height <= 0 ||
    source.length > MAX_SOURCE_BYTES ||
    new TextEncoder().encode(source).byteLength > MAX_SOURCE_BYTES ||
    /<!DOCTYPE\b/i.test(source)
  )
    return null

  const doc = new DOMParser().parseFromString(source, 'image/svg+xml')
  const root = doc.documentElement
  const target = doc.getElementById(id)
  if (
    doc.querySelector('parsererror') ||
    root.namespaceURI !== NS ||
    root.localName !== 'svg' ||
    !target ||
    target.namespaceURI !== NS
  )
    return null

  const ids = new Set<string>()
  for (const element of doc.getElementsByTagName('*')) {
    if (!element.id) continue
    if (ids.has(element.id)) return null
    ids.add(element.id)
  }

  // Keep the selected path through the tree intact: a bare <use href="#id">
  // loses the transforms and presentation inherited from the target's parents.
  // Other branches stay addressable for local paint and outline references.
  let branch: Element = target
  while (branch !== root) {
    let parent: Element | null = branch.parentElement
    if (!parent) return null
    for (const sibling of Array.from(parent.children)) {
      if (sibling === branch || DEFINITION_ELEMENTS.has(sibling.localName))
        continue
      const definitions = doc.createElementNS(NS, 'defs')
      parent.insertBefore(definitions, sibling)
      definitions.appendChild(sibling)
    }
    if (parent.localName === 'defs') {
      // A binding may refer to artwork stored in defs. Promote only its selected
      // ancestor path; siblings remain definitions rather than painted glyphs.
      const group = doc.createElementNS(NS, 'g')
      for (const attribute of parent.attributes)
        group.setAttributeNS(
          attribute.namespaceURI,
          attribute.name,
          attribute.value,
        )
      while (parent.firstChild) group.appendChild(parent.firstChild)
      parent.replaceWith(group)
      parent = group
    }
    branch = parent
  }

  if (!root.hasAttribute('width'))
    root.setAttribute('width', String(unitsPerEm))
  if (!root.hasAttribute('height'))
    root.setAttribute('height', String(unitsPerEm))
  root.setAttribute('overflow', 'visible')

  const wrapper = doc.createElementNS(NS, 'svg')
  wrapper.setAttribute(
    'viewBox',
    `${bounds.x} ${bounds.y} ${bounds.width} ${bounds.height}`,
  )
  // Decode at a bounded size even when a font uses a very large units-per-em.
  const imageScale = Math.min(
    1,
    MAX_IMAGE_DIMENSION / Math.max(bounds.width, bounds.height),
  )
  wrapper.setAttribute('width', String(bounds.width * imageScale))
  wrapper.setAttribute('height', String(bounds.height * imageScale))
  wrapper.style.color = foreground
  palette.forEach((color, index) =>
    wrapper.style.setProperty(`--color${index}`, color),
  )
  wrapper.appendChild(root)
  return new XMLSerializer().serializeToString(wrapper)
}
