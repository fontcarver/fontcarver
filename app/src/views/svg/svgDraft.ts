import type { SvgTable } from '../../api'

export interface SvgDraft {
  documents: { source: string; bindings: string }[]
}
export function toDraft(table: SvgTable | null): SvgDraft | null {
  return (
    table && {
      documents: table.documents.map(({ source, glyphs }) => ({
        source,
        bindings: Object.entries(glyphs)
          .map(([name, id]) => `${name} = ${id}`)
          .join('\n'),
      })),
    }
  )
}
/** Keep unparsed binding text in the draft until a complete transaction validates. */
export function toTable(draft: SvgDraft | null): SvgTable | null {
  return (
    draft && {
      documents: draft.documents.map(({ source, bindings }) => {
        const glyphs: Record<string, string> = Object.create(null) as Record<
          string,
          string
        >
        for (const line of bindings.split('\n').filter((line) => line.trim())) {
          const match = /^\s*([^\s=]+)\s*=\s*([\w.:-]+)\s*$/.exec(line)
          if (!match || Object.hasOwn(glyphs, match[1]))
            throw new Error('svg.invalidBindings')
          glyphs[match[1]] = match[2]
        }
        if (!Object.keys(glyphs).length) throw new Error('svg.invalidBindings')
        return { source, glyphs }
      }),
    }
  )
}

export function newDocument() {
  return {
    bindings: '',
    source:
      '<svg xmlns="http://www.w3.org/2000/svg">\n  <path id="art" fill="#2563eb" d="M100 0 L300 -700 L500 0 Z"/>\n</svg>',
  }
}
