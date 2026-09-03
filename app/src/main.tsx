import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import { App } from './App'
import './i18n'
import './styles/index.css'
import './styles/tokens.css'

const root = document.getElementById('root')

if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}
