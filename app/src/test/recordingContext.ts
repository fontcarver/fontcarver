/**
 * A canvas 2D context double that records every call and style assignment.
 *
 * The drawing modules are plain functions over a context, so recording the
 * call sequence is the natural way to test them: a quadratic contour must
 * produce `quadraticCurveTo` calls, a cubic one `bezierCurveTo`, and a colour
 * is asserted from the `strokeStyle`/`fillStyle` assignment that precedes the
 * stroke or fill it applies to.
 */

/** One recorded call or property assignment. */
export interface RecordedOp {
  name: string
  args: unknown[]
}

/** A recording context plus helpers for asserting against it. */
export interface RecordingContext {
  ctx: CanvasRenderingContext2D
  ops: RecordedOp[]
  /** Every recorded call with this name. */
  calls: (name: string) => RecordedOp[]
  /** Ordered names of every recorded call, assignments included. */
  names: () => string[]
  /** The value a style property held when a call with `name` happened. */
  styleAt: (name: string, property: string, occurrence?: number) => unknown
}

const STYLE_PROPERTIES = new Set([
  'fillStyle',
  'strokeStyle',
  'lineWidth',
  'font',
  'textAlign',
  'textBaseline',
  'globalAlpha',
])

/** Creates a recording 2D context. */
export function createRecordingContext(): RecordingContext {
  const ops: RecordedOp[] = []
  const state: Record<string, unknown> = {}
  const snapshots: Record<number, Record<string, unknown>> = {}

  const target = {
    canvas: { width: 0, height: 0 },
  } as unknown as CanvasRenderingContext2D

  const ctx = new Proxy(target as unknown as Record<string, unknown>, {
    get(_, key) {
      const name = String(key)
      if (name === 'canvas') return target.canvas
      if (name in state && !STYLE_PROPERTIES.has(name)) return state[name]
      if (STYLE_PROPERTIES.has(name)) return state[name]
      if (name === 'measureText')
        return (text: string) => ({ width: String(text).length * 6 })
      return (...args: unknown[]) => {
        snapshots[ops.length] = { ...state }
        ops.push({ name, args })
        return undefined
      }
    },
    set(_, key, value) {
      const name = String(key)
      state[name] = value
      ops.push({ name: `set:${name}`, args: [value] })
      return true
    },
  }) as unknown as CanvasRenderingContext2D

  return {
    ctx,
    ops,
    calls: (name) => ops.filter((op) => op.name === name),
    names: () => ops.map((op) => op.name),
    styleAt: (name, property, occurrence = 0) => {
      let seen = 0
      for (let index = 0; index < ops.length; index += 1) {
        if (ops[index]?.name !== name) continue
        if (seen === occurrence) return snapshots[index]?.[property]
        seen += 1
      }
      return undefined
    },
  }
}
