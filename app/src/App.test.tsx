import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { App } from './App'
import './i18n'

describe('App', () => {
  it('renders the application name', () => {
    render(<App />)

    expect(screen.getByRole('heading', { name: 'FontWorkshop' })).toBeInTheDocument()
  })

  it('cleans up the previous component after each test', () => {
    render(<App />)

    expect(screen.getAllByText('FontWorkshop')).toHaveLength(1)
  })
})
