import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { onDocumentEvent, type DocId } from '../../api'
import { Button } from '../../components/Button'
import { Dialog } from '../../components/Dialog'
import { NumberField } from '../../components/NumberField'
import { errorNotice } from '../../i18n/errors'
import { reportError } from '../../stores/uiStore'
import { useFontInfo } from '../../stores/fontInfoCache'
import { AxisEditor } from './AxisEditor'
import {
  DesignSpaceSession,
  type EditableAxis as Axis,
  type DesignSpaceValue as VariationState,
} from './designSpaceSession'
import { VariationPreview } from './VariationPreview'
import { MasterMetricsEditor } from './MasterMetricsEditor'
import { InstanceEditor } from './InstanceEditor'
import { VirtualMastersEditor } from './VirtualMastersEditor'
import { designDefault, userToDesign } from './designCoordinates'
import styles from './designSpace.module.css'

/** A staged, atomic editor for axes, master sources and named instances. */
export function DesignSpaceEditor({
  doc,
  onClose,
}: {
  doc: DocId
  onClose: () => void
}) {
  const { t } = useTranslation()
  const { info: fontInfo } = useFontInfo(doc)
  const root = useRef<HTMLDivElement>(null)
  const [session] = useState(
    () => new DesignSpaceSession(doc, t('variation.edit')),
  )
  const state = useSyncExternalStore(session.subscribe, session.snapshot),
    value = state.value
  const commitField = () => {
    const active = document.activeElement
    if (active instanceof HTMLElement && root.current?.contains(active))
      active.blur()
  }
  useEffect(() => {
    const dispose = session.connect(commitField)
    void session.load().catch(reportError)
    const off = onDocumentEvent((event) => {
      if (event.doc === doc) void session.load().catch(reportError)
    })
    return () => {
      off()
      dispose()
    }
  }, [session, doc])
  const disabled = state.busy || state.suspended
  const edit = (change: Partial<VariationState>) => {
    if (value) session.edit({ ...value, ...change })
  }
  const save = () => {
    commitField()
    void session.flush().catch(reportError)
  }
  const close = () => {
    commitField()
    void session.flush().then(onClose).catch(reportError)
  }
  const cancel = () => {
    void session.discard().then(onClose).catch(reportError)
  }
  const updateAxis = (index: number, axis: Axis) => {
    if (!value) return
    const old = value.axes[index]!.tag
    session.renameAxis(old, axis.tag)
    const rename = (location: Record<string, number>) => {
      const next = { ...location }
      if (old !== axis.tag && old in next) {
        next[axis.tag] = next[old]!
        delete next[old]
      }
      return next
    }
    edit({
      axes: value.axes.map((a, i) => (i === index ? axis : a)),
      masters: value.masters.map((m) => ({
        ...m,
        location: rename(m.location),
      })),
      instances: value.instances.map((i) => ({
        ...i,
        location: rename(i.location),
      })),
      virtual_masters: (value.virtual_masters ?? []).map((m) => ({
        ...m,
        location: rename(m.location),
      })),
    })
  }
  const addAxis = () => {
    if (!value) return
    const tag = [
      'wght',
      'wdth',
      'opsz',
      ...Array.from(
        { length: 16 },
        (_, i) => `AX${String(i).padStart(2, '0')}`,
      ),
    ].find((tag) => !value.axes.some((a) => a.tag === tag))!
    const axis: Axis = {
      tag,
      name: tag === 'wght' ? t('variation.weight') : tag,
      min: 100,
      default: 400,
      max: 900,
      hidden: false,
      map: [],
    }
    edit({
      axes: [...value.axes, axis],
      masters: value.masters.map((m) => ({
        ...m,
        location: { ...m.location, [tag]: axis.default },
      })),
    })
  }
  const removeAxis = (index: number) => {
    if (!value) return
    const tag = value.axes[index]!.tag
    const omit = (location: Record<string, number>) =>
      Object.fromEntries(
        Object.entries(location).filter(([key]) => key !== tag),
      )
    edit({
      axes: value.axes.filter((_, i) => i !== index),
      masters: value.masters.map((m) => ({ ...m, location: omit(m.location) })),
      instances: value.instances.map((i) => ({
        ...i,
        location: omit(i.location),
      })),
      virtual_masters: (value.virtual_masters ?? []).map((m) => ({
        ...m,
        location: omit(m.location),
      })),
    })
  }
  const addMaster = () => {
    if (!value?.masters.length) return
    const master = session.copySource()
    if (!master) return
    master.location = { ...value.masters[0]!.location }
    master.id = crypto.randomUUID().replaceAll('-', '')
    master.name = t('variation.newMaster', { count: value.masters.length + 1 })
    const axis = value.axes[0]
    if (axis) {
      const low = userToDesign(axis, axis.min),
        high = userToDesign(axis, axis.max)
      master.location[axis.tag] =
        [high, low, (high + low) / 2].find(
          (n) => !value.masters.some((m) => m.location[axis.tag] === n),
        ) ?? high
    }
    edit({ masters: [...value.masters, master] })
  }
  const addInstance = () => {
    if (value)
      edit({
        instances: [
          ...value.instances,
          {
            name: t('variation.newInstance', {
              count: value.instances.length + 1,
            }),
            style_name: '',
            family_name: null,
            location: Object.fromEntries(
              value.axes.map((a) => [a.tag, designDefault(a)]),
            ),
            active: true,
            is_variable_named_instance: true,
            user_data: null,
          },
        ],
      })
  }
  const error = state.error ? errorNotice(state.error, t) : null
  return (
    <Dialog
      title={t('variation.title')}
      size="lg"
      scrollBody
      onCancel={close}
      footer={
        <>
          <Button disabled={disabled} onClick={cancel}>
            {t('variation.discard')}
          </Button>
          <Button disabled={disabled} onClick={close}>
            {t('common.close')}
          </Button>
          <Button
            disabled={disabled || !state.dirty}
            variant="primary"
            onClick={save}
          >
            {t('variation.apply')}
          </Button>
        </>
      }
    >
      <div ref={root} className={styles.editor}>
        {error && (
          <div role="alert">
            <p>{error.message}</p>
            {error.details && <p>{error.details}</p>}
            <Button
              disabled={disabled}
              onClick={() => {
                void session.discard().catch(reportError)
              }}
            >
              {t('variation.reload')}
            </Button>
          </div>
        )}
        {!value ? (
          <p role="status">{t('fontInfo.loading')}</p>
        ) : (
          <>
            <fieldset disabled={disabled} className={styles.form}>
              <h3>{t('fontInfo.group.axes')}</h3>
              {value.axes.map((axis, index) => (
                <AxisEditor
                  key={index}
                  axis={axis}
                  onChange={(next) => updateAxis(index, next)}
                  onRemove={() => removeAxis(index)}
                />
              ))}
              <Button disabled={value.axes.length >= 16} onClick={addAxis}>
                {t('variation.addAxis')}
              </Button>
              <h3>{t('fontInfo.group.masterLocations')}</h3>
              <p>{t('variation.coordinatesHint')}</p>
              {value.masters.map((master, index) => (
                <fieldset className={styles.card} key={master.id}>
                  <legend>{master.name}</legend>
                  <div className={styles.row}>
                    <label>
                      {t('fontInfo.column.master')}
                      <input
                        value={master.name}
                        onChange={(e) =>
                          edit({
                            masters: value.masters.map((m, i) =>
                              i === index ? { ...m, name: e.target.value } : m,
                            ),
                          })
                        }
                      />
                    </label>
                    <Button
                      disabled={value.masters.length < 2}
                      onClick={() =>
                        edit({
                          masters: value.masters.filter((_, i) => i !== index),
                        })
                      }
                    >
                      {t('variation.removeMaster')}
                    </Button>
                  </div>
                  <div className={styles.row}>
                    {value.axes.map((axis) => (
                      <NumberField
                        key={axis.tag}
                        label={axis.tag}
                        value={master.location[axis.tag] ?? designDefault(axis)}
                        onCommit={(number) =>
                          edit({
                            masters: value.masters.map((m, i) =>
                              i === index
                                ? {
                                    ...m,
                                    location: {
                                      ...m.location,
                                      [axis.tag]: number,
                                    },
                                  }
                                : m,
                            ),
                          })
                        }
                      />
                    ))}
                  </div>
                  <MasterMetricsEditor
                    unitsPerEm={fontInfo?.info.units_per_em ?? 1000}
                    metrics={master.metrics}
                    onChange={(metrics) =>
                      edit({
                        masters: value.masters.map((m, i) =>
                          i === index ? { ...m, metrics } : m,
                        ),
                      })
                    }
                  />
                </fieldset>
              ))}
              <Button onClick={addMaster} disabled={!value.axes.length}>
                {t('variation.addMaster')}
              </Button>
              <p>{t('variation.copyHint')}</p>
              <VirtualMastersEditor
                state={value}
                onChange={(virtual_masters) => edit({ virtual_masters })}
              />
              <h3>{t('fontInfo.group.instances')}</h3>
              {value.instances.map((instance, index) => (
                <InstanceEditor
                  familyName={fontInfo?.info.family_name ?? ''}
                  key={index}
                  instance={instance}
                  axes={value.axes}
                  onChange={(next) =>
                    edit({
                      instances: value.instances.map((item, i) =>
                        i === index ? next : item,
                      ),
                    })
                  }
                  onRemove={() =>
                    edit({
                      instances: value.instances.filter((_, i) => i !== index),
                    })
                  }
                />
              ))}
              <Button disabled={!value.axes.length} onClick={addInstance}>
                {t('variation.addInstance')}
              </Button>
            </fieldset>
            {state.dirty ? (
              <p role="status">{t('variation.applyPreview')}</p>
            ) : (
              <VariationPreview
                doc={doc}
                state={value}
                onRefresh={() => {
                  void session.load().catch(reportError)
                }}
              />
            )}
          </>
        )}
      </div>
    </Dialog>
  )
}
