import type { FeatureBlock, FeatureClass, Features } from '../../api'

/** Ordered editable source sections in the mapped assembler. */
export type SourceKind = 'prefixes' | 'classes' | 'features'
export type SourceItem = {
  kind: SourceKind
  id: string
  label: string
  code: string
  disabled: boolean
}

/** Flatten source blocks without treating duplicate tags as identity. */
export function sourceItems(source: Features): SourceItem[] {
  return [
    ...source.prefixes.map((block) => ({
      kind: 'prefixes' as const,
      id: block.id,
      label: block.tag,
      code: block.code,
      disabled: block.disabled,
    })),
    ...source.classes.map((block) => ({
      kind: 'classes' as const,
      id: block.id,
      label: block.name,
      code: block.code,
      disabled: false,
    })),
    ...source.features.map((block) => ({
      kind: 'features' as const,
      id: block.id,
      label: block.tag,
      code: block.code,
      disabled: block.disabled,
    })),
  ]
}

/** Add a source block using an independent UUID, never its tag or position. */
export function addSource(
  source: Features,
  kind: SourceKind,
  name: string,
): Features {
  const id = crypto.randomUUID().replaceAll('-', '')
  if (kind === 'classes')
    return {
      ...source,
      classes: [...source.classes, { id, name, code: '', automatic: false }],
    }
  const block: FeatureBlock = {
    id,
    tag: name,
    code: '',
    automatic: false,
    disabled: false,
    notes: '',
  }
  return { ...source, [kind]: [...source[kind], block] }
}

/** Update one stable source identity while leaving the previous snapshot intact. */
export function updateSource(
  source: Features,
  id: string,
  patch: Partial<FeatureBlock & FeatureClass>,
): Features {
  return {
    ...source,
    prefixes: source.prefixes.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    ),
    classes: source.classes.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    ),
    features: source.features.map((item) =>
      item.id === id ? { ...item, ...patch } : item,
    ),
  }
}

/** Remove exactly one source identity. */
export function removeSource(source: Features, id: string): Features {
  return {
    ...source,
    prefixes: source.prefixes.filter((item) => item.id !== id),
    classes: source.classes.filter((item) => item.id !== id),
    features: source.features.filter((item) => item.id !== id),
  }
}

/** Reorder only within a section, matching the assembly model's ordering. */
export function moveSource(
  source: Features,
  id: string,
  delta: number,
): Features {
  for (const kind of ['prefixes', 'classes', 'features'] as const) {
    const items = [...source[kind]]
    const index = items.findIndex((item) => item.id === id)
    const target = index + delta
    if (index < 0 || target < 0 || target >= items.length) continue
    const [item] = items.splice(index, 1)
    items.splice(target, 0, item!)
    return { ...source, [kind]: items }
  }
  return source
}
