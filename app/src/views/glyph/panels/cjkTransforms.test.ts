import { describe, expect, it } from 'vitest'

import type { ComponentView } from '../../../api'
import { applyAffine, type Affine } from '../../../editor/affine'

import {
  applyDraftsCommand,
  componentsSignature,
  draftProblem,
  draftTransform,
  editDraft,
  hasDrafts,
  previewMatrix,
  TRANSFORM_FIELDS,
} from './cjkTransforms'

const target = { doc: 1, glyph: 'uni6797', layer: 'm1' }

function view(transform: Affine, base = 'uni6728'): ComponentView {
  return {
    component: {
      base,
      transform,
      anchor: null,
      auto_align: false,
      use_my_metrics: false,
      round_to_grid: false,
      smart: null,
      point_match: null,
    },
    path: 'M0 0L100 0L100 100Z',
    bbox: null,
  }
}

const components = [view([0.5, 0, 0, 1, 0, 0]), view([0.5, 0, 0, 1, 500, 0])]
const fieldOf = (key: string) =>
  TRANSFORM_FIELDS.find((field) => field.key === key)!

describe('staged component transforms', () => {
  it('reads a stored transform until a draft replaces it', () => {
    expect(draftTransform({}, components, 1)).toEqual([0.5, 0, 0, 1, 500, 0])
    expect(draftTransform({ 1: [1, 0, 0, 1, 2, 3] }, components, 1)).toEqual([
      1, 0, 0, 1, 2, 3,
    ])
    // An index with no component falls back to the identity rather than crashing.
    expect(draftTransform({}, components, 9)).toEqual([1, 0, 0, 1, 0, 0])
  })

  it('stages one coefficient and forgets a value typed back', () => {
    const staged = editDraft({}, components, 0, fieldOf('x'), 120)
    expect(staged).toEqual({ 0: [0.5, 0, 0, 1, 120, 0] })
    expect(hasDrafts(staged)).toBe(true)
    const scaled = editDraft(staged, components, 0, fieldOf('a'), 0.75)
    expect(scaled[0]).toEqual([0.75, 0, 0, 1, 120, 0])
    // Returning every coefficient to its stored value drops the draft entirely.
    const back = editDraft(
      editDraft(scaled, components, 0, fieldOf('a'), 0.5),
      components,
      0,
      fieldOf('x'),
      0,
    )
    expect(hasDrafts(back)).toBe(false)
    // An index with no component cannot be staged.
    expect(editDraft({}, components, 7, fieldOf('x'), 1)).toEqual({})
  })

  it('previews a draft against a non-identity stored transform', () => {
    // `ComponentView.path` already carries the stored transform, so the preview
    // matrix must undo it: a point the stored transform put at x = 250 must land
    // where the draft would put the original point.
    const stored: Affine = [0.5, 0, 0, 1, 500, 0]
    const draft: Affine = [0.25, 0, 0, 0.5, 100, 40]
    const matrix = previewMatrix(stored, draft)
    const source = { x: 120, y: 80 }
    expect(applyAffine(matrix, applyAffine(stored, source))).toEqual(
      applyAffine(draft, source),
    )
    // A stored transform that cannot be undone leaves the path where it is.
    expect(previewMatrix([0, 0, 0, 0, 0, 0], draft)).toEqual([1, 0, 0, 1, 0, 0])
  })

  it('names why a staged set cannot be applied', () => {
    expect(draftProblem({})).toBe('empty')
    expect(draftProblem({ 0: [1, 0, 0, 1, 0, 0] })).toBeNull()
    expect(draftProblem({ 0: [1, 0, 0, Number.NaN, 0, 0] })).toBe(
      'transformInvalid',
    )
    expect(draftProblem({ 0: [0, 0, 0, 1, 0, 0] })).toBe('transformCollapsed')
  })

  it('writes every staged component in index order as one batch', () => {
    const drafts = {
      1: [0.5, 0, 0, 1, 520, 0] as Affine,
      0: [0.5, 0, 0, 1, 10, 0] as Affine,
    }
    expect(applyDraftsCommand(target, components, drafts)).toEqual({
      Batch: [
        {
          SetComponent: {
            glyph: 'uni6797',
            layer: 'm1',
            index: 0,
            component: { ...components[0]!.component, transform: drafts[0] },
          },
        },
        {
          SetComponent: {
            glyph: 'uni6797',
            layer: 'm1',
            index: 1,
            component: { ...components[1]!.component, transform: drafts[1] },
          },
        },
      ],
    })
    // A draft whose component vanished is dropped instead of being written to
    // whatever now occupies that index.
    expect(applyDraftsCommand(target, [components[0]!], drafts)).toMatchObject({
      Batch: [{ SetComponent: { index: 0 } }],
    })
  })

  it('changes its signature whenever a draft could be misaimed', () => {
    const base = componentsSignature(target, components)
    expect(componentsSignature(target, components)).toBe(base)
    expect(componentsSignature(target, [components[0]!])).not.toBe(base)
    expect(
      componentsSignature(target, [components[0]!, view([1, 0, 0, 1, 0, 0])]),
    ).not.toBe(base)
    expect(
      componentsSignature({ ...target, layer: 'm2' }, components),
    ).not.toBe(base)
  })
})
