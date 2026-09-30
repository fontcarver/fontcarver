import {
  call,
  type ColorState,
  type DocId,
  type MasterId,
  type SvgState,
} from '../../api'
import { colorLayers, type ColorGlyphResource } from '../../canvas/colorGlyph'
import {
  defaultSvgGlyphBounds,
  serializeSvgGlyph,
} from '../../canvas/svgGlyphImage'

interface RenderSource {
  color: ColorState | null
  svg: SvgState | null
}

/** Read-only state: imported data is decoded and validated by the kernel. */
export async function colorRenderSource(doc: DocId): Promise<RenderSource> {
  const [color, svg] = await Promise.allSettled([
    call('color.state', { doc }),
    call('svg.state', { doc }),
  ])
  return {
    color: color.status === 'fulfilled' ? color.value : null,
    svg: svg.status === 'fulfilled' ? svg.value : null,
  }
}

let familySequence = 0

/** A per-document source loader with a 32 MiB live COLRv1 font-byte budget. */
export function colorGlyphLoader(
  doc: DocId,
  master: MasterId | null,
  invalidate: () => void,
) {
  const source = colorRenderSource(doc)
  let fontBytes = 0
  const load = async (glyph: string): Promise<ColorGlyphResource | null> => {
    const state = await source
    const color = state.color?.color ?? state.color?.preview_color
    const svg = state.svg?.svg ?? state.svg?.preview_svg
    const upem = state.color?.units_per_em ?? state.svg?.units_per_em ?? 1000
    if (
      color?.graph?.glyphs[glyph] !== undefined &&
      typeof FontFace !== 'undefined' &&
      globalThis.CSS?.supports?.('font-tech(color-COLRv1)')
    ) {
      try {
        const result = await call('color.graph_preview', {
          doc,
          glyph,
          color,
          palette_index: 0,
          ...(master ? { master } : {}),
        })
        if (
          result.bytes.length > 8 * 1024 * 1024 ||
          fontBytes + result.bytes.length > 32 * 1024 * 1024
        )
          throw new Error('Color preview font-byte budget')
        const family = `fc-color-glyph-${++familySequence}`
        const face = new FontFace(family, new Uint8Array(result.bytes))
        fontBytes += result.bytes.length
        try {
          await face.load()
        } catch (error) {
          fontBytes -= result.bytes.length
          throw error
        }
        document.fonts.add(face)
        let disposed = false
        return {
          paint(context, foreground) {
            context.save()
            context.scale(1, -1)
            context.font = `${upem}px "${family}"`
            context.textAlign = 'left'
            context.textBaseline = 'alphabetic'
            context.fillStyle = foreground
            context.fillText('\uE000', 0, 0)
            context.restore()
          },
          dispose() {
            if (disposed) return
            disposed = true
            document.fonts.delete(face)
            fontBytes -= result.bytes.length
          },
        }
      } catch {
        /* Continue with a supported fallback. */
      }
    }
    const layers = color?.glyphs[glyph]
    if (layers?.length) {
      try {
        const preview = await call('color.preview', {
          doc,
          glyph,
          ...(master ? { master } : {}),
          palette_index: 0,
          layers,
          palette: color?.palettes[0]?.colors ?? null,
        })
        return colorLayers(preview.layers)
      } catch {
        /* Continue with SVG or the existing outline. */
      }
    }
    const record = svg?.documents.find(
      (document) => document.glyphs[glyph] !== undefined,
    )
    if (!record) return null
    const palette =
      color?.palettes[0]?.colors.map(
        (c) => `rgba(${c.red},${c.green},${c.blue},${c.alpha / 255})`,
      ) ?? []
    return svgResource(
      record.source,
      record.glyphs[glyph]!,
      state.svg?.units_per_em ?? upem,
      palette,
      invalidate,
    )
  }
  return Object.assign(load, {
    names: source.then((state) => {
      const color = state.color?.color ?? state.color?.preview_color
      const svg = state.svg?.svg ?? state.svg?.preview_svg
      return new Set([
        ...Object.keys(color?.glyphs ?? {}),
        ...Object.keys(color?.graph?.glyphs ?? {}),
        ...(svg?.documents.flatMap((record) => Object.keys(record.glyphs)) ??
          []),
      ])
    }),
  })
}

async function svgResource(
  source: string,
  id: string,
  upem: number,
  palette: readonly string[],
  invalidate: () => void,
): Promise<ColorGlyphResource | null> {
  const bounds = defaultSvgGlyphBounds(upem)
  const images = new Map<
    string,
    { image: HTMLImageElement; url: string; ready: boolean }
  >()
  let disposed = false
  const load = async (foreground: string) => {
    const serialized = serializeSvgGlyph(
      source,
      id,
      upem,
      bounds,
      palette,
      foreground,
    )
    if (!serialized || disposed) return false
    const url = URL.createObjectURL(
      new Blob([serialized], { type: 'image/svg+xml' }),
    )
    const image = new Image()
    const entry = { image, url, ready: false }
    images.set(foreground, entry)
    const ready = await new Promise<boolean>((resolve) => {
      image.onload = () => resolve(true)
      image.onerror = () => resolve(false)
      image.src = url
    })
    if (!disposed && images.get(foreground) === entry) {
      entry.ready = ready
      if (ready) invalidate()
    }
    return ready
  }
  const first = '#202020'
  if (!(await load(first))) {
    for (const entry of images.values()) URL.revokeObjectURL(entry.url)
    return null
  }
  return {
    paint(context, foreground) {
      let entry = images.get(foreground)
      if (!entry) {
        if (images.size >= 4) {
          const oldest = images.keys().next().value
          if (oldest !== undefined) {
            const removed = images.get(oldest)!
            URL.revokeObjectURL(removed.url)
            images.delete(oldest)
          }
        }
        void load(foreground)
        entry = [...images.values()].find((item) => item.ready)
      }
      if (!entry?.ready) return
      context.save()
      context.scale(1, -1)
      context.drawImage(
        entry.image,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
      )
      context.restore()
    },
    dispose() {
      disposed = true
      for (const entry of images.values()) {
        entry.image.onload = null
        entry.image.onerror = null
        entry.image.src = ''
        URL.revokeObjectURL(entry.url)
      }
      images.clear()
    },
  }
}
