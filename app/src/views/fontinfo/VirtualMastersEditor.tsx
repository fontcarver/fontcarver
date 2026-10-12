import { useTranslation } from 'react-i18next'
import type { VirtualMaster } from '../../api'
import { Button } from '../../components/Button'
import { NumberField } from '../../components/NumberField'
import type { DesignSpaceValue } from './designSpaceSession'
import { designDefault, userToDesign } from './designCoordinates'
import styles from './designSpace.module.css'

/** Named sparse-source positions, staged in the same transaction as axis edits. */
export function VirtualMastersEditor({
  state,
  onChange,
}: {
  state: DesignSpaceValue
  onChange: (masters: VirtualMaster[]) => void
}) {
  const { t } = useTranslation()
  const masters = state.virtual_masters ?? []
  const edit = (index: number, next: VirtualMaster) =>
    onChange(masters.map((master, i) => (i === index ? next : master)))
  const add = () => {
    const location = Object.fromEntries(
      state.axes.map((axis) => [axis.tag, designDefault(axis)]),
    )
    const positions = [...state.masters, ...masters]
    for (const axis of state.axes) {
      const candidate = [
        userToDesign(axis, axis.max),
        userToDesign(axis, axis.min),
        (userToDesign(axis, axis.min) + userToDesign(axis, axis.max)) / 2,
      ].find((value) =>
        positions.every((master) =>
          state.axes.some(
            (other) =>
              (master.location[other.tag] ?? designDefault(other)) !==
              (other.tag === axis.tag ? value : location[other.tag]),
          ),
        ),
      )
      if (candidate !== undefined) {
        location[axis.tag] = candidate
        break
      }
    }
    let count = masters.length + 1
    while (
      masters.some(
        (master) => master.name === t('virtualMaster.newName', { count }),
      )
    )
      count++
    onChange([
      ...masters,
      { name: t('virtualMaster.newName', { count }), location },
    ])
  }
  return (
    <section aria-label={t('virtualMaster.title')}>
      <h3>{t('virtualMaster.title')}</h3>
      <p>{t('virtualMaster.help')}</p>
      {masters.map((master, index) => (
        <fieldset key={index} className={styles.card}>
          <legend>{master.name}</legend>
          <label>
            {t('virtualMaster.name')}
            <input
              value={master.name}
              onChange={(event) =>
                edit(index, { ...master, name: event.target.value })
              }
            />
          </label>
          <div className={styles.row}>
            {state.axes.map((axis) => (
              <NumberField
                key={axis.tag}
                label={axis.tag}
                value={master.location[axis.tag] ?? designDefault(axis)}
                onCommit={(value) =>
                  edit(index, {
                    ...master,
                    location: { ...master.location, [axis.tag]: value },
                  })
                }
              />
            ))}
            <Button
              onClick={() => onChange(masters.filter((_, i) => i !== index))}
            >
              {t('virtualMaster.remove')}
            </Button>
          </div>
        </fieldset>
      ))}
      <Button
        disabled={!state.axes.length || masters.length >= 1024}
        onClick={add}
      >
        {t('virtualMaster.add')}
      </Button>
    </section>
  )
}
