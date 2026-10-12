import type { TtHintDraft, TtHintState } from '../../api'

/** Imported programs with different point identities start as reference only. */
export function ttDraft(state: TtHintState): TtHintDraft {
  return {
    fingerprint: state.fingerprint,
    program:
      state.authored || state.binding_matches
        ? (state.program.assembly ?? '')
        : '',
    fpgm: state.fpgm.assembly ?? '',
    prep: state.prep.assembly ?? '',
    globals: {
      ...state.globals,
      max_zones: state.globals.max_zones ?? 2,
      max_twilight_points: state.globals.max_twilight_points ?? 0,
      max_storage: state.globals.max_storage ?? 0,
      max_function_defs: state.globals.max_function_defs ?? 0,
      max_instruction_defs: state.globals.max_instruction_defs ?? 0,
      max_stack_elements: state.globals.max_stack_elements ?? 256,
    },
  }
}

export function ttNeedsReview(state: TtHintState): boolean {
  return (
    (state.program.bytes.length > 0 && !state.binding_matches) ||
    !!state.program.error ||
    !!state.fpgm.error ||
    !!state.prep.error ||
    (state.globals.authored_units_per_em != null &&
      state.globals.authored_units_per_em !== state.units_per_em)
  )
}
