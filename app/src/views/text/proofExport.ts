import type { PreviewGlyph } from './presentationTypes'

/** One named, shaped outline run at its requested point size. */
export interface ProofEntry {
  label: string
  glyphs: readonly PreviewGlyph[]
  unitsPerEm: number
  size: number
}

type Segment = { command: string; values: number[] }
type Glyph = PreviewGlyph & { segments: Segment[] }
type Run = ProofEntry & {
  glyphs: Glyph[]
  minX: number
  minY: number
  maxX: number
  maxY: number
}

const PAGE_WIDTH = 595.276
const PAGE_HEIGHT = 841.89
const MARGIN = 48
const WIDTH = PAGE_WIDTH - 2 * MARGIN
const MAX_OUTLINE_COORDINATE = 1e9

function number(value: number): string {
  return String(Number(value.toFixed(6)))
}

function finite(value: number): number {
  if (!Number.isFinite(value) || Math.abs(value) > MAX_OUTLINE_COORDINATE)
    throw new Error('Invalid proof geometry')
  return value
}

/** Parses the absolute Bézier commands emitted by the kernel's kurbo paths. */
function segments(path: string): Segment[] {
  const tokens: string[] = []
  const token = /[a-zA-Z]|[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?/g
  let end = 0
  for (const match of path.matchAll(token)) {
    const gap = path.slice(end, match.index)
    const comma = gap.includes(',')
    if (
      !/^\s*,?\s*$/.test(gap) ||
      (comma &&
        (!tokens.length ||
          /^[a-zA-Z]$/.test(tokens.at(-1)!) ||
          /^[a-zA-Z]$/.test(match[0])))
    )
      throw new Error('Invalid proof outline')
    tokens.push(match[0])
    end = match.index + match[0].length
  }
  if (!/^\s*$/.test(path.slice(end))) throw new Error('Invalid proof outline')
  const result: Segment[] = []
  const arities: Record<string, number> = { M: 2, L: 2, Q: 4, C: 6, Z: 0 }
  let command = ''
  let index = 0
  while (index < tokens.length) {
    if (/^[a-zA-Z]$/.test(tokens[index])) command = tokens[index++]
    const count = arities[command]
    if (count === undefined || (!result.length && command !== 'M'))
      throw new Error('Unsupported proof outline command')
    const values = tokens.slice(index, index + count).map(Number)
    if (values.length !== count) throw new Error('Incomplete proof outline')
    values.forEach(finite)
    result.push({ command, values })
    index += count
    if (command === 'M') command = 'L'
    if (command === 'Z') command = ''
  }
  return result
}

function prepare(entries: readonly ProofEntry[]): Run[] {
  return entries.map((entry) => {
    if (
      !Number.isFinite(entry.unitsPerEm) ||
      entry.unitsPerEm < 1 ||
      entry.unitsPerEm > 1e6 ||
      !Number.isFinite(entry.size) ||
      entry.size < 0.01 ||
      entry.size > 1e6
    )
      throw new Error('Invalid proof size or units per em')
    let minX = 0
    let maxX = 0
    let minY = -entry.unitsPerEm * 0.2
    let maxY = entry.unitsPerEm * 0.8
    const include = (x: number, y: number) => {
      finite(x)
      finite(y)
      minX = Math.min(minX, x)
      maxX = Math.max(maxX, x)
      minY = Math.min(minY, y)
      maxY = Math.max(maxY, y)
    }
    const glyphs = entry.glyphs.map((glyph) => {
      const parsed = segments(glyph.path)
      ;[glyph.x, glyph.y, glyph.xAdvance, glyph.yAdvance].forEach(finite)
      include(glyph.x, glyph.y)
      include(glyph.x + glyph.xAdvance, glyph.y + glyph.yAdvance)
      // Bézier control hulls conservatively contain all extrema and overhangs.
      for (const segment of parsed) {
        for (let i = 0; i < segment.values.length; i += 2)
          include(glyph.x + segment.values[i], glyph.y + segment.values[i + 1])
      }
      return { ...glyph, segments: parsed }
    })
    return {
      ...entry,
      glyphs,
      minX,
      minY,
      maxX: Math.max(maxX, minX + 1),
      maxY,
    }
  })
}

function escapeHtml(text: string): string {
  return text.replace(
    /[&<>"']/g,
    (character) =>
      ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
      })[character]!,
  )
}

