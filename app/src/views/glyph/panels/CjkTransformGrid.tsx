import { useTranslation } from 'react-i18next'

import type { Component } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { setComponentCommand } from '../../../editor/pointCommands'
import { commitField } from '../commitField'

import styles from './CjkTransformGrid.module.css'
import type { PanelContext } from './panelContext'

// Display the two affine rows together: x = ax + cy + X, y = bx + dy + Y.
const coefficients = [
  { key: 'a', index: 0 },
  { key: 'c', index: 2 },
  { key: 'x', index: 4 },
  { key: 'b', index: 1 },
  { key: 'd', index: 3 },
  { key: 'y', index: 5 },
] as const

/** Editable affine coefficients for every component in the active layer. */
export function CjkTransformGrid({
  target,
  outline,
  reload,
}: Pick<PanelContext, 'target' | 'outline' | 'reload'>) {
  const { t } = useTranslation()

  const write = (
    component: Component,
    index: number,
    coefficient: number,
    value: number,
  ) => {
    const transform: Component['transform'] = [...component.transform]
    transform[coefficient] = value
    commitField(
      target,
      setComponentCommand(target, index, { ...component, transform }),
      t('editor.undo.setComponent'),
      reload,
    )
  }

  return (
    <div className={styles.grid}>
      <h4 className={styles.heading}>{t('cjkConstruction.transforms')}</h4>
      {outline.components.length === 0 ? (
        <p className={styles.empty}>{t('glyphInspector.noComponents')}</p>
      ) : (
        outline.components.map(({ component }, index) => (
          <fieldset
            className={styles.component}
            key={`${target.doc}:${target.glyph}:${target.layer}:${index}:${component.base}`}
          >
            <legend className={styles.name}>
              {t('cjkConstruction.component', {
                index: index + 1,
                base: component.base,
              })}
            </legend>
            <div className={styles.fields}>
              {coefficients.map((coefficient) => {
                const label = t(
                  `cjkConstruction.coefficients.${coefficient.key}`,
                )
                return (
                  <div className={styles.coefficient} key={coefficient.key}>
                    <span aria-hidden="true" className={styles.caption}>
                      {label}
                    </span>
                    <NumberField
                      label={t('cjkConstruction.transformField', {
                        coefficient: label,
                        index: index + 1,
                        base: component.base,
                      })}
                      value={component.transform[coefficient.index]}
                      formatValue={String}
                      revision={outline}
                      step={coefficient.index < 4 ? 0.05 : 1}
                      invalidHint={t('infoBox.invalid')}
                      onCommit={(value) =>
                        write(component, index, coefficient.index, value)
                      }
                    />
                  </div>
                )
              })}
            </div>
          </fieldset>
        ))
      )}
    </div>
  )
}
