import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  onDocumentEvent,
  type Bitmap,
  type BitmapStrikeInfo,
  type DocId,
  type GlyphId,
} from '../../api'
import { reportError } from '../../stores/uiStore'
import { cropGlyph, invert } from './bitmapTransforms'
import { BitmapReferences } from './BitmapReferences'
import { BitmapScale } from './BitmapScale'
import { BitmapMetrics } from './BitmapMetrics'
import { BitmapCanvas } from './BitmapCanvas'
import { BitmapSession } from './bitmapSession'
import {
  blank,
  crop,
  put,
  sample,
  transformSelection,
  type PixelSelection,
  type PixelTool,
} from './pixels'
import styles from './BitmapView.module.css'
let clipboard: Bitmap | null = null
export function BitmapEditor({
  doc,
  glyph,
  strike,
  upm,
  master,
  gid,
}: {
  doc: DocId
  glyph: string
  strike: BitmapStrikeInfo
  upm: number
  master: string
  gid?: GlyphId
}) {
  const { t } = useTranslation()
  const [session] = useState(() => {
    const bitmap = blank(
      Math.min(512, strike.ppem),
      Math.min(512, Math.max(1, strike.ascent + strike.descent)),
      strike.depth,
    )
    bitmap.origin_y = -strike.descent
    return new BitmapSession(
      { doc, glyph, ppem: strike.ppem, depth: strike.depth },
      { bitmap, advance: strike.ppem, vertical_advance: null, refs: [] },
    )
  })
  const state = useSyncExternalStore(session.subscribe, session.snapshot),
    value = state.value
  const [tool, setTool] = useState<PixelTool>('pencil'),
    [zoom, setZoom] = useState(16),
    [shade, setShade] = useState(255),
    [color, setColor] = useState('#171717'),
    [selection, setSelection] = useState<PixelSelection>()
  const [outlinePath, setOutlinePath] = useState('')
  useEffect(() => {
    let active = true
    const refresh = async () => {
      if (gid === undefined) return
      const detail = await call('glyphs.detail', { doc, gid })
      const layer = detail.glyph.layers.find(
        (layer) =>
          typeof layer.kind === 'object' &&
          'Master' in layer.kind &&
          layer.kind.Master === master,
      )
      if (active)
        setOutlinePath(
          detail.paths.find((path) => path.layer === layer?.id)?.flattened ??
            '',
        )
    }
    void refresh().catch(reportError)
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) void refresh().catch(reportError)
    })
    return () => {
      active = false
      off()
    }
  }, [doc, gid, master])
  useEffect(() => {
    const dispose = session.connect()
    void session.load().catch(reportError)
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) void session.load().catch(reportError)
    })
    return () => {
      off()
      dispose()
    }
  }, [session, doc])
  const commit = () => {
    void session.flush().catch(reportError)
  }
  const ink =
    strike.depth === 32
      ? (parseInt(color.slice(1), 16) * 256 + shade) >>> 0
      : Math.round((shade * ((1 << strike.depth) - 1)) / 255)
  const disabled = state.busy || state.suspended
  const whole = value
    ? { x: 0, y: 0, width: value.bitmap.width, height: value.bitmap.height }
    : undefined
  const copy = (cut: boolean) => {
    if (!value) return
    const region = selection ?? whole!
    clipboard = crop(value.bitmap, region)
    if (cut) {
      const next = structuredClone(value)
      for (let y = region.y; y < region.y + region.height; y++)
        for (let x = region.x; x < region.x + region.width; x++)
          put(next.bitmap, x, y, 0)
      session.edit(next)
      commit()
    }
  }
  const paste = () => {
    if (!value || !clipboard) return
    if (clipboard.depth !== strike.depth) {
      reportError(new Error(t('bitmap.clipboardDepth')))
      return
    }
    const next = structuredClone(value),
      region = selection ?? { x: 0, y: 0 }
    for (let y = 0; y < clipboard.height; y++)
      for (let x = 0; x < clipboard.width; x++)
        put(next.bitmap, x + region.x, y + region.y, sample(clipboard, x, y))
    session.edit(next)
    commit()
  }
  if (!value)
    return (
      <p role="status">
        {state.error ? t('bitmap.loadFailed') : t('bitmap.loading')}
      </p>
    )
  return (
    <section
      className={styles.editor}
      aria-label={t('bitmap.editor')}
      onKeyDown={(event) => {
        if (disabled || !(event.target instanceof HTMLCanvasElement)) return
        const mod = event.ctrlKey || event.metaKey,
          key = event.key.toLowerCase()
        if (mod && ['c', 'x', 'v'].includes(key)) {
          event.preventDefault()
          event.stopPropagation()
          if (key === 'v') paste()
          else copy(key === 'x')
        }
        const directions = {
          ArrowLeft: 'left',
          ArrowRight: 'right',
          ArrowUp: 'up',
          ArrowDown: 'down',
        } as const
        const direction = directions[event.key as keyof typeof directions]
        if (direction && tool !== 'hand') {
          event.preventDefault()
          event.stopPropagation()
          session.edit(transformSelection(value, selection, direction))
          if (selection)
            setSelection({
              ...selection,
              x:
                selection.x +
                (direction === 'left' ? -1 : direction === 'right' ? 1 : 0),
              y:
                selection.y +
                (direction === 'up' ? -1 : direction === 'down' ? 1 : 0),
            })
          commit()
        }
      }}
    >
      <div className={styles.toolbar}>
        {(
          [
            'pencil',
            'erase',
            'line',
            'rectangle',
            'ellipse',
            'fill',
            'select',
            'eyedropper',
            'hand',
          ] as const
        ).map((name) => (
          <button
            key={name}
            disabled={disabled}
            aria-pressed={tool === name}
            onClick={() => setTool(name)}
          >
            {name === 'eyedropper' || name === 'hand'
              ? t(`bitmapCanvas.${name}`)
              : t(`bitmap.tool.${name}`)}
          </button>
        ))}
        <label>
          {t('bitmap.zoom')}
          <select
            value={zoom}
            onChange={(e) => setZoom(Number(e.target.value))}
          >
            {[1, 2, 4, 8, 16, 24, 32].map((n) => (
              <option key={n} value={n}>
                {n}×
              </option>
            ))}
          </select>
        </label>
        {strike.depth === 32 && (
          <input
            type="color"
            aria-label={t('bitmap.color')}
            value={color}
            onChange={(e) => setColor(e.target.value)}
          />
        )}
        <label>
          {t('bitmap.coverage')}
          <input
            type="range"
            min={0}
            max={255}
            step={strike.depth === 1 ? 255 : 1}
            value={shade}
            onChange={(e) => setShade(Number(e.target.value))}
          />
          <output>{shade}</output>
        </label>
      </div>
      {state.error !== null && (
        <div role="alert">
          {t('bitmap.writeFailed')}{' '}
          <button disabled={disabled} onClick={commit}>
            {t('bitmap.retry')}
          </button>{' '}
          <button
            disabled={disabled}
            onClick={() => {
              void session.discard().catch(reportError)
            }}
          >
            {t('bitmap.discard')}
          </button>
        </div>
      )}
      <BitmapCanvas
        value={value}
        upm={upm}
        ppem={strike.ppem}
        outlinePath={outlinePath}
        resolved={state.resolved}
        onPickInk={(sample) => {
          if (strike.depth === 32) {
            setColor(`#${(sample >>> 8).toString(16).padStart(6, '0')}`)
            setShade(sample & 255)
          } else
            setShade(Math.round((sample * 255) / ((1 << strike.depth) - 1)))
        }}
        zoom={zoom}
        tool={tool}
        ink={ink}
        disabled={disabled}
        selection={selection}
        onSelect={setSelection}
        onEdit={(next) => session.edit(next)}
        onCommit={commit}
      />
      <div className={styles.toolbar}>
        {(
          ['flipX', 'flipY', 'rotate', 'left', 'right', 'up', 'down'] as const
        ).map((kind) => (
          <button
            key={kind}
            disabled={disabled}
            onClick={() => {
              session.edit(transformSelection(value, selection, kind))
              setSelection(undefined)
              commit()
            }}
          >
            {t(`bitmap.transform.${kind}`)}
          </button>
        ))}
        <button disabled={disabled} onClick={() => copy(false)}>
          {t('bitmap.copy')}
        </button>
        <button disabled={disabled} onClick={() => copy(true)}>
          {t('bitmap.cut')}
        </button>
        <button disabled={disabled} onClick={paste}>
          {t('bitmap.paste')}
        </button>
      </div>
      <div className={styles.toolbar}>
        <button
          disabled={disabled}
          onClick={() => {
            session.edit(cropGlyph(value, selection))
            setSelection(undefined)
            commit()
          }}
        >
          {t('bitmap.crop')}
        </button>
        <button
          disabled={disabled}
          onClick={() => {
            session.edit(invert(value, selection))
            commit()
          }}
        >
          {t('bitmap.invert')}
        </button>
      </div>
      <BitmapScale
        value={value}
        disabled={disabled}
        onEdit={(next) => {
          session.edit(next)
          setSelection(undefined)
          commit()
        }}
      />
      <BitmapMetrics session={session} value={value} disabled={disabled} />
      <BitmapReferences session={session} value={value} disabled={disabled} />
      <p className={styles.hint}>
        {t('bitmap.editHint')}{' '}
        {state.busy
          ? t('bitmap.saving')
          : state.dirty
            ? t('bitmap.unsaved')
            : t('bitmap.saved')}
      </p>
    </section>
  )
}