/** Exports a standalone printable HTML proof; all specimen glyphs are outlines. */
export function exportProofHtml(
  title: string,
  entries: readonly ProofEntry[],
): string {
  const sections = prepare(entries)
    .map((run) => {
      const width = run.maxX - run.minX
      const height = run.maxY - run.minY
      const paths = run.glyphs
        .map(
          (glyph) =>
            `<path d="${escapeHtml(glyph.path)}" transform="translate(${number(glyph.x)} ${number(glyph.y)})"/>`,
        )
        .join('')
      return `<section><h2>${escapeHtml(run.label)}</h2><svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="${escapeHtml(run.label)}" viewBox="${number(run.minX)} ${number(-run.maxY)} ${number(width)} ${number(height)}" width="${number((width * run.size) / run.unitsPerEm)}pt" height="${number((height * run.size) / run.unitsPerEm)}pt"><g transform="scale(1 -1)">${paths}</g></svg></section>`
    })
    .join('\n')
  return `<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${escapeHtml(title)}</title>
<style>body{font:14px system-ui,sans-serif;color:#17202a;background:white;max-width:1000px;margin:40px auto;padding:0 24px}h1{font-size:24px;font-weight:600;margin-bottom:32px}section{break-inside:avoid;border-top:1px solid #d9dde2;padding:16px 0 24px}h2{font-size:12px;font-weight:500;overflow-wrap:anywhere;margin:0 0 16px;color:#525b66}svg{display:block;max-width:100%;height:auto;fill:currentColor}@page{size:A4;margin:16mm}@media print{body{margin:0;padding:0}h1{font-size:18pt}}</style></head>
<body><h1>${escapeHtml(title)}</h1>${sections}</body></html>`
}

function unicodeHex(value: string): string {
  let result = 'FEFF'
  for (let index = 0; index < value.length; index++)
    result += value
      .charCodeAt(index)
      .toString(16)
      .padStart(4, '0')
      .toUpperCase()
  return result
}

function portableLabel(value: string): string {
  return Array.from(value, (char) => {
    const code = char.codePointAt(0)!
    if (code >= 32 && code <= 126) return char
    if (/\s/.test(char)) return ' '
    return `U+${code.toString(16).toUpperCase().padStart(4, '0')}`
  }).join('')
}

function labelLines(value: string, max: number): string[] {
  const lines: string[] = []
  let line = ''
  for (const character of value) {
    const token = portableLabel(character)
    if (line.length + token.length > max) {
      lines.push(line)
      line = ''
    }
    line += token
  }
  return [...lines, line]
}

function pdfLabel(
  original: string,
  lines: string[],
  y: number,
  size: number,
): string {
  const escaped = lines.map((line) => line.replace(/[()\\]/g, '\\$&'))
  return `/Span << /ActualText <${unicodeHex(original)}> >> BDC\nBT /F1 ${size} Tf ${size + 4} TL 1 0 0 1 ${MARGIN} ${number(y)} Tm\n${escaped.map((line, index) => `${index ? 'T* ' : ''}(${line}) Tj`).join('\n')}\nET\nEMC\n`
}

function pdfPath(parts: Segment[]): string {
  let x = 0
  let y = 0
  let startX = 0
  let startY = 0
  return parts
    .map(({ command, values }) => {
      const v = values
      if (command === 'Z') {
        x = startX
        y = startY
        return 'h'
      }
      let instruction: string
      if (command === 'Q') {
        instruction =
          [
            x + ((v[0] - x) * 2) / 3,
            y + ((v[1] - y) * 2) / 3,
            v[2] + ((v[0] - v[2]) * 2) / 3,
            v[3] + ((v[1] - v[3]) * 2) / 3,
            v[2],
            v[3],
          ]
            .map(number)
            .join(' ') + ' c'
      } else instruction = v.map(number).join(' ') + ` ${command.toLowerCase()}`
      x = v[v.length - 2]
      y = v[v.length - 1]
      if (command === 'M') {
        startX = x
        startY = y
      }
      return instruction
    })
    .join('\n')
}

