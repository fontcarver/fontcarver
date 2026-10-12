import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, it } from 'vitest'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { CjkFilters } from './CjkFilters'
beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useDocumentStore.getState().ensure(1)
})
function Filters() {
  const value = useDocumentStore((state) => state.byDoc[1]?.filter.cjk)
  return <CjkFilters doc={1} value={value} />
}
it('retains the chosen reading language before entering a query and clears filters', async () => {
  render(<Filters />)
  const user = userEvent.setup()
  await user.selectOptions(
    screen.getByLabelText('Reading language'),
    'japanese',
  )
  await user.type(screen.getByLabelText('Reading prefix'), 'mizu')
  expect(useDocumentStore.getState().byDoc[1]?.filter.cjk).toMatchObject({
    reading: 'mizu',
    language: 'japanese',
  })
  await user.clear(screen.getByLabelText('Reading prefix'))
  expect(useDocumentStore.getState().byDoc[1]?.filter.cjk).toBeNull()
  await user.selectOptions(screen.getByLabelText('Kangxi radical'), '85')
  expect(useDocumentStore.getState().byDoc[1]?.filter.cjk?.radical).toBe(85)
})
