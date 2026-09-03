import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import { App } from './App'
import { registerBuiltinCommands } from './shell/builtinCommands'
import './i18n'
import './styles/tokens.css'
import './styles/global.css'

registerBuiltinCommands()

const root = document.getElementById('root')

if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}
