import { describe, expect, it } from 'vitest'

import { nudgeStep } from './gestures/nudge'
import { controlModifier } from './gestures/drag'
import { modifiersOf } from './modifiers'

const none = { shiftKey: false, altKey: false, metaKey: false, ctrlKey: false }

describe('outline modifier grammar', () => {
  it('uses Command for ×100 and Control for snap suspension on macOS', () => {
    const command = modifiersOf({ ...none, metaKey: true }, true)
    const control = modifiersOf({ ...none, ctrlKey: true }, true)
    expect(nudgeStep(command)).toBe(100)
    expect(controlModifier(command)).toBe(false)
    expect(nudgeStep(control)).toBe(1)
    expect(controlModifier(control)).toBe(true)
  })

  it('uses Control as the command modifier on Windows and Linux', () => {
    const control = modifiersOf({ ...none, ctrlKey: true }, false)
    expect(nudgeStep(control)).toBe(100)
    expect(controlModifier(control)).toBe(true)
    expect(modifiersOf({ ...none, metaKey: true }, false).mod).toBe(false)
  })

  it('retains Shift ×10 and independent alternate behavior', () => {
    const modifiers = modifiersOf(
      { ...none, shiftKey: true, altKey: true },
      true,
    )
    expect(nudgeStep(modifiers)).toBe(10)
    expect(modifiers.alt).toBe(true)
  })
})
