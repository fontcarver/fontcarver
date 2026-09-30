import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { FeatureDiagnostic } from '../../api'
import { FeatureDiagnostics } from './FeatureDiagnostics'
import { featureState, featureValidation } from './featureTestFixtures'

function setup(overrides: Partial<FeatureDiagnostic> = {}) {
  const state = featureState()
  const source = state.active_source
  const code = '# setup\r\n# 字😀 invalid\rsub A by B;'
  source.prefixes[1]!.code = code
  const from = code.indexOf('invalid')
  const validation = {
    ...featureValidation(state),
    valid: false,
    diagnostics: [
      {
        severity: 'Error' as const,
        code: 'parse',
        message: 'Check this rule',
        source_id: 'p2',
        span: { start: from, end: from + 7 },
        generated: false,
        master: 'm1',
        ...overrides,
      },
    ],
  }
  const onNavigate = vi.fn()
  render(
    <FeatureDiagnostics
      validation={validation}
      source={source}
      onNavigate={onNavigate}
    />,
  )
  return { onNavigate, from }
}

describe('FeatureDiagnostics', () => {
  it('shows a source line and character column while navigating the original UTF-16 span', () => {
    const { onNavigate, from } = setup()
    const diagnostic = screen.getByRole('button', {
      name: /Check this rule.*Unicode.*Line 2, column 6/,
    })
    fireEvent.click(diagnostic)
    expect(onNavigate).toHaveBeenCalledExactlyOnceWith('p2', from, from + 7)
  })

  it('opens source-level diagnostics without inventing a line location', () => {
    const { onNavigate } = setup({ span: null })
    expect(screen.queryByText(/Line \d+, column \d+/)).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /Check this rule/ }))
    expect(onNavigate).toHaveBeenCalledExactlyOnceWith('p2', 0, 0)
  })

  it('keeps diagnostics without an available source non-navigable', () => {
    const { onNavigate } = setup({ source_id: 'missing-source' })
    const diagnostic = screen.getByRole('button', { name: /Check this rule/ })
    expect(diagnostic).toBeDisabled()
    expect(screen.queryByText(/Line \d+, column \d+/)).not.toBeInTheDocument()
    fireEvent.click(diagnostic)
    expect(onNavigate).not.toHaveBeenCalled()
  })
})
