import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Component, SmartAxis } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { SmartAxisSlider } from './SmartAxisSlider'
import { SmartComponentHandle } from './SmartComponentHandle'
import {
  canonicalSmart,
  clampSmart,
  sameSmart,
  smartValue,
} from './smartComponentValues'
import styles from './smartComponentControls.module.css'

/** Local axis editing preserves the instance transform and every other parameter. */
export function SmartComponentControls({
  axes,
  component,
  disabled = false,
  onWrite,
}: {
  axes: SmartAxis[]
  component: Component
  disabled?: boolean
  onWrite: (component: Component) => void
}) {
  const { t } = useTranslation()
  const [values, setValues] = useState(component.smart ?? {})
  const [source, setSource] = useState(component)
  if (source !== component) {
    setSource(component)
    setValues(component.smart ?? {})
  }
  const draft = useRef(component.smart ?? {})
  const committed = useRef(component.smart ?? {})
  useEffect(() => {
    const next = component.smart ?? {}
    committed.current = next
    draft.current = next
  }, [component])
  const preview = (next: Record<string, number>) => {
    draft.current = { ...draft.current, ...next }
    setValues(draft.current)
  }
  const commit = () => {
    if (disabled || sameSmart(axes, draft.current, committed.current)) return
    const smart = canonicalSmart(axes, draft.current)
    committed.current = smart
    draft.current = smart
    setValues(smart)
    onWrite({ ...component, smart: Object.keys(smart).length ? smart : null })
  }
  const cancel = () => {
    draft.current = committed.current
    setValues(committed.current)
  }
  if (!axes.length) return null
  return (
    <fieldset className={styles.controls} disabled={disabled}>
      <legend>{t('panel.smartComponent.title')}</legend>
      {axes.map((axis) => (
        <div key={axis.name} className={styles.axis}>
          <NumberField
            label={axis.name}
            value={smartValue(axis, values)}
            revision={values}
            disabled={disabled}
            step={(axis.max - axis.min) / 100}
            onCommit={(value) => {
              preview({ [axis.name]: clampSmart(axis, value) })
              commit()
            }}
          />
          <SmartAxisSlider
            axis={axis}
            value={smartValue(axis, values)}
            disabled={disabled}
            onPreview={(value) => preview({ [axis.name]: value })}
            onCommit={commit}
            onCancel={cancel}
          />
        </div>
      ))}
      <SmartComponentHandle
        axes={axes}
        values={values}
        disabled={disabled}
        onPreview={preview}
        onCommit={commit}
        onCancel={cancel}
      />
      <button
        type="button"
        className={styles.reset}
        disabled={disabled}
        onClick={() => {
          preview(
            Object.fromEntries(axes.map((axis) => [axis.name, axis.default])),
          )
          commit()
        }}
      >
        {t('panel.smartComponent.reset')}
      </button>
    </fieldset>
  )
}