function pdfPages(title: string, runs: Run[]): string[] {
  const pages: string[] = []
  let content = ''
  let y = PAGE_HEIGHT - MARGIN
  const begin = () => {
    content = '0.09 0.12 0.16 rg\n'
    // A bounded header leaves useful room even for pathological document names.
    const allTitleLines = labelLines(title, 27)
    const titleLines = allTitleLines.slice(0, 3)
    if (allTitleLines.length > 3) titleLines[2] += '...'
    content += pdfLabel(title, titleLines, PAGE_HEIGHT - MARGIN - 16, 16)
    y = PAGE_HEIGHT - MARGIN - titleLines.length * 20 - 22
  }
  const finish = () => {
    content += pdfLabel(
      String(pages.length + 1),
      [String(pages.length + 1)],
      25,
      9,
    )
    pages.push(content)
  }
  begin()
  for (const run of runs) {
    const labels = labelLines(run.label, 48)
    let originalLabel = run.label
    // Very long labels receive their own pages instead of overlapping outlines.
    while (labels.length > 12) {
      const chunk = labels.splice(0, 12)
      if (y < MARGIN + chunk.length * 14 + 20) {
        finish()
        begin()
      }
      content += pdfLabel(originalLabel, chunk, y - 10, 10)
      originalLabel = ''
      y -= chunk.length * 14 + 20
    }
    // Reserve one line for a numeric scale annotation when a run needs fitting.
    const labelHeight = labels.length * 14 + 26
    const availableHeight = PAGE_HEIGHT - 2 * MARGIN - 110 - labelHeight
    const scale = Math.min(
      run.size / run.unitsPerEm,
      WIDTH / (run.maxX - run.minX),
      availableHeight / (run.maxY - run.minY),
    )
    const height = (run.maxY - run.minY) * scale
    if (y - labelHeight - height - 20 < MARGIN) {
      finish()
      begin()
    }
    if (scale < run.size / run.unitsPerEm - 1e-12)
      labels.push(`[${number(run.size)} -> ${number(scale * run.unitsPerEm)}]`)
    content += pdfLabel(originalLabel, labels, y - 10, 10)
    const baseline = y - labelHeight - run.maxY * scale
    const preciseScale = scale.toFixed(12).replace(/0+$/, '').replace(/\.$/, '')
    content += `q\n${preciseScale} 0 0 ${preciseScale} ${number(MARGIN - run.minX * scale)} ${number(baseline)} cm\n`
    for (const glyph of run.glyphs) {
      if (!glyph.segments.length) continue
      content += `q\n1 0 0 1 ${number(glyph.x)} ${number(glyph.y)} cm\n${pdfPath(glyph.segments)}\nf\nQ\n`
    }
    content += 'Q\n'
    y -= labelHeight + height + 24
  }
  finish()
  return pages
}

/**
 * Exports vector PDF pages without a font dependency. Non-ASCII label characters
 * display as U+XXXX; their original Unicode is retained in PDF ActualText.
 * Oversized runs fit the printable area, preserving their shaped proportions.
 */
export function exportProofPdf(
  title: string,
  entries: readonly ProofEntry[],
): Uint8Array {
  const pages = pdfPages(title, prepare(entries))
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    `<< /Type /Pages /Count ${pages.length} /Kids [${pages.map((_, index) => `${5 + index * 2} 0 R`).join(' ')}] >>`,
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    `<< /Title <${unicodeHex(title)}> >>`,
  ]
  pages.forEach((content, index) => {
    objects.push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${PAGE_WIDTH} ${PAGE_HEIGHT}] /Resources << /Font << /F1 3 0 R >> >> /Contents ${6 + index * 2} 0 R >>`,
    )
    objects.push(`<< /Length ${content.length} >>\nstream\n${content}endstream`)
  })
  let pdf = '%PDF-1.7\n'
  const offsets = [0]
  objects.forEach((object, index) => {
    offsets.push(pdf.length)
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`
  })
  const xref = pdf.length
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`
  pdf += offsets
    .slice(1)
    .map((offset) => `${String(offset).padStart(10, '0')} 00000 n \n`)
    .join('')
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R /Info 4 0 R >>\nstartxref\n${xref}\n%%EOF\n`
  // Every serialized byte is ASCII; all Unicode strings use UTF-16BE hex.
  return new TextEncoder().encode(pdf)
}
