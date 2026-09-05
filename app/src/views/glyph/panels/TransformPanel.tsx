import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Command } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import {
  movePointsCommand,
  transformSelectionCommand,
} from '../../../editor/pointCommands'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  alignMoves,
  isNeutral,
  NEUTRAL_TRANSFORM,
  ORIGIN_HANDLES,
  originPoint,
  transformAffine,
  translate,
  type AlignMode,
  type TransformInput,
} from './transformMath'
import { alignItems, transformTargets } from './transformTargets'

/** The align and distribute buttons, in the order design spec §7.7 lists them. */
const ALIGNMENTS: readonly { mode: AlignMode; glyph: string }[] = [
  { mode: 'left', glyph: '⇤' },
  { mode: 'centerH', glyph: '⇔' },
  { mode: 'right', glyph: '⇥' },
  { mode: 'top', glyph: '⤒' },
  { mode: 'middle', glyph: '⇕' },
  { mode: 'bottom', glyph: '⤓' },
  { mode: 'distributeH', glyph: '⇹' },
  { mode: 'distributeV', glyph: '⤄' },
]

/**
 * The Transform section (design spec §7.7).
 *
 * Apply composes one affine locally and sends a single `TransformSelection`,
 * so a move, a scale, a rotation and a shear together are one undo entry.
 * Align and distribute compute their deltas here as well and commit one
 * `Batch`, because the kernel has no notion of "align" — only of moving the
 * things the interface names.
 */
export function TransformPanel({ target, outline, reload }: PanelContext) {
  const { t } = useTranslation()
  const selection = useEditorStore((state) => state.selection)
  const [input, setInput] = useState<TransformInput>(NEUTRAL_TRANSFORM)
  const [origin, setOrigin] = useState(4)
  const [locked, setLocked] = useState(false)

  const targets = transformTargets(outline, selection)
  const box = targets.box
  const pivot = box ? originPoint(box, origin) : { x: 0, y: 0 }
  const set = (patch: Partial<TransformInput>) =>
    setInput((current) => ({ ...current, ...patch }))

  const apply = (affine: ReturnType<typeof transformAffine>, label: string) => {
    if (!box) return
    commitField(
      target,
      transformSelectionCommand(target, targets, affine),
      label,
      reload,
    )
  }

  const items = alignItems(outline, selection)

  const align = (mode: AlignMode) => {
    const moves = alignMoves(items, mode)
    if (moves.length === 0) return
    const writes: Command[] = moves.map((move) =>
      move.item.points.length > 0
        ? movePointsCommand(
            target,
            move.item.points,
            move.dx,
            move.dy,
            'WithHandles',
          )
        : transformSelectionCommand(
            target,
            {
              points: [],
              components:
                move.item.component === undefined ? [] : [move.item.component],
              anchors: move.item.anchor === undefined ? [] : [move.item.anchor],
            },
            translate(move.dx, move.dy),
          ),
    )
    commitField(
      target,
      writes.length === 1 && writes[0] ? writes[0] : { Batch: writes },
      t(`editor.undo.align.${mode}`),
      reload,
    )
  }

  return (
    <>
      <div className={styles.grid2}>
        <NumberField
          label={t('panel.transform.dx')}
          value={input.dx}
          invalidHint={t('infoBox.invalid')}
          onCommit={(dx) => set({ dx })}
        />
        <NumberField
          label={t('panel.transform.dy')}
          value={input.dy}
          invalidHint={t('infoBox.invalid')}
          onCommit={(dy) => set({ dy })}
        />
        <NumberField
          label={t('infoBox.scaleX')}
          value={input.scaleX}
          invalidHint={t('infoBox.invalid')}
          onCommit={(scaleX) =>
            set(locked ? { scaleX, scaleY: scaleX } : { scaleX })
          }
        />
        <NumberField
          label={t('infoBox.scaleY')}
          value={input.scaleY}
          invalidHint={t('infoBox.invalid')}
          onCommit={(scaleY) =>
            set(locked ? { scaleX: scaleY, scaleY } : { scaleY })
          }
        />
        <NumberField
          label={t('infoBox.rotation')}
          value={input.rotate}
          invalidHint={t('infoBox.invalid')}
          onCommit={(rotate) => set({ rotate })}
        />
        <NumberField
          label={t('panel.transform.skewX')}
          value={input.skewX}
          invalidHint={t('infoBox.invalid')}
          onCommit={(skewX) => set({ skewX })}
        />
        <NumberField
          label={t('panel.transform.skewY')}
          value={input.skewY}
          invalidHint={t('infoBox.invalid')}
          onCommit={(skewY) => set({ skewY })}
        />
        <button
          type="button"
          className={styles.toggle}
          aria-pressed={locked}
          onClick={() => setLocked((value) => !value)}
        >
          {t('infoBox.proportional')}
        </button>
      </div>

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.origin')}</span>
        <span
          className={styles.origin}
          role="radiogroup"
          aria-label={t('infoBox.origin')}
        >
          {ORIGIN_HANDLES.map((_, index) => (
            <button
              key={index}
              type="button"
              role="radio"
              aria-checked={origin === index}
              aria-label={t(`infoBox.originPoint.${index}`)}
              className={styles.originDot}
              onClick={() => setOrigin(index)}
            />
          ))}
        </span>
      </div>

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={box === null || isNeutral(input)}
          onClick={() => {
            apply(transformAffine(input, pivot), t('editor.undo.transform'))
            setInput(NEUTRAL_TRANSFORM)
          }}
        >
          {t('panel.transform.apply')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={box === null}
          onClick={() =>
            apply(
              transformAffine({ ...NEUTRAL_TRANSFORM, flipH: true }, pivot),
              t('editor.undo.flipH'),
            )
          }
        >
          {t('panel.transform.flipH')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={box === null}
          onClick={() =>
            apply(
              transformAffine({ ...NEUTRAL_TRANSFORM, flipV: true }, pivot),
              t('editor.undo.flipV'),
            )
          }
        >
          {t('panel.transform.flipV')}
        </button>
      </div>

      <div
        className={styles.actions}
        role="group"
        aria-label={t('panel.transform.alignGroup')}
      >
        {ALIGNMENTS.map((entry) => (
          <button
            key={entry.mode}
            type="button"
            className={styles.actionButton}
            aria-label={t(`panel.transform.align.${entry.mode}`)}
            title={t(`panel.transform.align.${entry.mode}`)}
            disabled={alignMoves(items, entry.mode).length === 0}
            onClick={() => align(entry.mode)}
          >
            {entry.glyph}
          </button>
        ))}
      </div>

      <p className={styles.hint}>
        {targets.wholeLayer
          ? t('panel.transform.wholeLayer')
          : t('panel.transform.selection', {
              count: targets.points.length,
            })}
      </p>
    </>
  )
}
