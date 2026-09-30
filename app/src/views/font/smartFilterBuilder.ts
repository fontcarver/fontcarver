import type { SmartFilterNode, SmartFilterOperator } from '../../api'

/** Editable predicate vocabulary shared by the visual builder and its labels. */
export const SMART_FILTER_FIELDS = [
  { field: 'name', type: 'text' },
  { field: 'paths', type: 'number' },
  { field: 'components', type: 'number' },
  { field: 'tags', type: 'text' },
  { field: 'script', type: 'text' },
  { field: 'category', type: 'text' },
  { field: 'subcategory', type: 'text' },
  { field: 'case', type: 'text' },
  { field: 'master_compatible', type: 'boolean' },
  { field: 'export', type: 'boolean' },
  { field: 'has_unicode', type: 'boolean' },
  { field: 'has_components', type: 'boolean' },
  { field: 'contains_component', type: 'text' },
  { field: 'has_hints', type: 'boolean' },
  { field: 'has_ps_hints', type: 'boolean' },
  { field: 'has_tt_hints', type: 'boolean' },
  { field: 'has_corners', type: 'boolean' },
  { field: 'has_anchors', type: 'boolean' },
  { field: 'has_special_layers', type: 'boolean' },
  { field: 'has_custom_glyph_info', type: 'boolean' },
  { field: 'has_annotations', type: 'boolean' },
  { field: 'is_auto_aligned', type: 'boolean' },
  { field: 'has_metrics_keys', type: 'boolean' },
  { field: 'metrics_out_of_sync', type: 'boolean' },
  { field: 'has_kerning_groups', type: 'boolean' },
  { field: 'color_label', type: 'number' },
  { field: 'layer_color_label', type: 'number' },
  { field: 'is_hangul_key', type: 'boolean' },
  { field: 'note', type: 'text' },
  { field: 'unicode', type: 'scalar' },
  { field: 'changed', type: 'boolean' },
  { field: 'width', type: 'number' },
  { field: 'block', type: 'text' },
] as const

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
