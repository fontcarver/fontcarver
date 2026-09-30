import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type ShapedRun } from '../../api'
import { useLayoutQuery } from './useLayoutQuery'
import { isValidFeatureTag } from './visualLookup'
import styles from './VisualLookupPreview.module.css'

interface PreviewRequest {
  context: string
  text: string
  serial: number
}

const totalAdvance = (run: ShapedRun, axis: 'x_advance' | 'y_advance') =>
  run.glyphs.reduce((total, glyph) => total + glyph[axis], 0)
const formatNumber = (value: number) =>
  String(Number(value.toFixed(3))).replace('-', '−')

/** Compares actual applied shaping with the selected feature disabled and enabled. */
export function VisualLookupPreview({
  doc,
  master,
  generation,
  tag,
  hasDraft,
}: {
  doc: number
  master: string | null
  generation: number
  tag: string
  hasDraft: boolean
}) {
  const { t } = useTranslation()
  const [text, setText] = useState('')
  const [request, setRequest] = useState<PreviewRequest | null>(null)
  const serial = useRef(0)
  const context = JSON.stringify([doc, master, generation, tag, hasDraft])
  if (request && request.context !== context) setRequest(null)
  const active = request?.context === context ? request : null
  const answer = useLayoutQuery(
    JSON.stringify([context, active?.serial ?? null]),
    async () => {
      if (!active) return null
      return Promise.all(
        [false, true].map((enabled) =>
          call('text.shape', {
            doc,
            master,
            text: active.text,
            features: [[tag, enabled]],
            script: null,
            language: null,
            direction: null,
            size: null,
          }),
        ),
      )
    },
  )
  const runs = active ? answer?.value : null
  const loading = !!active && !answer
  const stale = runs?.some((run) => run.stale)
  const warnings = [...new Set(runs?.flatMap((run) => run.warnings) ?? [])]
  const valid = isValidFeatureTag(tag) && text.length > 0 && text.length <= 128
  return (
    <section
      className={styles.root}
      aria-label={t('visualLookupPreview.title')}
    >
      <h3>{t('visualLookupPreview.title')}</h3>
      <p className={styles.muted}>
        {t(hasDraft ? 'visualLookupPreview.draft' : 'visualLookupPreview.help')}
      </p>
      <div className={styles.controls}>
        <label className={styles.field}>
          {t('visualLookupPreview.text')}
          <input
            value={text}
            maxLength={128}
            onChange={(event) => {
              setText(event.target.value.slice(0, 128))
              setRequest(null)
            }}
          />
        </label>
        <button
          type="button"
          disabled={!valid || loading}
          onClick={() => {
            serial.current += 1
            setRequest({ context, text, serial: serial.current })
          }}
        >
          {t('visualLookupPreview.preview')}
        </button>
      </div>
      {loading && <p role="status">{t('visualLookupPreview.loading')}</p>}
      {active && answer?.failed && (
        <p role="alert" className={styles.error}>
          {t('visualLookupPreview.failed')}
        </p>
      )}
      {stale && <p role="status">{t('visualLookupPreview.stale')}</p>}
      {warnings.map((warning) => (
        <p key={warning} className={styles.muted}>
          {t('visualLookupPreview.warning', { warning })}
        </p>
      ))}
      {runs && !stale && (
        <>
          <p>
            {t('visualLookupPreview.difference', {
              x: formatNumber(
                totalAdvance(runs[1], 'x_advance') -
                  totalAdvance(runs[0], 'x_advance'),
              ),
              y: formatNumber(
                totalAdvance(runs[1], 'y_advance') -
                  totalAdvance(runs[0], 'y_advance'),
              ),
            })}
          </p>
          <div className={styles.comparison}>
            {runs.map((run, index) => (
              <div className={styles.run} key={index}>
                <table>
                  <caption>
                    {t(
                      index === 0
                        ? 'visualLookupPreview.off'
                        : 'visualLookupPreview.on',
                      { tag },
                    )}
                  </caption>
                  <thead>
                    <tr>
                      {[
                        'glyph',
                        'cluster',
                        'xAdvance',
                        'yAdvance',
                        'xOffset',
                        'yOffset',
                      ].map((column) => (
                        <th key={column} scope="col">
                          {t(`visualLookupPreview.${column}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {run.glyphs.map((glyph, glyphIndex) => (
                      <tr key={glyphIndex}>
                        <th scope="row">
                          {run.names[glyphIndex] ??
                            t('visualLookupPreview.unnamed', {
                              gid: glyph.gid,
                            })}
                        </th>
                        <td>{glyph.cluster}</td>
                        <td>{glyph.x_advance}</td>
                        <td>{glyph.y_advance}</td>
                        <td>{glyph.x_offset}</td>
                        <td>{glyph.y_offset}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {run.glyphs.length === 0 && (
                  <p>{t('visualLookupPreview.empty')}</p>
                )}
              </div>
            ))}
          </div>
          <p className={styles.muted}>{t('visualLookupPreview.units')}</p>
        </>
      )}
    </section>
  )
}
