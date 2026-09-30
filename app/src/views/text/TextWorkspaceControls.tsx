import type { RefObject } from 'react'
import type { DocSummary } from '../../api'
import type { TextTabState } from '../../stores/uiStore'
import { TextBar } from './TextBar'
import { ProofLibraryPanel } from './ProofLibraryPanel'
import { TextShapingControls } from './TextShapingControls'
import { VariableProofPanel } from './VariableProofPanel'
import { TextGapPicker } from './TextGapPicker'
import { TextKerningPanel } from './TextKerningPanel'
import { KerningGhostControls } from './KerningGhostControls'
import { KerningTable } from './KerningTable'
import type { ProofEntry } from './proofLibrary'
import type { TextSampleId } from './presentationTypes'
import type { PositionedGlyph, TextRequestContext } from './textLayout'
import type { useTextKerningControls } from './useTextKerningControls'
import type { useKerningGhosts } from './useKerningGhosts'

/** Presentation controls share the workspace's one transition barrier. */
export function TextWorkspaceControls({
  summary,
  state,
  context,
  draft,
  inputRef,
  glyphs,
  visible,
  pending,
  kerning,
  ghosts,
  onTextChange,
  onChange,
  onSample,
  onProofSelect,
}: {
  summary: DocSummary
  state: TextTabState
  context: TextRequestContext
  draft: string
  inputRef: RefObject<HTMLInputElement | null>
  glyphs: readonly PositionedGlyph[]
  visible: readonly number[]
  pending: boolean
  kerning: ReturnType<typeof useTextKerningControls>
  ghosts: ReturnType<typeof useKerningGhosts>
  onTextChange: (text: string) => void
  onChange: (changes: Partial<TextTabState>) => void
  onSample: (id: TextSampleId) => void
  onProofSelect: (entry: ProofEntry) => void
}) {
  return (
    <>
      <TextBar
        text={draft}
        inputRef={inputRef}
        onTextChange={onTextChange}
        size={state.size}
        onSizeChange={(size) => onChange({ size })}
        master={state.master ?? ''}
        masters={summary.masters}
        onMasterChange={(master) => onChange({ master })}
        features={state.features}
        onFeaturesChange={(features) => onChange({ features })}
        direction={state.direction}
        onDirectionChange={(direction) => onChange({ direction })}
        onSample={onSample}
        kerningMode={state.kerningMode ?? 'on'}
        onKerningModeChange={(kerningMode) => onChange({ kerningMode })}
        spacingSide={state.spacingSide ?? 'lsb'}
        onSpacingSideChange={(spacingSide) => onChange({ spacingSide })}
      />
      <ProofLibraryPanel
        key={`library-${summary.doc}`}
        context={context}
        title={`${summary.family_name} ${summary.style_name}`}
        inputRef={inputRef}
        onSelect={onProofSelect}
      />
      <TextShapingControls
        features={state.features}
        script={state.script ?? null}
        language={state.language ?? null}
        glyphs={glyphs}
        onChange={onChange}
      />
      <VariableProofPanel
        key={`variable-${summary.doc}`}
        summary={summary}
        state={state}
      />
      <TextGapPicker
        gaps={kerning.gaps}
        selected={kerning.selectedGap}
        onSelect={kerning.selectGap}
        pending={pending || state.kerningMode === 'off'}
        visible={visible}
      />
      <TextKerningPanel
        model={kerning.panel}
        gapDisabledReason={kerning.disabledReason}
        pending={pending}
      />
      <KerningGhostControls model={ghosts} />
      <KerningTable
        doc={summary.doc}
        master={state.master}
        generation={context.generation}
        vertical={state.direction === 'ttb'}
        disabled={state.kerningMode === 'off'}
      />
    </>
  )
}
