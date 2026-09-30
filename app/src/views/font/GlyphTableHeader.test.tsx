import { useState } from 'react'
import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { GlyphTableHeader } from './GlyphTableHeader'
import {
  DEFAULT_GLYPH_TABLE_COLUMNS,
  glyphTableTemplate,
  type GlyphTableColumn,
} from './glyphTableColumns'

function renderHeader(sort: 'Order' | 'Name' | 'Unicode' | 'Width' = 'Order') {
  const onSort = vi.fn()
  const onMove = vi.fn()
  render(
    <GlyphTableHeader
      columns={DEFAULT_GLYPH_TABLE_COLUMNS}
      sort={sort}
      onSort={onSort}
      onMove={onMove}
    />,
  )
  return { onSort, onMove }
}

describe('GlyphTableHeader', () => {
  it('sorts only name, Unicode and width with ascending semantics', async () => {
    const user = userEvent.setup()
    const { onSort } = renderHeader('Unicode')

    expect(screen.getAllByRole('columnheader')).toHaveLength(6)
    for (const [label, sort] of [
      ['Name', 'Name'],
      ['Unicode', 'Unicode'],
      ['Width', 'Width'],
    ]) {
      const button = screen.getByRole('button', { name: label })
      await user.click(button)
      expect(onSort).toHaveBeenLastCalledWith(sort)
      const column = button.closest('[role="columnheader"]')
      if (sort === 'Unicode') {
        expect(column).toHaveAttribute('aria-sort', 'ascending')
      } else {
        expect(column).not.toHaveAttribute('aria-sort')
      }
    }
    for (const label of ['Category', 'Script', 'Status']) {
      expect(
        screen.queryByRole('button', { name: label }),
      ).not.toBeInTheDocument()
    }
  })

  it('moves columns in either direction through keyboard reachable controls', async () => {
    const user = userEvent.setup()
    function Header() {
      const [columns, setColumns] = useState([...DEFAULT_GLYPH_TABLE_COLUMNS])
      function move(column: GlyphTableColumn, before: GlyphTableColumn) {
        setColumns((current) => {
          const next = current.filter((value) => value !== column)
          next.splice(next.indexOf(before), 0, column)
          return next
        })
      }
      return (
        <GlyphTableHeader
          columns={columns}
          sort="Order"
          onSort={vi.fn()}
          onMove={move}
        />
      )
    }
    render(<Header />)

    const moveName = screen.getByRole('button', {
      name: 'Move Name column right',
    })
    moveName.focus()
    await user.keyboard('{Enter}')
    expect(
      within(screen.getAllByRole('columnheader')[0]).getByText('Unicode'),
    ).toBeInTheDocument()
    expect(moveName).toHaveFocus()

    const moveStatus = screen.getByRole('button', {
      name: 'Move Status column left',
    })
    moveStatus.focus()
    await user.keyboard(' ')
    expect(
      within(screen.getAllByRole('columnheader')[4]).getByText('Status'),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Move Unicode column left' }),
    ).toBeDisabled()
    expect(
      screen.getByRole('button', { name: 'Move Script column right' }),
    ).toBeDisabled()
  })

  it('drops any column before another column', () => {
    const { onMove } = renderHeader()
    const headers = screen.getAllByRole('columnheader')
    const dataTransfer = {
      setData: vi.fn(),
      effectAllowed: 'none',
      dropEffect: 'none',
    }

    fireEvent.dragStart(headers[5], { dataTransfer })
    fireEvent.dragOver(headers[0], { dataTransfer })
    fireEvent.drop(headers[0], { dataTransfer })

    expect(onMove).toHaveBeenCalledExactlyOnceWith('status', 'name')
    expect(dataTransfer.effectAllowed).toBe('move')
    expect(dataTransfer.dropEffect).toBe('move')
    fireEvent.drop(headers[1], { dataTransfer })
    expect(onMove).toHaveBeenCalledTimes(1)
  })

  it('ignores unrelated drops and dropping a column onto itself', () => {
    const { onMove } = renderHeader()
    const header = screen.getAllByRole('columnheader')[0]
    const dataTransfer = { setData: vi.fn(), effectAllowed: 'none' }

    fireEvent.drop(header, { dataTransfer })
    fireEvent.dragStart(header, { dataTransfer })
    fireEvent.drop(header, { dataTransfer })
    expect(onMove).not.toHaveBeenCalled()
  })

  it('keeps header interactions out of parent glyph selection and navigation', async () => {
    const user = userEvent.setup()
    const parentEvent = vi.fn()
    render(
      <div
        onClick={parentEvent}
        onDoubleClick={parentEvent}
        onPointerDown={parentEvent}
        onKeyDown={parentEvent}
        onKeyUp={parentEvent}
      >
        <GlyphTableHeader
          columns={DEFAULT_GLYPH_TABLE_COLUMNS}
          sort="Order"
          onSort={vi.fn()}
          onMove={vi.fn()}
        />
      </div>,
    )
    const name = screen.getByRole('button', { name: 'Name' })
    await user.dblClick(name)
    await user.keyboard('{ArrowRight}{Enter} ')
    expect(parentEvent).not.toHaveBeenCalled()
  })
})

describe('glyphTableTemplate', () => {
  it('keeps column widths attached to their reordered columns', () => {
    expect(
      glyphTableTemplate([
        'width',
        'name',
        'status',
        'script',
        'category',
        'unicode',
      ]),
    ).toBe(
      '100px minmax(180px, 2fr) minmax(140px, 1.5fr) 90px 120px minmax(140px, 1.3fr)',
    )
  })
})
