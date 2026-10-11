import type { SmartFilterNode, SmartFilterOperator } from '../../api'
import { SMART_FILTER_FIELDS } from '../../api/generated/smartFilterFields'

/**
 * The predicate vocabulary, generated from the kernel's own list.
 *
 * The parser, the evaluator and this builder read one table, so a field cannot
 * exist on one side of the boundary and not the other.
 */
export { SMART_FILTER_FIELDS } from '../../api/generated/smartFilterFields'

/** Valid operators for a field; text comparisons include bounded regex. */
export function smartRuleOperators(field: string): SmartFilterOperator[] {
  const type = SMART_FILTER_FIELDS.find((item) => item.field === field)?.type
  if (type === 'boolean') return ['eq', 'ne']
  if (type === 'number' || type === 'scalar')
    return ['eq', 'ne', 'gt', 'gte', 'lt', 'lte']
  return ['eq', 'ne', 'contains', 'regex']
}

/** Creates a useful initial predicate without an empty boolean or number. */
export function defaultSmartRule(field = 'name'): SmartFilterNode {
  const type = SMART_FILTER_FIELDS.find((item) => item.field === field)?.type
  if (type === 'scalar')
    return { kind: 'predicate', field, operator: 'eq', value: 'U+0041' }
  return {
    kind: 'predicate',
    field,
    operator: type === 'number' ? 'gt' : type === 'boolean' ? 'eq' : 'contains',
    value: type === 'number' ? '0' : type === 'boolean' ? 'true' : '',
  }
}

const SYMBOLS: Record<SmartFilterOperator, string> = {
  eq: ':',
  ne: '!=',
  contains: '~',
  regex: '=~',
  gt: '>',
  gte: '>=',
  lt: '<',
  lte: '<=',
}

/** Serializes a builder tree to the kernel DSL with lossless quoted values. */
export function formatSmartFilter(node: SmartFilterNode): string {
  if (node.kind === 'predicate')
    return `${node.field}${SYMBOLS[node.operator]}${JSON.stringify(node.value)}`
  const separator = node.kind === 'all' ? ' and ' : ' or '
  const group = `(${node.children.map(formatSmartFilter).join(separator)})`
  return node.kind === 'none' ? `not ${group}` : group
}
