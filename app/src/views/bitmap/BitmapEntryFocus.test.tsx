import { useEffect, useState } from 'react'
import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { Dialog } from '../../components/Dialog'
import { MenuBar } from '../../shell/MenuBar'
import {
  registerCommand,
  resetCommands,
  unregisterCommand,
} from '../../shell/commands'
import { installShortcuts } from '../../shell/shortcuts'
import { useKeymapStore } from '../../stores/keymapStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { BitmapCanvas } from './BitmapCanvas'
import { PIXEL_TOOL_KEYS, registerBitmapCommands } from './bitmapCommands'
import { useBitmapToolStore } from './bitmapToolStore'
import { blank } from './pixels'

vi.mock('../../canvas/bitmapRenderer', () => ({ renderBitmap: vi.fn() }))

function PixelSurface() {
  const tool = useBitmapToolStore((state) => state.tool)
  useEffect(() => useBitmapToolStore.getState().registerEditor(), [])
  return (
    <BitmapCanvas
      value={{
        bitmap: blank(3, 3, 1),
        advance: 4,
        vertical_advance: null,
        refs: [],
      }}
      zoom={10}
      tool={tool}
      ink={1}
      disabled={false}
      onSelect={() => undefined}
      onEdit={() => undefined}
      onCommit={() => undefined}
    />
  )
}

function EntryHost({ ready }: { ready: Promise<void> }) {
  const [visible, setVisible] = useState(false)
  useEffect(() => installShortcuts(), [])
  useEffect(() => {
    registerCommand({
      id: 'bitmap.open',
      title: 'bitmap.title',
      menu: 'bitmap',
      run: async () => {
        await ready
        setVisible(true)
      },
    })
    return () => unregisterCommand('bitmap.open')
  }, [ready])
  return (
    <>
      <MenuBar />
      <input aria-label="Pending input" />
      <button>Other action</button>
      {visible ? <PixelSurface /> : null}
    </>
  )
}

beforeEach(() => {
  resetCommands()
  registerBitmapCommands()
  useUiStore.setState(initialUiState())
  useKeymapStore.setState({ overrides: {} })
  useBitmapToolStore.setState({ tool: 'pencil', editors: 0 })
})

afterEach(() => {
  resetCommands()
  useUiStore.setState(initialUiState())
  vi.restoreAllMocks()
})

async function openFromMenu(mode: 'pointer' | 'keyboard') {
  let resolve!: () => void
  const ready = new Promise<void>((done) => {
    resolve = done
  })
  render(<EntryHost ready={ready} />)
  const user = userEvent.setup()
  const title = screen.getByRole('menuitem', { name: 'Bitmap' })
  if (mode === 'pointer') {
    await user.click(title)
    await user.click(screen.getByRole('menuitem', { name: 'Bitmap editor' }))
  } else {
    title.focus()
    await user.keyboard('{Enter}{Enter}')
  }
  expect(title).toHaveFocus()
  expect(screen.queryByLabelText('Bitmap pixel canvas')).not.toBeInTheDocument()
  const finishLoad = async () => {
    await act(async () => resolve())
  }
  return { user, title, finishLoad }
}

it.each(['pointer', 'keyboard'] as const)(
  'hands delayed bitmap entry focus to the canvas after %s menu activation',
  async (mode) => {
    const { user, finishLoad } = await openFromMenu(mode)
    await finishLoad()
    const canvas = screen.getByLabelText('Bitmap pixel canvas')
    expect(canvas).toHaveFocus()
    for (const { shortcut, tool } of PIXEL_TOOL_KEYS) {
      await user.keyboard(shortcut.toLowerCase())
      expect(useBitmapToolStore.getState().tool).toBe(tool)
      expect(canvas).toHaveFocus()
    }
    const input = screen.getByRole('textbox')
    await user.click(input)
    await user.keyboard('m')
    expect(input).toHaveValue('m')
    expect(useBitmapToolStore.getState().tool).toBe('hand')
  },
)

it.each(['input', 'button', 'open menu', 'other open menu', 'modal'] as const)(
  'does not steal focus when the user moved to %s during loading',
  async (destination) => {
    const { user, title, finishLoad } = await openFromMenu('pointer')
    let owner: HTMLElement = title
    if (destination === 'input') {
      owner = screen.getByRole('textbox')
      await user.click(owner)
    } else if (destination === 'button') {
      owner = screen.getByRole('button', { name: 'Other action' })
      await user.click(owner)
    } else if (
      destination === 'open menu' ||
      destination === 'other open menu'
    ) {
      await user.click(title)
      expect(title).toHaveAttribute('aria-expanded', 'true')
      if (destination === 'other open menu') {
        await user.hover(screen.getByRole('menuitem', { name: 'File' }))
        expect(title).toHaveAttribute('aria-expanded', 'false')
        expect(screen.getByRole('menu', { name: 'File' })).toBeVisible()
      }
    } else {
      useUiStore.getState().openModal()
    }
    await finishLoad()
    expect(screen.getByLabelText('Bitmap pixel canvas')).toBeInTheDocument()
    expect(owner).toHaveFocus()
  },
)

it('focuses an initial bitmap canvas when the document body still owns focus', () => {
  expect(document.body).toHaveFocus()
  render(<PixelSurface />)
  expect(screen.getByLabelText('Bitmap pixel canvas')).toHaveFocus()
})

it('never takes focus before a dialog registers its modal count in the same commit', () => {
  const focusCanvas = vi.spyOn(HTMLCanvasElement.prototype, 'focus')
  render(
    <>
      <PixelSurface />
      <Dialog title="Pending dialog" onCancel={() => undefined}>
        <input aria-label="Dialog input" />
      </Dialog>
    </>,
  )
  expect(focusCanvas).not.toHaveBeenCalled()
  expect(screen.getByRole('textbox', { name: 'Dialog input' })).toHaveFocus()
})
