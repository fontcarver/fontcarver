import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { REPO_ROOT } from './support'

const plist = (body: string) =>
  `<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>${body}</dict></plist>`

/** Temporary source data derives only from this project's licensed Minimal UFO. */
export function featureFixture(twoMasters = true) {
  const directory = mkdtempSync(join(tmpdir(), 'fc-p2b-browser-'))
  // `[name, code point]`; a glyph without a code point is reachable only
  // through layout. The P5 recipe rows need figures, ordinals, fraction parts,
  // a mark and a dotless form on top of the original P2b inventory.
  const glyphs: [string, string?][] = [
    ['.notdef'],
    ['A', '0041'],
    ['V', '0056'],
    ['a', '0061'],
    ['v', '0076'],
    ['f', '0066'],
    ['i', '0069'],
    ['f_i'],
    ['A.sc'],
    ['A.ss01'],
    ['A.case'],
    ['A.vert'],
    ['a.sc'],
    ['o', '006f'],
    ['one', '0031'],
    ['two', '0032'],
    ['one.lf'],
    ['one.osf'],
    ['one.tf'],
    ['one.pnum'],
    ['one.sups'],
    ['one.sinf'],
    ['one.numr'],
    ['one.dnom'],
    ['two.numr'],
    ['two.dnom'],
    ['fraction', '2044'],
    ['slash', '002f'],
    ['period', '002e'],
    ['period.ordn'],
    ['ordfeminine', '00aa'],
    ['ordmasculine', '00ba'],
    ['idotless', '0131'],
    ['acutecomb', '0301'],
  ]
  const names = glyphs.map(([name]) => name)
  const makeMaster = (style: string) => {
    const path = join(directory, `${style}.ufo`)
    cpSync(resolve(REPO_ROOT, 'fixtures/ufo/Minimal.ufo'), path, {
      recursive: true,
    })
    for (const file of ['groups.plist', 'kerning.plist', 'lib.plist'])
      writeFileSync(join(path, file), plist(''))
    writeFileSync(join(path, 'features.fea'), '')
    const info = readFileSync(join(path, 'fontinfo.plist'), 'utf8')
      .replace('<string>Minimal</string>', '<string>P2b Browser</string>')
      .replace('<string>Regular</string>', `<string>${style}</string>`)
    writeFileSync(join(path, 'fontinfo.plist'), info)
    const outline = readFileSync(join(path, 'glyphs/A_.glif'), 'utf8')
    for (const [index, [name, codepoint]] of glyphs.entries()) {
      const unicode = codepoint ? `<unicode hex="${codepoint}"/>` : ''
      writeFileSync(
        join(path, 'glyphs', `p2b-${index}.glif`),
        outline
          .replace('name="A"', `name="${name}"`)
          .replace('<unicode hex="0041"/>', unicode),
      )
    }
    writeFileSync(
      join(path, 'glyphs/contents.plist'),
      plist(
        names
          .map(
            (name, index) =>
              `<key>${name}</key><string>p2b-${index}.glif</string>`,
          )
          .join(''),
      ),
    )
    return path
  }
  const regular = makeMaster('Regular')
  let input = regular
  if (twoMasters) {
    makeMaster('Bold')
    input = join(directory, 'P2b.designspace')
    writeFileSync(
      input,
      `<?xml version="1.0" encoding="UTF-8"?>
<designspace format="5.0"><axes><axis tag="wght" name="Weight" minimum="400" maximum="700" default="400"/></axes><sources>
<source filename="Regular.ufo" name="regular" familyname="P2b Browser" stylename="Regular"><location><dimension name="Weight" xvalue="400"/></location></source>
<source filename="Bold.ufo" name="bold" familyname="P2b Browser" stylename="Bold"><location><dimension name="Weight" xvalue="700"/></location></source>
</sources></designspace>`,
    )
  }
  return {
    directory,
    input,
    regular,
    cleanup: () => rmSync(directory, { recursive: true, force: true }),
  }
}

/** Add a competing layout table to a generated TTF for fail-closed UI coverage. */
export function withUnsupportedTable(input: string, output: string) {
  const original = readFileSync(input)
  const tables: Array<{ tag: string; data: Buffer }> = []
  for (let index = 0; index < original.readUInt16BE(4); index += 1) {
    const start = 12 + index * 16
    const offset = original.readUInt32BE(start + 8)
    tables.push({
      tag: original.toString('ascii', start, start + 4),
      data: Buffer.from(
        original.subarray(offset, offset + original.readUInt32BE(start + 12)),
      ),
    })
  }
  tables.push({ tag: 'JSTF', data: Buffer.from([0, 1, 0, 0]) })
  tables.sort((a, b) => a.tag.localeCompare(b.tag))
  const head = tables.find((table) => table.tag === 'head')
  if (!head) throw new Error('generated TTF is missing head')
  head.data.writeUInt32BE(0, 8)
  const align = (value: number) => (value + 3) & ~3
  const bytes = Buffer.alloc(
    12 +
      tables.length * 16 +
      tables.reduce((total, table) => total + align(table.data.length), 0),
  )
  original.copy(bytes, 0, 0, 4)
  bytes.writeUInt16BE(tables.length, 4)
  const power = Math.floor(Math.log2(tables.length))
  bytes.writeUInt16BE(2 ** power * 16, 6)
  bytes.writeUInt16BE(power, 8)
  bytes.writeUInt16BE(tables.length * 16 - 2 ** power * 16, 10)
  const checksum = (data: Buffer) => {
    const padded = Buffer.alloc(align(data.length))
    data.copy(padded)
    let sum = 0
    for (let at = 0; at < padded.length; at += 4)
      sum = (sum + padded.readUInt32BE(at)) >>> 0
    return sum
  }
  let offset = 12 + tables.length * 16
  let headOffset = 0
  for (const [index, table] of tables.entries()) {
    const start = 12 + index * 16
    bytes.write(table.tag, start, 4, 'ascii')
    bytes.writeUInt32BE(checksum(table.data), start + 4)
    bytes.writeUInt32BE(offset, start + 8)
    bytes.writeUInt32BE(table.data.length, start + 12)
    table.data.copy(bytes, offset)
    if (table.tag === 'head') headOffset = offset
    offset += align(table.data.length)
  }
  bytes.writeUInt32BE((0xb1b0afba - checksum(bytes)) >>> 0, headOffset + 8)
  writeFileSync(output, bytes)
}
