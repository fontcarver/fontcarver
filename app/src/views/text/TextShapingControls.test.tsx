import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import {
  TextShapingControls,
  type TextShapingControlsProps,
} from './TextShapingControls'
import type { PositionedGlyph } from './textLayout'

function props(
  overrides: Partial<TextShapingControlsProps> = {},
): TextShapingControlsProps {
  return {
    features: [],
    script: null,
    language: null,
    glyphs: [],
    onChange: vi.fn(),
    ...overrides,
  }
}

function openControls() {
  fireEvent.click(screen.getByText('Shaping controls'))
}

describe('TextShapingControls', () => {
  it('keeps features at shaper defaults until an explicit override is chosen', async () => {
    const user = userEvent.setup()
    const callbacks = props({
      features: [
        ['liga', true],
        ['ss01', false],
      ],
    })
    const { rerender } = render(<TextShapingControls {...callbacks} />)
    openControls()

    expect(screen.getByRole('combobox', { name: 'kern feature' })).toHaveValue(
      'default',
    )
    expect(screen.getByRole('combobox', { name: 'ss01 feature' })).toHaveValue(
      'off',
    )
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'kern feature' }),
      'off',
    )
    expect(callbacks.onChange).toHaveBeenLastCalledWith({
      features: [
        ['liga', true],
        ['ss01', false],
        ['kern', false],
      ],
    })

    rerender(
      <TextShapingControls
        {...callbacks}
        features={[
          ['liga', true],
          ['ss01', false],
          ['kern', false],
        ]}
      />,
    )
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'kern feature' }),
      'default',
    )
    expect(callbacks.onChange).toHaveBeenLastCalledWith({
      features: [
        ['liga', true],
        ['ss01', false],
      ],
    })
    for (const tag of ['liga', 'calt', 'mark', 'mkmk', 'vert', 'vkrn']) {
      expect(
        screen.getByRole('combobox', { name: `${tag} feature` }),
      ).toBeInTheDocument()
    }
  })

  it('validates custom tags and adds them without forcing an override', async () => {
    const callbacks = props()
    render(<TextShapingControls {...callbacks} />)
    openControls()
    const custom = screen.getByRole('textbox', { name: 'Custom feature tag' })
    fireEvent.change(custom, { target: { value: 'ab' } })
    await userEvent.click(screen.getByRole('button', { name: 'Add feature' }))
    expect(custom).toHaveAttribute('aria-invalid', 'true')
    expect(callbacks.onChange).not.toHaveBeenCalled()

    fireEvent.change(custom, { target: { value: 'ss02' } })
    fireEvent.keyDown(custom, { key: 'Enter' })
    const selector = screen.getByRole('combobox', { name: 'ss02 feature' })
    expect(selector).toHaveValue('default')
    expect(callbacks.onChange).not.toHaveBeenCalled()
    await userEvent.selectOptions(selector, 'on')
    expect(callbacks.onChange).toHaveBeenCalledWith({
      features: [['ss02', true]],
    })
  })

  it('commits valid script and language overrides and clears to automatic detection', async () => {
    const user = userEvent.setup()
    const callbacks = props()
    const { rerender } = render(<TextShapingControls {...callbacks} />)
    openControls()
    const script = screen.getByRole('textbox', { name: 'Script override' })
    await user.click(script)
    fireEvent.change(script, { target: { value: 'arab' } })
    fireEvent.keyDown(script, { key: 'Enter' })
    expect(callbacks.onChange).toHaveBeenLastCalledWith({ script: 'Arab' })
    const language = screen.getByRole('textbox', { name: 'Language override' })
    await user.click(language)
    fireEvent.change(language, { target: { value: 'zh-Hant' } })
    fireEvent.blur(language)
    expect(callbacks.onChange).toHaveBeenLastCalledWith({ language: 'zh-hant' })

    rerender(
      <TextShapingControls {...callbacks} script="Arab" language="zh-hant" />,
    )
    await user.click(script)
    fireEvent.change(script, { target: { value: ' ' } })
    fireEvent.blur(script)
    expect(callbacks.onChange).toHaveBeenLastCalledWith({ script: null })
    fireEvent.change(language, { target: { value: '' } })
    fireEvent.keyDown(language, { key: 'Enter' })
    expect(callbacks.onChange).toHaveBeenLastCalledWith({ language: null })
  })

  it('rejects malformed overrides and cancels edits without a blur commit', async () => {
    const user = userEvent.setup()
    const callbacks = props({ script: 'Latn', language: 'en' })
    render(<TextShapingControls {...callbacks} />)
    openControls()
    const script = screen.getByRole('textbox', { name: 'Script override' })
    fireEvent.change(script, { target: { value: 'Latin' } })
    fireEvent.blur(script)
    expect(script).toHaveAttribute('aria-invalid', 'true')
    const language = screen.getByRole('textbox', { name: 'Language override' })
    fireEvent.change(language, { target: { value: 'en_漢字' } })
    fireEvent.blur(language)
    expect(language).toHaveAttribute('aria-invalid', 'true')
    expect(callbacks.onChange).not.toHaveBeenCalled()

    await user.click(script)
    await user.clear(script)
    await user.type(script, 'Arab')
    await user.keyboard('{Escape}')
    expect(script).toHaveValue('Latn')
    expect(script).not.toHaveAttribute('aria-invalid')
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('rejects non-ASCII script and language codes and malformed custom tags', () => {
    const callbacks = props()
    render(<TextShapingControls {...callbacks} />)
    openControls()
    const script = screen.getByRole('textbox', { name: 'Script override' })
    fireEvent.change(script, { target: { value: 'LaKn' } })
    fireEvent.blur(script)
    expect(script).toHaveAttribute('aria-invalid', 'true')

    const language = screen.getByRole('textbox', { name: 'Language override' })
    for (const value of ['en--US', 'Ko', '123', 'a'.repeat(256)]) {
      fireEvent.change(language, { target: { value } })
      fireEvent.blur(language)
      expect(language).toHaveAttribute('aria-invalid', 'true')
    }
    const custom = screen.getByRole('textbox', { name: 'Custom feature tag' })
    fireEvent.change(custom, { target: { value: 'ligé' } })
    fireEvent.keyDown(custom, { key: 'Enter' })
    expect(custom).toHaveAttribute('aria-invalid', 'true')
    expect(callbacks.onChange).not.toHaveBeenCalled()
  })

  it('preserves visual glyph order and repeated UTF-8 source clusters in the inspector', () => {
    const glyph: PositionedGlyph = {
      compiledGid: 3,
      documentGid: 3,
      name: 'alef',
      editable: true,
      x: 0,
      y: 0,
      xAdvance: 550,
      yAdvance: -1000,
      sourceByte: 4,
      sourceKind: 'literal',
    }
    render(
      <TextShapingControls
        {...props({
          glyphs: [
            glyph,
            { ...glyph, name: 'mark', xAdvance: 0, yAdvance: 0 },
            { ...glyph, name: null, sourceByte: 0, xAdvance: 600, yAdvance: 0 },
          ],
        })}
      />,
    )
    openControls()
    fireEvent.click(screen.getByText('Glyph clusters (3)'))
    const rows = within(screen.getByRole('table')).getAllByRole('row').slice(1)
    expect(
      rows.map((row) =>
        within(row)
          .getAllByRole('cell')
          .map((cell) => cell.textContent),
      ),
    ).toEqual([
      ['4', 'alef', '550', '-1000'],
      ['4', 'mark', '0', '0'],
      ['0', 'Unnamed glyph', '600', '0'],
    ])
  })
})
