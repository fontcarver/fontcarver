import {
  call,
  type DocId,
  type FeatureGenerationSettings,
  type FeatureGenerator,
  type Features,
  type FeatureState,
  type FeatureValidationResult,
} from '../../api'
import { noteDocumentEdit } from '../../editor/editSession'
import { ApiCallError } from '../../transport'

/** State owned by one mounted document's source editor. */
export interface FeatureEditorSnapshot {
  state: FeatureState | null
  source: Features | null
  settings: FeatureGenerationSettings | null
  validation: FeatureValidationResult | null
  pending: boolean
  saving: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}

/** One draft owner; source changes are saved independently from activation. */
export class FeatureDraftSession {
  private snapshot: FeatureEditorSnapshot = {
    state: null,
    source: null,
    settings: null,
    validation: null,
    pending: false,
    saving: false,
    busy: false,
    suspended: false,
    error: null,
  }
  private listeners = new Set<() => void>()
  private sequence = Promise.resolve()
  private localRevision = 0
  private inputRevision = 0
  private master: string | null = null
  private composing = false
  private compositionWaiters: Array<() => void> = []

  constructor(readonly doc: DocId) {}

  getSnapshot = (): FeatureEditorSnapshot => this.snapshot
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }
  hasPendingEdits = (): boolean =>
    this.snapshot.pending || this.snapshot.saving || this.composing

  private publish(patch: Partial<FeatureEditorSnapshot>) {
    this.snapshot = { ...this.snapshot, ...patch }
    for (const listener of this.listeners) listener()
  }

  private enqueue<T>(run: () => Promise<T>): Promise<T> {
    const next = this.sequence.then(run)
    this.sequence = next.then(
      () => undefined,
      () => undefined,
    )
    return next
  }

  private accept(state: FeatureState, preserveLocal = false) {
    this.publish({
      state,
      ...(preserveLocal
        ? {
            pending:
              JSON.stringify([this.snapshot.source, this.snapshot.settings]) !==
              JSON.stringify([
                state.draft?.source ?? state.active_source,
                state.draft?.settings ?? state.active_settings,
              ]),
          }
        : {
            source: state.draft?.source ?? state.active_source,
            settings: state.draft?.settings ?? state.active_settings,
            pending: false,
          }),
      error: null,
    })
  }

  /** Invalidate immediately on master/glyph/history changes, before any read. */
  invalidate(master: string | null = this.master): void {
    this.master = master
    this.inputRevision += 1
    if (this.snapshot.validation) this.publish({ validation: null })
  }

  /** Refresh effective revision, retaining a local unsaved draft. */
  reload = (): Promise<void> =>
    this.enqueue(async () => {
      try {
        const state = await call('features.get', { doc: this.doc })
        const old = this.snapshot.state
        if (
          old &&
          (old.compile_revision !== state.compile_revision ||
            old.draft_revision !== state.draft_revision)
        )
          this.invalidate()
        if (
          this.snapshot.pending &&
          old &&
          old.draft_revision !== state.draft_revision
        )
          throw this.conflict()
        this.accept(state, this.snapshot.pending)
      } catch (error) {
        this.publish({ error })
        throw error
      }
    })

  private conflict() {
    return new ApiCallError('stale_features', 'Feature draft revision changed')
  }

  /** Ordinary typing only changes local source; document history owns undo. */
  change(source: Features, settings: FeatureGenerationSettings): void {
    if (
      this.snapshot.state?.authority === 'ImportedTables' ||
      this.snapshot.busy ||
      this.snapshot.suspended
    )
      return
    if (
      JSON.stringify([source, settings]) ===
      JSON.stringify([this.snapshot.source, this.snapshot.settings])
    )
      return
    this.localRevision += 1
    this.inputRevision += 1
    noteDocumentEdit(this.doc)
    const state = this.snapshot.state
    const saved = state
      ? [
          state.draft?.source ?? state.active_source,
          state.draft?.settings ?? state.active_settings,
        ]
      : null
    this.publish({
      source,
      settings,
      pending: JSON.stringify([source, settings]) !== JSON.stringify(saved),
      validation: null,
      error: null,
    })
  }

  setSuspended = (suspended: boolean): void => {
    this.publish({ suspended })
  }

  setComposing = (composing: boolean): void => {
    if (composing && !this.composing) noteDocumentEdit(this.doc)
    this.composing = composing
    if (!composing) {
      for (const finish of this.compositionWaiters.splice(0)) finish()
    }
  }

  private async finishComposition() {
    while (this.composing)
      await new Promise<void>((resolve) =>
        this.compositionWaiters.push(resolve),
      )
  }

  /** Persist even invalid text; errors reject the shell handover. */
  flush = async (): Promise<void> => {
    await this.finishComposition()
    return this.enqueue(async () => {
      await this.finishComposition()
      if (!this.snapshot.pending) return
      this.publish({ saving: true, error: null })
      try {
        while (this.snapshot.pending) {
          await this.finishComposition()
          const fresh = await call('features.get', { doc: this.doc })
          await this.finishComposition()
          if (
            this.snapshot.state &&
            fresh.draft_revision !== this.snapshot.state.draft_revision
          )
            throw this.conflict()
          const source = this.snapshot.source
          const settings = this.snapshot.settings
          if (!source || !settings) return
          const revision = this.localRevision
          const state = await call('features.set_draft', {
            doc: this.doc,
            expected_generation: fresh.generation,
            expected_draft_revision: fresh.draft_revision,
            source,
            settings,
            token: null,
          })
          this.accept(state, this.localRevision !== revision)
          await this.finishComposition()
        }
      } catch (error) {
        this.publish({ error })
        throw error
      } finally {
        this.publish({ saving: false })
      }
    })
  }

  private params(state: FeatureState) {
    return {
      doc: this.doc,
      expected_generation: state.generation,
      expected_compile_revision: state.compile_revision,
      expected_draft_revision: state.draft_revision,
    }
  }

  private validationMatches(
    result: FeatureValidationResult,
    fresh: FeatureState,
    revision: number,
  ) {
    return (
      revision === this.inputRevision &&
      result.doc === this.doc &&
      result.compile_revision === fresh.compile_revision &&
      result.draft_revision === fresh.draft_revision
    )
  }

  private async runExclusive(run: () => Promise<void>): Promise<void> {
    await this.flush()
    this.publish({ busy: true, error: null })
    try {
      await this.reload()
      await run()
    } catch (error) {
      this.publish({ error })
      throw error
    } finally {
      this.publish({ busy: false })
    }
  }

  /** Pure diagnostics are accepted only for the still-current compile inputs. */
  validate = (): Promise<void> =>
    this.runExclusive(async () => {
      const state = this.snapshot.state
      if (!state) return
      const revision = this.inputRevision
      const validation = await call('features.validate', this.params(state))
      const fresh = await call('features.get', { doc: this.doc })
      if (this.validationMatches(validation, fresh, revision))
        this.publish({ validation })
    })

  /** Activate through the kernel's all-master compile gate. */
  apply = (): Promise<void> =>
    this.runExclusive(async () => {
      const state = this.snapshot.state
      if (!state || state.authority !== 'Fea') return
      const master = this.master
      const result = await call('features.apply', this.params(state))
      const fresh = await call('features.get', { doc: this.doc })
      this.accept(fresh)
      this.publish({
        validation:
          master === this.master &&
          ((result.applied &&
            result.state.compile_revision === fresh.compile_revision &&
            result.state.draft_revision === fresh.draft_revision) ||
            (result.validation.compile_revision === fresh.compile_revision &&
              result.validation.draft_revision === fresh.draft_revision))
            ? result.validation
            : null,
      })
    })

  discard = (): Promise<void> =>
    this.runExclusive(async () => {
      const state = this.snapshot.state
      if (!state) return
      const result = await call('features.discard_draft', {
        doc: this.doc,
        expected_generation: state.generation,
        expected_draft_revision: state.draft_revision,
      })
      this.invalidate()
      this.accept(result)
    })

  /** Freeze one whole recipe, or with `entry` only that feature tag or class. */
  freeze = (generator: FeatureGenerator, entry?: string): Promise<void> =>
    this.runExclusive(async () => {
      const state = this.snapshot.state
      if (!state || state.authority !== 'Fea') return
      const result = await call('features.freeze', {
        ...this.params(state),
        generator,
        entry: entry ?? null,
      })
      this.invalidate()
      this.accept(result)
    })

  /** Explicit recovery abandons local text only after a saved snapshot arrives. */
  recover = (): Promise<void> =>
    this.enqueue(async () => {
      await this.finishComposition()
      const revision = this.localRevision
      const state = await call('features.get', { doc: this.doc })
      await this.finishComposition()
      if (revision !== this.localRevision) return
      this.localRevision += 1
      this.invalidate()
      this.accept(state)
    })
}
