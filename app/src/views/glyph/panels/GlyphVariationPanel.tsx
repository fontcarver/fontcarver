import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  call,
  onDocumentEvent,
  type DocId,
  type VariationPreview as Preview,
} from '../../../api'
import { useVariationPreviewStore } from '../../../stores/variationPreviewStore'
import { errorNotice } from '../../../i18n/errors'
import { VariationPreview } from '../../fontinfo/VariationPreview'
import type { DesignSpaceValue } from '../../fontinfo/designSpaceSession'
import styles from '../../fontinfo/designSpace.module.css'
import { PanelSection } from './PanelSection'

/** Live interpolation beside the editable source, with no mutation while scrubbing. */
export function GlyphVariationPanel({
  doc,
  glyph,
}: {
  doc: DocId
  glyph: string
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<DesignSpaceValue | null>(null)
  const [error, setError] = useState<unknown>(null)
  const [refresh, setRefresh] = useState(0)
  const publishPreview = useCallback(
    (preview: Preview | null) => {
      const store = useVariationPreviewStore.getState()
      if (preview) store.show(doc, glyph, preview)
      else store.clear(doc, glyph)
    },
    [doc, glyph],
  )
  useEffect(
    () => () => useVariationPreviewStore.getState().clear(doc, glyph),
    [doc, glyph],
  )
  useEffect(() => {
    let request = 0
    let disposed = false
    const load = () => {
      const current = ++request
      void call('variation.state', { doc })
        .then((value) => {
          if (disposed || current !== request) return
          setState({
            ...value,
            first_glyph: glyph,
            axes: value.axes.map((axis) => ({
              ...axis,
              tag: String.fromCharCode(...axis.tag),
            })),
          })
          setError(null)
        })
        .catch((error: unknown) => {
          if (!disposed && current === request) setError(error)
        })
    }
    load()
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) load()
    })
    return () => {
      disposed = true
      off()
    }
  }, [doc, glyph, refresh])
  return (
    <PanelSection
      id="variation"
      title={t('variation.preview')}
      defaultOpen={false}
    >
      {error != null && <p role="alert">{errorNotice(error, t).message}</p>}
      {state && (
        <div className={styles.editor}>
          <VariationPreview
            key={`${doc}:${glyph}`}
            doc={doc}
            state={state}
            fixedGlyph={glyph}
            onPreviewChange={publishPreview}
            onRefresh={() => setRefresh((value) => value + 1)}
          />
        </div>
      )}
    </PanelSection>
  )
}
