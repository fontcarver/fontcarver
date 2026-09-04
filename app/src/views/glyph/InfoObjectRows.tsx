import { useTranslation } from 'react-i18next'

import { NumberField } from '../../components/NumberField'
import {
  setAnchorCommand,
  setComponentCommand,
  setGuideCommand,
} from '../../editor/pointCommands'
import { onlyTarget } from '../../editor/selection'

import { commitField } from './commitField'
import styles from './infoBox.module.css'
import type { InfoRowsProps } from './infoBoxProps'

/** Decomposes a component transform into offset, scale and rotation. */
function decompose(transform: readonly number[]): {
  x: number
  y: number
  scaleX: number
  scaleY: number
  rotation: number
} {
  const [a = 1, b = 0, c = 0, d = 1, e = 0, f = 0] = transform
  const scaleX = Math.hypot(a, b) || 1
  const scaleY = Math.hypot(c, d) || 1
  return {
    x: e,
    y: f,
    scaleX: scaleX * 100,
    scaleY: scaleY * 100,
    rotation: (Math.atan2(b, a) * 180) / Math.PI,
  }
}

/** Rebuilds a transform from offset, scale and rotation. */
function compose(
  x: number,
  y: number,
  scaleX: number,
  scaleY: number,
  rotation: number,
): [number, number, number, number, number, number] {
  const radians = (rotation * Math.PI) / 180
  const cos = Math.cos(radians)
  const sin = Math.sin(radians)
  return [
    (scaleX / 100) * cos,
    (scaleX / 100) * sin,
    (-scaleY / 100) * sin,
    (scaleY / 100) * cos,
    x,
    y,
  ]
}

/**
 * A single component, anchor or guide (design spec §6.1).
 *
 * These three share a shape — a name plus a handful of numbers — and one
 * command each (`SetComponent`, `SetAnchor`, `SetGuide`), so they share a row
 * group rather than three near-identical ones.
 */
export function InfoObjectRows({
  target,
  outline,
  selection,
  master,
  onCommitted,
}: InfoRowsProps) {
  const { t } = useTranslation()
  const only = onlyTarget(selection)

  if (only?.kind === 'component') {
    const view = outline.components[only.index]
    if (!view) return null
    const parts = decompose(view.component.transform)
    const write = (
      next: Partial<{
        x: number
        y: number
        scaleX: number
        scaleY: number
        rotation: number
      }>,
    ) => {
      const merged = { ...parts, ...next }
      commitField(
        target,
        setComponentCommand(target, only.index, {
          ...view.component,
          transform: compose(
            merged.x,
            merged.y,
            merged.scaleX,
            merged.scaleY,
            merged.rotation,
          ),
        }),
        t('editor.undo.setComponent'),
        onCommitted,
      )
    }
    return (
      <>
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.base')}</span>
          <span className={styles.rowValue}>{view.component.base}</span>
        </div>
        <NumberField
          label={t('infoBox.x')}
          value={parts.x}
          invalidHint={t('infoBox.invalid')}
          onCommit={(x) => write({ x })}
        />
        <NumberField
          label={t('infoBox.y')}
          value={parts.y}
          invalidHint={t('infoBox.invalid')}
          onCommit={(y) => write({ y })}
        />
        <NumberField
          label={t('infoBox.scaleX')}
          value={parts.scaleX}
          invalidHint={t('infoBox.invalid')}
          onCommit={(scaleX) => write({ scaleX })}
        />
        <NumberField
          label={t('infoBox.scaleY')}
          value={parts.scaleY}
          invalidHint={t('infoBox.invalid')}
          onCommit={(scaleY) => write({ scaleY })}
        />
        <NumberField
          label={t('infoBox.rotation')}
          value={parts.rotation}
          invalidHint={t('infoBox.invalid')}
          onCommit={(rotation) => write({ rotation })}
        />
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.autoAlign')}</span>
          <button
            type="button"
            className={styles.toggle}
            aria-pressed={view.component.auto_align}
            onClick={() =>
              commitField(
                target,
                setComponentCommand(target, only.index, {
                  ...view.component,
                  auto_align: !view.component.auto_align,
                }),
                t('editor.undo.setComponent'),
                onCommitted,
              )
            }
          >
            {view.component.auto_align ? t('common.yes') : t('common.no')}
          </button>
        </div>
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.useMyMetrics')}</span>
          <button
            type="button"
            className={styles.toggle}
            aria-pressed={view.component.use_my_metrics}
            onClick={() =>
              commitField(
                target,
                setComponentCommand(target, only.index, {
                  ...view.component,
                  use_my_metrics: !view.component.use_my_metrics,
                }),
                t('editor.undo.setComponent'),
                onCommitted,
              )
            }
          >
            {view.component.use_my_metrics ? t('common.yes') : t('common.no')}
          </button>
        </div>
      </>
    )
  }

  if (only?.kind === 'anchor') {
    const anchor = outline.anchors[only.index]
    if (!anchor) return null
    const write = (next: Partial<typeof anchor>) =>
      commitField(
        target,
        setAnchorCommand(target, only.index, { ...anchor, ...next }),
        t('editor.undo.setAnchor'),
        onCommitted,
      )
    return (
      <>
        <label className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.name')}</span>
          <input
            className={styles.textInput}
            defaultValue={anchor.name}
            aria-label={t('infoBox.name')}
            onBlur={(event) => {
              const name = event.currentTarget.value.trim()
              if (name && name !== anchor.name) write({ name })
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur()
            }}
          />
        </label>
        <NumberField
          label={t('infoBox.x')}
          value={anchor.x}
          invalidHint={t('infoBox.invalid')}
          onCommit={(x) => write({ x })}
        />
        <NumberField
          label={t('infoBox.y')}
          value={anchor.y}
          invalidHint={t('infoBox.invalid')}
          onCommit={(y) => write({ y })}
        />
      </>
    )
  }

  if (only?.kind === 'guide') {
    const guide =
      only.scope === 'layer' ? outline.guides[only.index] : undefined
    if (!guide) return null
    const write = (next: Partial<typeof guide>) =>
      commitField(
        target,
        setGuideCommand(target, only.scope, master, only.index, {
          ...guide,
          ...next,
        }),
        t('editor.undo.setGuide'),
        onCommitted,
      )
    return (
      <>
        <NumberField
          label={t('infoBox.x')}
          value={guide.x}
          invalidHint={t('infoBox.invalid')}
          onCommit={(x) => write({ x })}
        />
        <NumberField
          label={t('infoBox.y')}
          value={guide.y}
          invalidHint={t('infoBox.invalid')}
          onCommit={(y) => write({ y })}
        />
        <NumberField
          label={t('infoBox.angle')}
          value={guide.angle}
          invalidHint={t('infoBox.invalid')}
          onCommit={(angle) => write({ angle })}
        />
        <div className={styles.row}>
          <span className={styles.rowLabel}>{t('infoBox.locked')}</span>
          <button
            type="button"
            className={styles.toggle}
            aria-pressed={guide.locked}
            onClick={() => write({ locked: !guide.locked })}
          >
            {guide.locked ? t('common.yes') : t('common.no')}
          </button>
        </div>
      </>
    )
  }

  return null
}
