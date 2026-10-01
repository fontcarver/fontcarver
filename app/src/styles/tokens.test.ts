import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const SRC = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const TOKENS = join(SRC, 'styles', 'tokens.css')

/** Every `.css` file of the app, `tokens.css` excluded. */
function stylesheets(directory: string = SRC): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) return stylesheets(path)
    return entry.name.endsWith('.css') && path !== TOKENS ? [path] : []
  })
}

function declarations(block: string): Map<string, string> {
  return new Map(
    [...block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map(([, name, value]) => [
      name ?? '',
      (value ?? '').trim(),
    ]),
  )
}

/** The declarations of one top-level rule of `tokens.css`. */
function themeBlock(selector: string): Map<string, string> {
  const css = readFileSync(TOKENS, 'utf8')
  const start = css.indexOf(selector)
  expect(start, `${selector} is missing from tokens.css`).toBeGreaterThan(-1)
  const open = css.indexOf('{', start)
  const close = css.indexOf('}', open)
  return declarations(css.slice(open, close))
}

const LIGHT = themeBlock(':root {')
const DARK = themeBlock(":root[data-theme='dark']")

function channels(colour: string): [number, number, number] {
  const hex = colour.trim().replace('#', '')
  const full =
    hex.length === 3
      ? [...hex].map((digit) => `${digit}${digit}`).join('')
      : hex
  return [0, 2, 4].map((index) =>
    Number.parseInt(full.slice(index, index + 2), 16),
  ) as [number, number, number]
}

function luminance(colour: string): number {
  return channels(colour)
    .map((channel) => channel / 255)
    .map((value) =>
      value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
    )
    .reduce(
      (total, value, index) => total + [0.2126, 0.7152, 0.0722][index]! * value,
      0,
    )
}

/** WCAG 2.1 contrast ratio between two opaque `#rrggbb` colours. */
export function contrastRatio(a: string, b: string): number {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [
    number,
    number,
  ]
  return (high + 0.05) / (low + 0.05)
}

function value(theme: Map<string, string>, token: string): string {
  const resolved = theme.get(token)
  expect(resolved, `${token} is undefined`).toBeDefined()
  expect(
    /^#[0-9a-f]{3,8}$/i.test(resolved ?? ''),
    `${token} is ${resolved ?? 'undefined'}, which this check cannot measure`,
  ).toBe(true)
  return resolved ?? '#000000'
}

const THEMES: readonly [string, Map<string, string>][] = [
  ['light', LIGHT],
  ['dark', DARK],
]

/** Grounds any UI text can sit on. */
const GROUNDS = [
  '--color-app',
  '--color-surface',
  '--color-surface-raised',
  '--color-canvas',
] as const

/** Tokens used as text colour; the design spec requires 4.5:1 at ≤ 14 px. */
const INKS = [
  '--color-text',
  '--color-text-muted',
  '--color-text-faint',
] as const

/**
 * Canvas tokens that paint a *marker* and owe the design spec 3:1 against
 * `--cv-bg`. The recessive tokens are listed in {@link CANVAS_RECESSIVE}.
 */
const CANVAS_MARKERS = [
  '--cv-ink',
  '--cv-node-corner',
  '--cv-node-smooth',
  '--cv-node-implied',
  '--cv-handle',
  '--cv-component',
  '--cv-selection',
  '--cv-anchor',
  '--cv-warning',
  '--cv-guide-local',
  '--cv-guide-global',
  '--cv-other-master',
] as const

/**
 * Canvas tokens that are deliberately below 3:1: background reference strokes
 * and alpha fills that must never compete with the outline. The design spec
 * fixes `--cv-metric` at `#B8BEC7`/`#3F4750` itself, and names
 * `--cv-background-layer` (60 % alpha) and `--cv-zone` (30 % alpha) as washes.
 */
/** Pixel-canvas tokens that paint a marker and owe 3:1 against `--bm-paper`. */
const PIXEL_MARKERS = ['--bm-ink', '--bm-accent'] as const

/**
 * Pixel-canvas tokens that are deliberately below 3:1. The grid is a hairline
 * *between* pixels, drawn only above 6× zoom: at 3:1 it would read as ink and
 * every cell would look filled.
 */
const PIXEL_RECESSIVE = ['--bm-grid'] as const

const CANVAS_RECESSIVE = [
  '--cv-metric',
  '--cv-sidebearing',
  '--cv-background-layer',
  '--cv-zone',
  '--cv-ruler',
] as const

/**
 * Stylesheets allowed to use colours that are not design tokens, with the
 * reason. The preview strip is a *proof* surface: it carries its own
 * `color-scheme` so the proof can be dark while the interface is light, and its
 * ink and paper therefore have to come from `Canvas`/`CanvasText` and
 * `currentcolor` rather than from the interface palette.
 */
const LITERAL_COLOUR_ALLOWLIST = new Set(['views/text/PreviewStrip.module.css'])

/**
 * A literal colour. The system-colour keywords are matched case-sensitively and
 * only where they are not part of an identifier, so `--color-canvas` and a
 * `.canvas` class name are not mistaken for the `Canvas` system colour.
 */
const COLOUR_LITERAL =
  /#[0-9a-f]{3,8}\b|\brgba?\(\s*[\d.]|\bhsla?\(\s*[\d.]|(?<![\w-])(?:Canvas|CanvasText|ButtonFace|LinkText)(?![\w-])/

describe('design tokens', () => {
  it('defines the same custom properties in both themes', () => {
    // Structure, type and motion are theme-independent and live in `:root`
    // only; everything that names a colour has to exist in both themes.
    const colourish = (token: string) =>
      token.startsWith('--color-') ||
      token.startsWith('--cv-') ||
      token.startsWith('--bm-') ||
      token.startsWith('--shadow-')
    const light = [...LIGHT.keys()].filter(colourish)
    const dark = [...DARK.keys()].filter(colourish)
    expect(light.filter((token) => !dark.includes(token))).toEqual([])
    expect(dark.filter((token) => !light.includes(token))).toEqual([])
  })

  it('keeps interface text at 4.5:1 on every ground', () => {
    const failures: string[] = []
    for (const [name, theme] of THEMES)
      for (const ink of INKS)
        for (const ground of GROUNDS) {
          const ratio = contrastRatio(value(theme, ink), value(theme, ground))
          if (ratio < 4.5)
            failures.push(`${name}: ${ink} on ${ground} = ${ratio.toFixed(2)}`)
        }
    expect(failures).toEqual([])
  })

  it('keeps accent, danger and warning text at 4.5:1 on their own grounds', () => {
    /*
     * `--color-app` is the window ground *behind* the panels: status and accent
     * text always sits inside a panel (`--color-surface`, `--color-surface-raised`)
     * or a field (`--color-canvas`), and `--color-active`/`--color-sunken` are
     * pressed and inset fills that carry `--color-text`. The browser audit in
     * `docs/p5f-m-help-acceptance.md` measures the composited pairs that are
     * actually rendered, and is what catches a combination this list does not
     * predict — it is how `--color-accent` on `--color-surface-raised` was found.
     */
    const pairs: readonly [string, string][] = [
      ['--color-accent-text', '--color-accent'],
      ['--color-text', '--color-accent-soft'],
      ['--color-danger', '--color-danger-soft'],
      ['--color-warning', '--color-warning-soft'],
      ['--color-danger', '--color-surface'],
      ['--color-danger', '--color-surface-raised'],
      ['--color-danger', '--color-canvas'],
      ['--color-warning', '--color-surface'],
      ['--color-warning', '--color-surface-raised'],
      ['--color-warning', '--color-canvas'],
      ['--color-accent', '--color-surface'],
      ['--color-accent', '--color-surface-raised'],
      ['--color-accent', '--color-canvas'],
      ['--color-accent-hover', '--color-surface'],
      ['--color-accent-hover', '--color-surface-raised'],
    ]
    const failures: string[] = []
    for (const [name, theme] of THEMES)
      for (const [ink, ground] of pairs) {
        const ratio = contrastRatio(value(theme, ink), value(theme, ground))
        if (ratio < 4.5)
          failures.push(`${name}: ${ink} on ${ground} = ${ratio.toFixed(2)}`)
      }
    expect(failures).toEqual([])
  })

  it('keeps canvas markers at 3:1 against their canvas ground', () => {
    const failures: string[] = []
    for (const [name, theme] of THEMES) {
      for (const marker of CANVAS_MARKERS) {
        const ratio = contrastRatio(
          value(theme, marker),
          value(theme, '--cv-bg'),
        )
        if (ratio < 3)
          failures.push(`${name}: ${marker} on --cv-bg = ${ratio.toFixed(2)}`)
      }
      for (const marker of PIXEL_MARKERS) {
        const ratio = contrastRatio(
          value(theme, marker),
          value(theme, '--bm-paper'),
        )
        if (ratio < 3)
          failures.push(
            `${name}: ${marker} on --bm-paper = ${ratio.toFixed(2)}`,
          )
      }
    }
    expect(failures).toEqual([])
  })

  it('classifies every pixel-canvas colour token as a marker or as recessive', () => {
    const classified = new Set<string>([
      ...PIXEL_MARKERS,
      ...PIXEL_RECESSIVE,
      // The ground is not a marker on itself.
      '--bm-paper',
    ])
    const unclassified = [...LIGHT.keys()].filter(
      (token) => token.startsWith('--bm-') && !classified.has(token),
    )
    expect(unclassified).toEqual([])
  })

  it('classifies every canvas colour token as a marker or as recessive', () => {
    const classified = new Set<string>([
      ...CANVAS_MARKERS,
      ...CANVAS_RECESSIVE,
      // Grounds and inverted pills are not markers on their own ground.
      '--cv-bg',
      '--cv-metric-label',
      '--cv-hud-bg',
      '--cv-hud-ink',
      '--cv-selection-ink',
    ])
    const unclassified = [...LIGHT.keys()].filter(
      (token) => token.startsWith('--cv-') && !classified.has(token),
    )
    expect(unclassified).toEqual([])
  })

  it('gives the bitmap canvas its own palette, distinct from the interface', () => {
    for (const [name, theme] of THEMES) {
      const paper = value(theme, '--bm-paper')
      const ink = value(theme, '--bm-ink')
      value(theme, '--bm-grid')
      value(theme, '--bm-accent')
      // A pixel surface that looked like a panel would read as chrome.
      expect(paper, `${name} --bm-paper equals --color-canvas`).not.toBe(
        theme.get('--color-canvas'),
      )
      expect(ink, `${name} --bm-ink equals --color-text`).not.toBe(
        theme.get('--color-text'),
      )
    }
  })
})

describe('the canvas renderers', () => {
  /** The modules that resolve a canvas palette out of the document element. */
  const RENDERERS: readonly [string, RegExp][] = [
    ['views/glyph/glyphTheme.ts', /--cv-/],
    ['views/bitmap/BitmapCanvas.tsx', /--bm-/],
    ['views/bitmap/BitmapWaterfall.tsx', /--bm-/],
  ]

  it('reads its own canvas palette and not the interface palette', () => {
    // A canvas cannot read CSS variables, so each renderer resolves tokens off
    // the document element by name. Reading `--color-*` there is how a canvas
    // silently ends up painted in interface colours.
    const wrong: string[] = []
    for (const [name, expected] of RENDERERS) {
      const source = readFileSync(join(SRC, name), 'utf8')
      if (!expected.test(source))
        wrong.push(`${name} reads no ${expected.source} token`)
      for (const [, token] of source.matchAll(
        /getPropertyValue\(\s*'(--color-[\w-]+)'/g,
      ))
        wrong.push(`${name} resolves ${token ?? ''} for its canvas`)
      for (const [, token] of source.matchAll(/token\(\s*'(--color-[\w-]+)'/g))
        wrong.push(`${name} resolves ${token ?? ''} for its canvas`)
    }
    // The glyph canvas legitimately shares the accent and the border with the
    // interface: the marquee is the interface accent, and the grid and ruler
    // lines are the interface hairline. Everything else must be a `--cv-` token.
    expect(
      wrong.filter(
        (entry) =>
          !entry.endsWith('--color-accent for its canvas') &&
          !entry.endsWith('--color-border for its canvas'),
      ),
    ).toEqual([])
  })
})

describe('component stylesheets', () => {
  const sheets = stylesheets().map((path) => ({
    path,
    name: relative(SRC, path),
    css: readFileSync(path, 'utf8'),
  }))

  it('finds every design token it references', () => {
    const defined = new Set([...LIGHT.keys(), ...DARK.keys()])
    const missing: string[] = []
    for (const sheet of sheets)
      for (const [, token, fallback] of sheet.css.matchAll(
        /var\(\s*(--[\w-]+)\s*(,[^)]*)?\)/g,
      )) {
        if (!token || defined.has(token)) continue
        // A local custom property declared in the same stylesheet is fine.
        if (new RegExp(`${token}\\s*:`).test(sheet.css)) continue
        missing.push(`${sheet.name}: ${token}${fallback ? ' (fallback)' : ''}`)
      }
    expect(missing).toEqual([])
  })

  it('declares no colour outside tokens.css', () => {
    const offenders: string[] = []
    for (const sheet of sheets) {
      if (LITERAL_COLOUR_ALLOWLIST.has(sheet.name)) continue
      for (const line of sheet.css.split('\n')) {
        const code = line.replace(/\/\*.*?\*\//g, '')
        if (COLOUR_LITERAL.test(code))
          offenders.push(`${sheet.name}: ${code.trim()}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('keeps every rule that sets both colour and background at 4.5:1', () => {
    const failures: string[] = []
    for (const sheet of sheets) {
      if (LITERAL_COLOUR_ALLOWLIST.has(sheet.name)) continue
      for (const [, body] of sheet.css.matchAll(/\{([^{}]*)\}/g)) {
        const ink = /(?<![\w-])color:\s*var\(\s*(--[\w-]+)/.exec(body ?? '')
        const ground = /background(?:-color)?:\s*var\(\s*(--[\w-]+)/.exec(
          body ?? '',
        )
        if (!ink?.[1] || !ground?.[1]) continue
        for (const [name, theme] of THEMES) {
          const a = theme.get(ink[1])
          const b = theme.get(ground[1])
          if (!a?.startsWith('#') || !b?.startsWith('#')) continue
          const ratio = contrastRatio(a, b)
          if (ratio < 4.5)
            failures.push(
              `${sheet.name} (${name}): ${ink[1]} on ${ground[1]} = ${ratio.toFixed(2)}`,
            )
        }
      }
    }
    expect(failures).toEqual([])
  })
})
