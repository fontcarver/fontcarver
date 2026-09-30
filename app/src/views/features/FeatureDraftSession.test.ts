import { beforeEach, describe, expect, it, vi } from 'vitest'
import { FeatureDraftSession } from './FeatureDraftSession'
import { featureSettings } from './featureTestFixtures'
import {
  call,
  type FeatureState,
  type FeatureValidationResult,
} from '../../api'
vi.mock('../../api', () => ({ call: vi.fn() }))
const settings = featureSettings()
function initial(): FeatureState {
  return {
    doc: 1,
    generation: 0,
    compile_revision: 0,
    draft_revision: 0,
    authority: 'Fea',
    active_source: {
      prefixes: [
        {
          id: 'p1',
          tag: 'Source',
          code: '# hello',
          automatic: false,
          disabled: false,
          notes: '',
        },
      ],
      classes: [],
      features: [],
      source_of_truth: 'Fea',
    },
    active_settings: settings,
    draft: null,
    has_unapplied_changes: false,
    generation_stale: false,
    original_layout_available: false,
    automatic_entries: [],
  }
}
const deferred = <T>() => {
  let resolve!: (v: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
let state: FeatureState
beforeEach(() => {
  state = initial()
  vi.mocked(call).mockReset()
  vi.mocked(call).mockImplementation((async (method, params) => {
    if (method === 'features.get') return structuredClone(state)
    if (method === 'features.set_draft') {
      const p = params as Parameters<typeof call<'features.set_draft'>>[1]
      state = {
        ...state,
        generation: state.generation + 1,
        draft_revision: state.draft_revision + 1,
        has_unapplied_changes: true,
        draft: {
          source: p.source,
          settings: p.settings,
          revision: state.draft_revision + 1,
        },
      }
      return structuredClone(state)
    }
    throw new Error(`unexpected ${method}`)
  }) as typeof call)
})
function edit(session: FeatureDraftSession, code: string) {
  const source = session.getSnapshot().source!
  session.change(
    { ...source, prefixes: source.prefixes.map((b) => ({ ...b, code })) },
    session.getSnapshot().settings!,
  )
}

describe('FeatureDraftSession', () => {
  it('opening and flushing an untouched source stays clean', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    await session.flush()
    expect(
      vi.mocked(call).mock.calls.every(([method]) => method === 'features.get'),
    ).toBe(true)
    expect(session.hasPendingEdits()).toBe(false)
  })
  it('coalesces typing including Unicode into one draft mutation', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, '# 中文')
    edit(session, '# 中文\r\ninvalid {')
    await session.flush()
    expect(
      vi.mocked(call).mock.calls.filter(([m]) => m === 'features.set_draft'),
    ).toHaveLength(1)
    expect(state.draft?.source.prefixes[0]?.code).toBe('# 中文\r\ninvalid {')
    expect(session.hasPendingEdits()).toBe(false)
  })
  it('waits for composition before persistence and retains the composed text', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    session.setComposing(true)
    edit(session, '# 中')
    const saving = session.flush()
    await Promise.resolve()
    expect(state.draft).toBeNull()
    edit(session, '# 中文')
    session.setComposing(false)
    await saving
    expect(state.draft?.source.prefixes[0]?.code).toBe('# 中文')
  })
  it('keeps unsaved text and rejects the navigation flush on failed persistence', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'invalid 中文')
    vi.mocked(call)
      .mockImplementationOnce(async () => state)
      .mockRejectedValueOnce(new Error('disk unavailable'))
    await expect(session.flush()).rejects.toThrow('disk unavailable')
    expect(session.getSnapshot().source?.prefixes[0]?.code).toBe('invalid 中文')
    expect(session.hasPendingEdits()).toBe(true)
  })
  it('does not overwrite a draft changed by document undo or another editor', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'local')
    state = { ...state, draft_revision: 9 }
    await expect(session.flush()).rejects.toMatchObject({
      code: 'stale_features',
    })
    expect(session.getSnapshot().source?.prefixes[0]?.code).toBe('local')
    expect(
      vi.mocked(call).mock.calls.filter(([m]) => m === 'features.set_draft'),
    ).toHaveLength(0)
  })
  it('ignores diagnostics when compile input changes without a draft edit', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    const pending = deferred<FeatureValidationResult>()
    const fallback = vi.mocked(call).getMockImplementation()!
    vi.mocked(call).mockImplementation(((
      method: Parameters<typeof call>[0],
      params: never,
    ) =>
      method === 'features.validate'
        ? pending.promise
        : fallback(method, params)) as typeof call)
    const run = session.validate()
    await vi.waitFor(() =>
      expect(
        vi.mocked(call).mock.calls.some(([m]) => m === 'features.validate'),
      ).toBe(true),
    )
    state = { ...state, generation: 1, compile_revision: 1 }
    session.invalidate('master2')
    pending.resolve({
      doc: 1,
      generation: 0,
      compile_revision: 0,
      draft_revision: 0,
      valid: false,
      diagnostics: [],
      truncated: false,
      compiled_masters: [],
      generated_source: state.active_source,
      generated_settings: settings,
    })
    await run
    expect(session.getSnapshot().validation).toBeNull()
  })
  it('persists a newer edit back to the original text after an older save finishes', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    const original = session.getSnapshot().source!.prefixes[0]!.code
    edit(session, 'older saved value')
    const pending = deferred<FeatureState>()
    const fallback = vi.mocked(call).getMockImplementation()!
    let first = true
    vi.mocked(call).mockImplementation(((
      method: Parameters<typeof call>[0],
      params: never,
    ) => {
      if (method === 'features.set_draft' && first) {
        first = false
        return fallback(method, params).then((value) => {
          state = value as FeatureState
          return pending.promise
        })
      }
      return fallback(method, params)
    }) as typeof call)
    const saving = session.flush()
    await vi.waitFor(() => expect(first).toBe(false))
    edit(session, original)
    pending.resolve(structuredClone(state))
    await saving
    expect(state.draft?.source.prefixes[0]?.code).toBe(original)
    expect(session.hasPendingEdits()).toBe(false)
    expect(
      vi.mocked(call).mock.calls.filter(([m]) => m === 'features.set_draft'),
    ).toHaveLength(2)
  })

  it('does not replace newer text with a delayed recovery response', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'original local')
    const pending = deferred<FeatureState>()
    vi.mocked(call).mockImplementationOnce(() => pending.promise as never)
    const recovering = session.recover()
    await Promise.resolve()
    await Promise.resolve()
    edit(session, 'newer local')
    pending.resolve(structuredClone(state))
    await recovering
    expect(session.getSnapshot().source?.prefixes[0]?.code).toBe('newer local')
    expect(session.hasPendingEdits()).toBe(true)
  })

  it('retains local source when recovery fails', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'local source')
    vi.mocked(call).mockRejectedValueOnce(new Error('offline'))
    await expect(session.recover()).rejects.toThrow('offline')
    expect(session.getSnapshot().source?.prefixes[0]?.code).toBe('local source')
  })
  it('waits for IME that starts while the saved revision query is delayed', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'before composition')
    const pending = deferred<FeatureState>()
    vi.mocked(call).mockImplementationOnce(() => pending.promise as never)
    const saving = session.flush()
    await vi.waitFor(() =>
      expect(
        vi.mocked(call).mock.calls.filter(([m]) => m === 'features.get'),
      ).toHaveLength(2),
    )
    session.setComposing(true)
    edit(session, '# 中')
    pending.resolve(structuredClone(state))
    await Promise.resolve()
    await Promise.resolve()
    expect(state.draft).toBeNull()
    edit(session, '# 中文')
    session.setComposing(false)
    await saving
    expect(state.draft?.source.prefixes[0]?.code).toBe('# 中文')
  })
  it('keeps successful apply diagnostics associated with the committed state', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    const warning = {
      severity: 'Warning' as const,
      code: 'writer-warning',
      message: 'Retained warning',
      source_id: null,
      span: null,
      generated: false,
      master: 'm1',
    }
    const validation: FeatureValidationResult = {
      doc: 1,
      generation: 0,
      compile_revision: 0,
      draft_revision: 0,
      valid: true,
      diagnostics: [warning],
      truncated: false,
      compiled_masters: ['m1'],
      generated_source: state.active_source,
      generated_settings: settings,
    }
    const fallback = vi.mocked(call).getMockImplementation()!
    vi.mocked(call).mockImplementation(((
      method: Parameters<typeof call>[0],
      params: never,
    ) => {
      if (method === 'features.apply') {
        state = {
          ...state,
          generation: 1,
          compile_revision: 1,
          draft_revision: 1,
        }
        return Promise.resolve({
          applied: true,
          state: structuredClone(state),
          validation,
        })
      }
      return fallback(method, params)
    }) as typeof call)
    await session.apply()
    expect(session.getSnapshot().validation?.diagnostics).toEqual([warning])
    expect(session.getSnapshot().state?.compile_revision).toBe(1)
  })

  it('retains saved invalid draft and previous active source after failed apply', async () => {
    const session = new FeatureDraftSession(1)
    await session.reload()
    edit(session, 'invalid syntax')
    await session.flush()
    const fallback = vi.mocked(call).getMockImplementation()!
    vi.mocked(call).mockImplementation(((
      method: Parameters<typeof call>[0],
      params: never,
    ) => {
      if (method === 'features.apply')
        return Promise.resolve({
          applied: false,
          state: structuredClone(state),
          validation: {
            doc: 1,
            generation: state.generation,
            compile_revision: state.compile_revision,
            draft_revision: state.draft_revision,
            valid: false,
            diagnostics: [
              {
                severity: 'Error',
                code: 'parse',
                message: 'Invalid syntax',
                source_id: 'p1',
                span: { start: 0, end: 7 },
                generated: false,
                master: 'm1',
              },
            ],
            truncated: false,
            compiled_masters: [],
            generated_source: state.draft!.source,
            generated_settings: settings,
          },
        })
      return fallback(method, params)
    }) as typeof call)
    await session.apply()
    expect(session.getSnapshot().source?.prefixes[0]?.code).toBe(
      'invalid syntax',
    )
    expect(state.active_source.prefixes[0]?.code).toBe('# hello')
    expect(session.getSnapshot().validation?.valid).toBe(false)
  })
})
