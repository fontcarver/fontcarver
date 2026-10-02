import { describe, expect, it } from 'vitest'

import {
  metricShortcut,
  type MetricShortcutEvent,
  type MetricSide,
} from './textMetrics'

describe('metricShortcut', () => {
  const event = (
    overrides: Partial<MetricShortcutEvent>,
  ): MetricShortcutEvent => ({
    key: 'ArrowLeft',
    altKey: false,
    shiftKey: false,
    ctrlKey: false,
    metaKey: false,
    ...overrides,
  })

  it('maps platform Mod+arrows to the stated sides and signs', () => {
    expect(metricShortcut(event({ ctrlKey: true }), false)).toEqual({
      side: 'lsb',
      delta: -1,
    })
    expect(
      metricShortcut(event({ key: 'ArrowRight', ctrlKey: true }), false),
    ).toEqual({ side: 'rsb', delta: 1 })
    expect(metricShortcut(event({ metaKey: true }), true)).toEqual({
      side: 'lsb',
      delta: -1,
    })
    expect(metricShortcut(event({ ctrlKey: true }), true)).toBeNull()
  })

  it('uses Shift for ten and macOS Meta+Ctrl for one hundred', () => {
    expect(
      metricShortcut(
        event({ key: 'ArrowRight', ctrlKey: true, shiftKey: true }),
        false,
      ),
    ).toEqual({ side: 'rsb', delta: 10 })
    expect(
      metricShortcut(event({ metaKey: true, ctrlKey: true }), true),
    ).toEqual({ side: 'lsb', delta: -100 })
    // Linux Ctrl is already Mod, so Ctrl alone remains the one-unit step.
    expect(metricShortcut(event({ ctrlKey: true }), false)?.delta).toBe(-1)
  })

  it('leaves Alt, unmodified arrows, and other keys unclaimed', () => {
    expect(
      metricShortcut(event({ ctrlKey: true, altKey: true }), false),
    ).toBeNull()
    expect(metricShortcut(event({}), false)).toBeNull()
    expect(
      metricShortcut(event({ key: 'ArrowUp', ctrlKey: true }), false),
    ).toBeNull()
  })

  it.each<MetricSide>(['lsb', 'rsb', 'both'])(
    'changes selected %s in both directions with brackets',
    (side) => {
      expect(
        metricShortcut(event({ key: '[' }), false, { side, step: 2.5 }),
      ).toEqual({ side, delta: -2.5 })
      expect(
        metricShortcut(event({ key: ']' }), false, { side, step: 2.5 }),
      ).toEqual({ side, delta: 2.5 })
    },
  )

  it.each([false, true])(
    'supports ×1 and ×10 and leaves modified brackets to navigation on macOS=%s',
    (isMac) => {
      const options = { side: 'both' as const, step: 2.5 }
      const mod = isMac ? { metaKey: true } : { ctrlKey: true }
      expect(metricShortcut(event({ key: ']' }), isMac, options)?.delta).toBe(
        2.5,
      )
      expect(
        metricShortcut(event({ key: '}', shiftKey: true }), isMac, options)
          ?.delta,
      ).toBe(25)
      // `Mod+[` and `Mod+]` step to the previous and next glyph, so a spacing
      // step here moved the glyph the tab was leaving by 100 units.
      expect(
        metricShortcut(event({ key: ']', ...mod }), isMac, options),
      ).toBeNull()
      expect(
        metricShortcut(
          event({ key: '{', shiftKey: true, ...mod }),
          isMac,
          options,
        ),
      ).toBeNull()
    },
  )

  it('uses physical bracket codes with translated key values', () => {
    expect(
      metricShortcut(
        event({ key: 'Å', code: 'BracketLeft', shiftKey: true }),
        false,
        { side: 'rsb', step: 3 },
      ),
    ).toEqual({ side: 'rsb', delta: -30 })
  })

  it.each([false, true])(
    'uses Alt for ×100 and Alt+Shift for exactly one on macOS=%s',
    (isMac) => {
      for (const side of ['lsb', 'rsb', 'both'] as const) {
        for (const [key, direction] of [
          ['[', -1],
          [']', 1],
        ] as const) {
          const options = { side, step: 2.5 }
          expect(
            metricShortcut(event({ key, altKey: true }), isMac, options),
          ).toEqual({ side, delta: direction * 250 })
          expect(
            metricShortcut(
              event({ key, altKey: true, shiftKey: true }),
              isMac,
              options,
            ),
          ).toEqual({ side, delta: direction })
        }
      }
    },
  )

  it('recognizes physical Alt brackets without claiming Control or Meta chords', () => {
    const options = { side: 'both' as const, step: 3 }
    expect(
      metricShortcut(
        event({ key: '“', code: 'BracketLeft', altKey: true }),
        true,
        options,
      ),
    ).toEqual({ side: 'both', delta: -300 })
    for (const isMac of [false, true]) {
      for (const modifier of [{ ctrlKey: true }, { metaKey: true }]) {
        for (const altKey of [false, true]) {
          expect(
            metricShortcut(
              event({ key: ']', altKey, ...modifier }),
              isMac,
              options,
            ),
          ).toBeNull()
        }
      }
    }
  })

  it.each([undefined, 0, -2, Number.NaN, Infinity])(
    'falls back to one for an unavailable or invalid step %s',
    (step) => {
      expect(
        metricShortcut(event({ key: ']' }), false, { side: 'lsb', step }),
      ).toEqual({ side: 'lsb', delta: 1 })
    },
  )

  it('preserves legacy arrows and reserves Mod+Alt bracket combinations', () => {
    const options = { side: 'both' as const, step: 4 }
    expect(metricShortcut(event({ key: '[' }), false)).toBeNull()
    expect(metricShortcut(event({ ctrlKey: true }), false, options)).toEqual({
      side: 'lsb',
      delta: -1,
    })
    expect(
      metricShortcut(
        event({ key: ']', altKey: true, ctrlKey: true }),
        false,
        options,
      ),
    ).toBeNull()
    expect(metricShortcut(event({}), false, options)).toBeNull()
  })
})
