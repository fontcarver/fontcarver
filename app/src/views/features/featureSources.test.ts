import { describe, expect, it } from 'vitest'
import type { Features } from '../../api'
import {
  addSource,
  moveSource,
  removeSource,
  sourceItems,
  updateSource,
} from './featureSources'

const empty: Features = {
  prefixes: [],
  classes: [],
  features: [],
  source_of_truth: 'Fea',
}

describe('feature source identities', () => {
  it('keeps stable IDs through duplicate tags, edits and reorder', () => {
    let source = addSource(empty, 'features', 'kern')
    source = addSource(source, 'features', 'kern')
    const [first, second] = source.features
    source = updateSource(source, first!.id, { code: '# 中文\r\npos A V -80;' })
    source = moveSource(source, first!.id, 1)
    expect(source.features.map((item) => item.id)).toEqual([
      second!.id,
      first!.id,
    ])
    expect(source.features[1]?.code).toBe('# 中文\r\npos A V -80;')
    expect(sourceItems(source).map((item) => item.kind)).toEqual([
      'features',
      'features',
    ])
  })
  it('removes one duplicate-tag block only and does not mutate the old snapshot', () => {
    const source = addSource(
      addSource(empty, 'features', 'kern'),
      'features',
      'kern',
    )
    const result = removeSource(source, source.features[0]!.id)
    expect(result.features).toEqual([source.features[1]])
    expect(source.features).toHaveLength(2)
    expect(empty.features).toHaveLength(0)
  })
  it('uses class name without changing its identity or other source blocks', () => {
    const source = addSource(empty, 'classes', 'Left')
    const id = source.classes[0]!.id
    const result = updateSource(source, id, { name: 'Right', code: 'A V' })
    expect(result.classes[0]).toMatchObject({ id, name: 'Right', code: 'A V' })
  })
})
