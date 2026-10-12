import { describe, expect, it } from 'vitest'

import {
  canStepFromSurface,
  proofStepDirection,
  type ProofStepEvent,
} from './proofStepping'

function press(overrides: Partial<ProofStepEvent> = {}): ProofStepEvent {
  return {
    key: 'ArrowDown',
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    ...overrides,
  }
}

describe('proof library stepping keys', () => {
  it('steps backwards on Up and forwards on Down', () => {
    expect(proofStepDirection(press({ key: 'ArrowUp' }))).toBe(-1)
    expect(proofStepDirection(press({ key: 'ArrowDown' }))).toBe(1)
  })

  it('leaves every modified arrow and every other key alone', () => {
    for (const modifier of [
      'altKey',
      'ctrlKey',
      'metaKey',
      'shiftKey',
    ] as const)
      expect(proofStepDirection(press({ [modifier]: true }))).toBeNull()
    expect(proofStepDirection(press({ key: 'ArrowLeft' }))).toBeNull()
    expect(proofStepDirection(press({ key: 'PageDown' }))).toBeNull()
    expect(proofStepDirection(press({ key: 'a' }))).toBeNull()
  })

  it('leaves a composing or already handled press alone', () => {
    expect(proofStepDirection(press({ isComposing: true }))).toBeNull()
    expect(proofStepDirection(press({ defaultPrevented: true }))).toBeNull()
  })

  it('claims a canvas press only with nothing selected and no field focused', () => {
    expect(
      canStepFromSurface({ hasSelection: false, editableTarget: false }),
    ).toBe(true)
    expect(
      canStepFromSurface({ hasSelection: true, editableTarget: false }),
    ).toBe(false)
    expect(
      canStepFromSurface({ hasSelection: false, editableTarget: true }),
    ).toBe(false)
  })

  it('leaves the arrow keys to a focusable control inside the view', () => {
    // The preview strip's resize handle is a focusable separator that moves on
    // Up and Down itself, and so are the sliders and tab strips of the panels
    // the text view hosts: a surface-wide stepper must not take their keys.
    expect(
      canStepFromSurface({
        hasSelection: false,
        editableTarget: false,
        interactiveTarget: true,
      }),
    ).toBe(false)
    // The canvas is focusable but is not an interactive control, which is why
    // `isInteractiveTarget` excludes it; stepping there still works.
    expect(
      canStepFromSurface({
        hasSelection: false,
        editableTarget: false,
        interactiveTarget: false,
      }),
    ).toBe(true)
  })
})
