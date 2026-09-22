import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { call, type DocId, type Glyph, type SmartAxis } from '../../../api'
import { Button } from '../../../components/Button'
import { Dialog } from '../../../components/Dialog'
import { NumberField } from '../../../components/NumberField'
import { activeEditor } from '../../../editor/activeEditor'
import { errorNotice } from '../../../i18n/errors'
import styles from '../../fontinfo/designSpace.module.css'

/** Local component axes are authored independently of the font's global axes. */
export function SmartGlyphEditor({
  doc,
  glyph,
  onClose,
  onApplied,
}: {
  doc: DocId
  glyph: Glyph
  onClose: () => void
  onApplied: () => void
}) {
  const { t } = useTranslation()
  const [axes, setAxes] = useState<SmartAxis[]>(glyph.smart_axes)
  const [generation, setGeneration] = useState<number>()
  const [busy, setBusy] = useState(false),
    [error, setError] = useState('')
  useEffect(() => {
    let current = true
    void Promise.resolve(activeEditor()?.settle())
      .then(() => call('variation.state', { doc }))
      .then((value) => {
        if (current) setGeneration(value.generation)
      })
      .catch((error: unknown) => {
        if (current) {
          const notice = errorNotice(error, t)
          setError(notice.details ?? notice.message)
        }
      })
    return () => {
      current = false
    }
  }, [doc, t])
  const apply = () => {
    if (generation === undefined || busy) return
    setBusy(true)
    void call('variation.apply', {
      doc,
      expected_generation: generation,
      label: t('variation.smartAxes'),
      command: { SmartAxes: { glyph: glyph.name, axes } },
    })
      .then(() => {
        onApplied()
        onClose()
      })
      .catch((error: unknown) => {
        const notice = errorNotice(error, t)
        setError(notice.details ?? notice.message)
      })
      .finally(() => setBusy(false))
  }
  return (
    <Dialog
      title={t('variation.smartAxes')}
      size="lg"
      onCancel={() => {
        if (!busy) onClose()
      }}
      scrollBody
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={busy || generation === undefined}
            onClick={apply}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div className={styles.editor}>
        <p>{t('variation.smartHint')}</p>
        {error && <p role="alert">{error}</p>}
        <fieldset className={styles.form} disabled={busy}>
          {axes.map((axis, index) => (
            <fieldset key={index} className={styles.card}>
              <div className={styles.row}>
                <label>
                  {t('fontInfo.column.axisName')}
                  <input
                    value={axis.name}
                    onChange={(e) =>
                      setAxes((current) =>
                        current.map((a, i) =>
                          i === index ? { ...a, name: e.target.value } : a,
                        ),
                      )
                    }
                  />
                </label>
                <Button
                  onClick={() =>
                    setAxes((current) => current.filter((_, i) => i !== index))
                  }
                >
                  {t('variation.removeAxis')}
                </Button>
              </div>
              <div className={styles.row}>
                {(['min', 'default', 'max'] as const).map((key) => (
                  <NumberField
                    key={key}
                    label={t(`fontInfo.column.${key}`)}
                    value={axis[key]}
                    onCommit={(number) =>
                      setAxes((current) =>
                        current.map((a, i) =>
                          i === index ? { ...a, [key]: number } : a,
                        ),
                      )
                    }
                  />
                ))}
              </div>
            </fieldset>
          ))}
          <Button
            disabled={axes.length >= 8}
            onClick={() =>
              setAxes((current) => [
                ...current,
                {
                  name: t('variation.newLocalAxis', { count: axes.length + 1 }),
                  min: 0,
                  default: 0,
                  max: 100,
                },
              ])
            }
          >
            {t('variation.addAxis')}
          </Button>
        </fieldset>
      </div>
    </Dialog>
  )
}
