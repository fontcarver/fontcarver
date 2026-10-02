import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  type BitmapGlyph,
  type BitmapStrikeInfo,
  type DocId,
} from '../../api'
import { decodeSheetPng, encodeSheetPng } from '../../canvas/bitmapSheetImage'
import { withSettledDocument } from '../../editor/documentPersistence'
import { saveArtifact } from '../../shell/saveArtifact'
import {
  describeSheetLayout,
  exportSheetPixels,
  importSheetPixels,
  parseSheetNames,
  sheetDimensions,
  SheetError,
  type SheetLayout,
} from './bitmapSheet'
import styles from './BitmapSheet.module.css'

/** Explicit name-to-cell PNG interchange with one undoable import operation. */
export function BitmapSheet({
  doc,
  strike,
}: {
  doc: DocId
  strike: BitmapStrikeInfo
}) {
  const { t } = useTranslation()
  const [namesText, setNamesText] = useState('')
  const [cellWidth, setCellWidth] = useState(strike.ppem)
  const [cellHeight, setCellHeight] = useState(strike.ppem)
  const [columns, setColumns] = useState(8)
  const [file, setFile] = useState<File>()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [status, setStatus] = useState('')
  const [download, setDownload] = useState<string>()
  const [prepared, setPrepared] = useState<Blob>()
  const [description, setDescription] = useState('')
  useEffect(
    () => () => {
      if (download) URL.revokeObjectURL(download)
    },
    [download],
  )

  const run = (importing: boolean) => {
    if (busy) return
    setBusy(true)
    setError('')
    setStatus('')
    void withSettledDocument(doc, async () => {
      const image = importing && file ? await decodeSheetPng(file) : undefined
      if (importing && !image) throw new SheetError('png')
      // An exported sheet carries its own layout, so an import prefers it over
      // whatever the form currently holds and shows what it used.
      if (image?.layout) {
        setNamesText(image.layout.names.join(' '))
        setCellWidth(image.layout.cellWidth)
        setCellHeight(image.layout.cellHeight)
        setColumns(image.layout.columns)
      }
      const names = image?.layout
        ? image.layout.names
        : parseSheetNames(namesText)
      const options = image?.layout ?? { cellWidth, cellHeight, columns }
      sheetDimensions(names.length, options)
      const current = await call('bitmap.state', { doc })
      const latest = current.strikes.find(
        (s) => s.ppem === strike.ppem && s.depth === strike.depth,
      )
      if (!latest) throw new SheetError('changed')
      const glyphs: Record<string, BitmapGlyph> = Object.create(null)
      // Bound concurrent decoding and retain only images that fit their cells.
      for (let start = 0; start < names.length; start += 4) {
        const group = names.slice(start, start + 4)
        const images = await Promise.all(
          group.map((glyph) =>
            call('bitmap.glyph', {
              doc,
              ppem: strike.ppem,
              depth: strike.depth,
              glyph,
            }),
          ),
        )
        images.forEach((item, i) => {
          if (item.generation !== current.generation)
            throw new SheetError('changed')
          const value = item.resolved
          if (!value && !importing) throw new SheetError('missing')
          if (value) {
            if (
              value.bitmap.width > options.cellWidth ||
              value.bitmap.height > options.cellHeight
            )
              throw new SheetError('clipped')
            glyphs[group[i]] = value
          }
        })
      }
      if (image) {
        const values = importSheetPixels(
          image,
          names,
          options,
          strike.depth,
          latest.descent,
          glyphs,
        )
        await call('bitmap.apply', {
          doc,
          expected_generation: current.generation,
          label: t('bitmapSheet.undo'),
          command: {
            SetGlyphs: {
              ppem: strike.ppem,
              depth: strike.depth,
              glyphs: values,
            },
          },
        })
        setStatus(t('bitmapSheet.imported', { count: names.length }))
        setDescription(describeSheetLayout({ ...options, names }))
      } else {
        const layout: SheetLayout = { ...options, names }
        const blob = await encodeSheetPng(
          exportSheetPixels(
            names.map((name) => glyphs[name]),
            options,
          ),
          layout,
        )
        if (
          (await call('bitmap.state', { doc })).generation !==
          current.generation
        )
          throw new SheetError('changed')
        setDownload(URL.createObjectURL(blob))
        setPrepared(blob)
        setDescription(describeSheetLayout(layout))
        setStatus(t('bitmapSheet.exported', { count: names.length }))
      }
    })
      .catch((reason: unknown) => {
        setError(
          reason instanceof SheetError
            ? t(`bitmapSheet.errors.${reason.code}`)
            : t('bitmapSheet.errors.failed'),
        )
      })
      .finally(() => setBusy(false))
  }

  const save = async () => {
    if (busy || !prepared) return
    setBusy(true)
    setError('')
    try {
      const saved = await saveArtifact(
        `bitmap-${strike.ppem}px-${strike.depth}bpp.png`,
        new Uint8Array(await prepared.arrayBuffer()),
        'image/png',
        t('bitmapSheet.title'),
      )
      if (saved) setStatus(t('bitmapSheet.saved'))
    } catch {
      setError(t('bitmapSheet.errors.save'))
    } finally {
      setBusy(false)
    }
  }

  let dimensions = ''
  try {
    const size = sheetDimensions(parseSheetNames(namesText).length, {
      cellWidth,
      cellHeight,
      columns,
    })
    dimensions = t('bitmapSheet.dimensions', size)
  } catch {
    /* The action reports an invalid draft without interrupting typing. */
  }

  return (
    <details className={styles.sheet}>
      <summary>{t('bitmapSheet.title')}</summary>
      <p>{t('bitmapSheet.hint')}</p>
      <p>{t('bitmapSheet.coverage')}</p>
      <fieldset disabled={busy}>
        <label className={styles.mapping}>
          {t('bitmapSheet.names')}
          <textarea
            value={namesText}
            rows={3}
            onChange={(event) => setNamesText(event.target.value)}
          />
        </label>
        <div className={styles.options}>
          <label>
            {t('bitmapSheet.cellWidth')}
            <input
              type="number"
              min={1}
              max={4096}
              value={cellWidth}
              onChange={(event) => setCellWidth(Number(event.target.value))}
            />
          </label>
          <label>
            {t('bitmapSheet.cellHeight')}
            <input
              type="number"
              min={1}
              max={4096}
              value={cellHeight}
              onChange={(event) => setCellHeight(Number(event.target.value))}
            />
          </label>
          <label>
            {t('bitmapSheet.columns')}
            <input
              type="number"
              min={1}
              max={256}
              value={columns}
              onChange={(event) => setColumns(Number(event.target.value))}
            />
          </label>
          <span>{dimensions}</span>
        </div>
        <div className={styles.options}>
          <button
            type="button"
            disabled={!dimensions}
            onClick={() => run(false)}
          >
            {t('bitmapSheet.export')}
          </button>
          <label>
            {t('bitmapSheet.file')}
            <input
              type="file"
              accept="image/png,.png"
              onChange={(event) => setFile(event.target.files?.[0])}
            />
          </label>
          <button
            type="button"
            disabled={!file || !dimensions}
            onClick={() => run(true)}
          >
            {t('bitmapSheet.import')}
          </button>
        </div>
      </fieldset>
      <p className={styles.layoutHint}>{t('bitmapSheet.layoutHint')}</p>
      {busy && <p role="status">{t('bitmapSheet.busy')}</p>}
      {error && <p role="alert">{error}</p>}
      {status && <p role="status">{status}</p>}
      {description && (
        <label className={styles.mapping}>
          {t('bitmapSheet.layout')}
          <textarea readOnly rows={3} value={description} />
        </label>
      )}
      {download && (
        <div className={styles.preview}>
          <button type="button" disabled={busy} onClick={() => void save()}>
            {t('bitmapSheet.download')}
          </button>
          <img src={download} alt={t('bitmapSheet.preview')} />
        </div>
      )}
    </details>
  )
}
