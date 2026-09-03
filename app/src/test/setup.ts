import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

import '../i18n'

afterEach(cleanup)

/**
 * jsdom implements neither `Path2D` nor a canvas 2D context. Stubbing both
 * keeps the glyph-grid component tests exercising the real cell renderer
 * instead of silently skipping it, and keeps "Not implemented" noise out of
 * the test output.
 */
if (typeof globalThis.Path2D === 'undefined') {
  globalThis.Path2D = class {
    constructor(readonly data?: string) {}
  } as unknown as typeof Path2D
}

function stubContext(): CanvasRenderingContext2D {
  const properties: Record<string, unknown> = {}
  return new Proxy(properties, {
    get(target, key) {
      if (key === 'measureText')
        return (text: string) => ({ width: String(text).length * 6 })
      if (key in target) return target[key as string]
      return () => undefined
    },
    set(target, key, value) {
      target[key as string] = value
      return true
    },
  }) as unknown as CanvasRenderingContext2D
}

if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = function getContext() {
    return stubContext()
  } as unknown as HTMLCanvasElement['getContext']
}
