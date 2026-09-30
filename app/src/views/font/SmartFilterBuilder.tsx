import { useTranslation } from 'react-i18next'

import type { SmartFilterNode, SmartFilterOperator } from '../../api'
import { Button } from '../../components/Button'

import {
  defaultSmartRule,
  SMART_FILTER_FIELDS,
  smartRuleOperators,
} from './smartFilterBuilder'
import styles from './smartFilters.module.css'

export interface SmartFilterBuilderProps {
  root: SmartFilterNode
  onChange: (root: SmartFilterNode) => void
  disabled?: boolean
  depth?: number
  onRemove?: () => void
}

/** A recursive, keyboard-accessible editor for nested All, Any and None rules. */
export function SmartFilterBuilder({
  root,
  onChange,
  disabled = false,
  depth = 0,
  onRemove,
}: SmartFilterBuilderProps) {
  const { t } = useTranslation()
  const removeButton = onRemove && (
    <Button disabled={disabled} onClick={onRemove}>
      {t(
        root.kind === 'predicate'
          ? 'smartFilters.removeRule'
          : 'smartFilters.removeGroup',
      )}
    </Button>
  )

  if (root.kind === 'predicate') {
    const boolean =
      SMART_FILTER_FIELDS.find(({ field }) => field === root.field)?.type ===
      'boolean'
    return (
      <div className={styles.rule}>
        <select
          aria-label={t('smartFilters.field')}
          className={styles.control}
          value={root.field}
          disabled={disabled}
          onChange={(event) => onChange(defaultSmartRule(event.target.value))}
        >
          {SMART_FILTER_FIELDS.map(({ field }) => (
            <option key={field} value={field}>
              {t(`smartFilters.fields.${field}`)}
            </option>
          ))}
        </select>
        <select
          aria-label={t('smartFilters.operator')}
          className={styles.control}
          value={root.operator}
          disabled={disabled}
          onChange={(event) =>
            onChange({
              ...root,
              operator: event.target.value as SmartFilterOperator,
            })
          }
        >
          {smartRuleOperators(root.field).map((operator) => (
            <option key={operator} value={operator}>
              {t(`smartFilters.operators.${operator}`)}
            </option>
          ))}
        </select>
        {boolean ? (
          <select
            aria-label={t('smartFilters.value')}
            className={styles.control}
            value={root.value}
            disabled={disabled}
            onChange={(event) =>
              onChange({ ...root, value: event.target.value })
            }
          >
            <option value="true">{t('smartFilters.yes')}</option>
            <option value="false">{t('smartFilters.no')}</option>
          </select>
        ) : (
          <input
            aria-label={t('smartFilters.value')}
            className={styles.control}
            value={root.value}
            disabled={disabled}
            onChange={(event) =>
              onChange({ ...root, value: event.target.value })
            }
          />
        )}
        {removeButton}
      </div>
    )
  }

  return (
    <fieldset className={styles.group} disabled={disabled}>
      <legend>{t('smartFilters.group')}</legend>
      <div className={styles.groupActions}>
        <select
          aria-label={t('smartFilters.match')}
          className={styles.control}
          value={root.kind}
          onChange={(event) =>
            onChange({
              ...root,
              kind: event.target.value as 'all' | 'any' | 'none',
            })
          }
        >
          {(['all', 'any', 'none'] as const).map((kind) => (
            <option key={kind} value={kind}>
              {t(`smartFilters.groups.${kind}`)}
            </option>
          ))}
        </select>
        <Button
          onClick={() =>
            onChange({
              ...root,
              children: [...root.children, defaultSmartRule()],
            })
          }
        >
          {t('smartFilters.addRule')}
        </Button>
        <Button
          disabled={depth >= 10}
          onClick={() =>
            onChange({
              ...root,
              children: [
                ...root.children,
                { kind: 'any', children: [defaultSmartRule()] },
              ],
            })
          }
        >
          {t('smartFilters.addGroup')}
        </Button>
        {removeButton}
      </div>
      <div className={styles.children}>
        {root.children.map((child, index) => (
          <SmartFilterBuilder
            key={index}
            root={child}
            depth={depth + 1}
            onChange={(next) =>
              onChange({
                ...root,
                children: root.children.map((entry, position) =>
                  position === index ? next : entry,
                ),
              })
            }
            onRemove={
              root.children.length > 1
                ? () =>
                    onChange({
                      ...root,
                      children: root.children.filter(
                        (_, position) => position !== index,
                      ),
                    })
                : undefined
            }
            disabled={disabled}
          />
        ))}
      </div>
    </fieldset>
  )
}
