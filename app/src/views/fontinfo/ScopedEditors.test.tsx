import { useState } from 'react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it } from 'vitest'
import { makeInstance, makeMasterMetrics } from '../../test/fontFixtures'
import { InstanceEditor } from './InstanceEditor'
import { MasterMetricsEditor } from './MasterMetricsEditor'

it('shows automatic metric values and enables an override without changing the effective value', async () => {
  const user = userEvent.setup()
  function Editor() {
    const [metrics, setMetrics] = useState(
      makeMasterMetrics({ ascender: 800, descender: -200 }),
    )
    return (
      <MasterMetricsEditor
        metrics={metrics}
        unitsPerEm={2048}
        onChange={setMetrics}
      />
    )
  }
  render(<Editor />)
  await user.click(screen.getByText('Master metrics'))
  const advance = screen.getByRole('textbox', { name: 'Vertical advance' })
  expect(advance).toBeDisabled()
  expect(advance).toHaveValue('2048')
  await user.click(
    screen.getByRole('checkbox', { name: 'Override Vertical advance' }),
  )
  expect(advance).toBeEnabled()
  expect(advance).toHaveValue('2048')
  await user.clear(advance)
  await user.type(advance, '0{Enter}')
  expect(advance).toHaveValue('0')
  await user.click(
    screen.getByRole('checkbox', { name: 'Override Vertical advance' }),
  )
  expect(advance).toHaveValue('2048')
  expect(advance).toBeDisabled()
})

it('shows the effective inherited family and resets an export override', async () => {
  const user = userEvent.setup()
  function Editor() {
    const [instance, setInstance] = useState(
      makeInstance({ name: 'Book', style_name: '', family_name: null }),
    )
    return (
      <InstanceEditor
        instance={instance}
        familyName="Family default"
        axes={[]}
        onChange={setInstance}
        onRemove={() => {}}
      />
    )
  }
  render(<Editor />)
  expect(
    screen.getByText('Inherited from font: Family default'),
  ).toBeInTheDocument()
  expect(screen.getByText('From instance name: Book')).toBeInTheDocument()
  const family = screen.getByRole('textbox', {
    name: 'Family override (optional)',
  })
  await user.type(family, 'Export Family')
  expect(screen.getByText('Export override: Export Family')).toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: 'Use font family' }))
  expect(family).toHaveValue('')
  expect(
    screen.getByText('Inherited from font: Family default'),
  ).toBeInTheDocument()
})
