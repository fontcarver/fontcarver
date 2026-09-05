import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Component } from '../../../api'
import { activeEditor } from '../../../editor/activeEditor'
import { setComponentCommand } from '../../../editor/pointCommands'
import { selectionOf } from '../../../editor/selection'
import { reportError } from '../../../shell/commands'
import { useEditorStore } from '../../../stores/editorStore'
import { commitField } from '../commitField'
import { openGlyphByName } from '../openBase'

import { ComponentRow } from './ComponentRow'
import { GlyphPicker } from './GlyphPicker'
import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  addComponentCommand,
  decomposeComponentsCommand,
  removeComponentsCommand,
  reorderComponentsCommand,
} from './panelCommands'

/**
 * The Components section (design spec §7.4): the list of references, their
 * offset and scale, drag reordering, the search popover that adds one, and
 * decompose. Selecting a row selects the component on the canvas, so the Info
 * box and the panel always describe the same thing.
 */
export function ComponentsPanel({ target, outline, reload }: PanelContext) {
  const { t } = useTranslation()
  const [picking, setPicking] = useState(false)
  const [dragging, setDragging] = useState<number | null>(null)
  const selection = useEditorStore((state) => state.selection)
  const components = outline.components

  const selected = (index: number) => selection.has(`component:${index}`)

  const run = (command: Parameters<typeof commitField>[1], label: string) =>
    commitField(target, command, label, reload)

  const writeComponent = (index: number, component: Component) =>
    run(
      setComponentCommand(target, index, component),
      t('editor.undo.setComponent'),
    )

  const move = (from: number, to: number) => {
    const command = reorderComponentsCommand(
      target,
      components.map((view) => view.component),
      from,
      to,
    )
    if (command) run(command, t('editor.undo.reorderComponents'))
  }

  if (components.length === 0 && !picking)
    return (
      <>
        <p className={styles.empty}>{t('glyphInspector.noComponents')}</p>
        <div className={styles.actions}>
          <button
            type="button"
            className={styles.actionButton}
            onClick={() => setPicking(true)}
          >
            {t('panel.components.add')}
          </button>
        </div>
      </>
    )

  return (
    <>
      <div className={styles.list}>
        {components.map((view, index) => (
          <ComponentRow
            key={`${view.component.base}-${index}`}
            view={view}
            index={index}
            count={components.length}
            selected={selected(index)}
            dragging={dragging === index}
            onSelect={() =>
              activeEditor()?.setSelection(
                selectionOf([{ kind: 'component', index }]),
              )
            }
            onOpenBase={() => {
              void openGlyphByName(target.doc, view.component.base).catch(
                reportError,
              )
            }}
            onWrite={(component) => writeComponent(index, component)}
            onRemove={() =>
              run(
                removeComponentsCommand(target, [index]),
                t('editor.undo.removeComponents'),
              )
            }
            onDecompose={() =>
              run(
                decomposeComponentsCommand(target, [index]),
                t('editor.undo.decompose'),
              )
            }
            onMove={(to) => move(dragging ?? index, to)}
            onDragStart={() => setDragging(index)}
            onDragEnd={() => setDragging(null)}
          />
        ))}
      </div>

      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          aria-expanded={picking}
          onClick={() => setPicking((open) => !open)}
        >
          {t('panel.components.add')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={components.length === 0}
          onClick={() =>
            run(
              decomposeComponentsCommand(
                target,
                components.map((_, index) => index),
              ),
              t('editor.undo.decompose'),
            )
          }
        >
          {t('panel.components.decomposeAll')}
        </button>
      </div>

      {picking ? (
        <GlyphPicker
          doc={target.doc}
          label={t('panel.components.add')}
          onClose={() => setPicking(false)}
          onPick={(name) => {
            setPicking(false)
            run(
              addComponentCommand(target, name),
              t('editor.undo.addComponent'),
            )
          }}
        />
      ) : null}
    </>
  )
}
