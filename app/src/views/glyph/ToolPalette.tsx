import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'

import { selectActiveTool, useEditorStore } from '../../stores/editorStore'
import { TOOLS, type ToolId } from '../../editor/tools/registry'
import { formatShortcut } from '../../shell/shortcuts'
import { toolShortcut } from '../../shell/toolShortcuts'
import { useKeymapStore } from '../../stores/keymapStore'

import styles from './toolPalette.module.css'

/** Which group boundary needs a hairline above it. */
function startsGroup(index: number): boolean {
  if (index === 0) return false
  return TOOLS[index]?.group !== TOOLS[index - 1]?.group
}

/**
 * The tool strip (design spec §3, §5).
 *
 * Every tool of the table is rendered from the start so the strip, its
 * tooltips and the keyboard map are complete; the ones that are not live yet
 * still switch and set their cursor, which is what makes the surface honest
 * rather than a wall of disabled buttons.
 */
export function ToolPalette() {
  const { t } = useTranslation()
  const active = useEditorStore(selectActiveTool)
  const borrowed = useEditorStore((state) => state.borrowedTool)
  const setTool = useEditorStore((state) => state.setTool)
  const overrides = useKeymapStore((state) => state.overrides)
  const hint = t('shortcuts.toolHint')

  return (
    <div
      className={styles.strip}
      role="toolbar"
      aria-orientation="vertical"
      aria-label={t('tool.palette')}
    >
      {TOOLS.map((tool, index) => {
        const Icon = tool.icon
        const label = t(tool.labelKey)
        const binding = toolShortcut(tool.id, overrides)
        const isActive = active === tool.id
        const classes = [styles.tool]
        if (isActive) classes.push(styles.active)
        if (borrowed === tool.id) classes.push(styles.borrowed)
        return (
          <Fragment key={tool.id as ToolId}>
            {startsGroup(index) ? (
              <span className={styles.separator} role="separator" />
            ) : null}
            <button
              type="button"
              className={classes.join(' ')}
              aria-label={label}
              aria-pressed={isActive}
              aria-description={hint}
              data-help-topic="glyph"
              title={`${binding ? `${label} (${formatShortcut(binding)})` : label}\n${hint}`}
              onClick={() => setTool(tool.id)}
            >
              <Icon size={18} strokeWidth={1.5} aria-hidden="true" />
            </button>
          </Fragment>
        )
      })}
    </div>
  )
}
