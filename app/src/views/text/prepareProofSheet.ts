import type { GlyphCell } from '../../api'
import type { Transport } from '../../transport'
import { buildTextLayout, type TextRequestContext } from './textLayout'
import type { ProofEntry as LibraryEntry } from './proofLibrary'
import type { ProofEntry as SheetEntry } from './proofExport'

export interface PrepareProofOptions {
  signal?: AbortSignal
  isCurrent?: () => boolean
}

/** Resolves complete shaped runs; the strip's bounded thumbnail subset is never exported. */
export async function prepareProofSheet(
  transport: Transport,
  context: TextRequestContext,
  samples: readonly LibraryEntry[],
  sizes: readonly number[],
  options: PrepareProofOptions = {},
): Promise<SheetEntry[]> {
  if (
    !samples.length ||
    samples.length > 128 ||
    samples.length * sizes.length > 512
  )
    throw new Error('proof.export.errors.limit')
  if (
    !sizes.length ||
    sizes.length > 8 ||
    sizes.some((size) => !Number.isFinite(size) || size < 6 || size > 256)
  )
    throw new Error('proof.export.errors.size')
  const check = () => {
    options.signal?.throwIfAborted()
    if (options.isCurrent && !options.isCurrent())
      throw new Error('proof.export.errors.changed')
  }
  check()
  const master =
    context.master ??
    (await transport.call('doc.font_info', { doc: context.doc })).masters[0]?.id
  if (!master) throw new Error('proof.export.errors.unresolved')
  check()
  const cells = new Map<number, GlyphCell>()
  const entries: SheetEntry[] = []
  for (const sample of samples) {
    check()
    const request = {
      ...context,
      master,
      text: sample.text,
      script: sample.script || null,
      language: sample.language || null,
      direction: sample.direction,
    }
    let layout = await buildTextLayout(transport, request, {
      signal: options.signal,
    })
    for (let retry = 0; layout.stale && retry < 20; retry += 1) {
      if (
        layout.diagnostics.some(
          (diagnostic) => diagnostic.code === 'compile-failed',
        )
      )
        throw new Error('proof.export.errors.compile')
      await new Promise<void>((resolve) => setTimeout(resolve, 250))
      check()
      layout = await buildTextLayout(transport, request, {
        signal: options.signal,
      })
    }
    if (layout.stale) throw new Error('proof.export.errors.stale')
    if (layout.glyphs.length > 4096)
      throw new Error('proof.export.errors.limit')
    if (layout.glyphs.some((glyph) => glyph.documentGid === null))
      throw new Error('proof.export.errors.unresolved')
    const missing = [
      ...new Set(
        layout.glyphs.flatMap((glyph) =>
          glyph.documentGid === null || cells.has(glyph.documentGid)
            ? []
            : [glyph.documentGid],
        ),
      ),
    ]
    for (let offset = 0; offset < missing.length; offset += 8) {
      check()
      const page = await Promise.all(
        missing.slice(offset, offset + 8).map(async (gid) => {
          const cell = await transport.call('glyphs.cell', {
            doc: context.doc,
            gid,
            master,
          })
          return [gid, cell] as const
        }),
      )
      check()
      for (const [gid, cell] of page) cells.set(gid, cell)
    }
    const glyphs = layout.glyphs.map((glyph) => ({
      ...glyph,
      path: cells.get(glyph.documentGid!)?.path ?? '',
    }))
    for (const size of sizes)
      entries.push({
        label: sample.name,
        glyphs,
        unitsPerEm: context.unitsPerEm,
        size,
      })
  }
  check()
  return entries
}
