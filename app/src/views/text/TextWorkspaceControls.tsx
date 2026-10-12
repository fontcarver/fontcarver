import { useEffect, useId, useRef, useState, type RefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { ChevronDown, ChevronRight } from 'lucide-react'
import type { DocSummary } from '../../api'
import { useUiStore, type TextTabState } from '../../stores/uiStore'
import { TextBar } from './TextBar'
import { ProofLibraryPanel } from './ProofLibraryPanel'
import { TextShapingControls } from './TextShapingControls'
import { VariableProofPanel } from './VariableProofPanel'
import { TextGapPicker } from './TextGapPicker'
import { TextKerningPanel } from './TextKerningPanel'
import { KerningGhostControls } from './KerningGhostControls'
import { KerningTable } from './KerningTable'
import { SpacingMatrix } from './SpacingMatrix'
import type { ProofEntry } from './proofLibrary'
import type { TextSampleId } from './presentationTypes'
import type { PositionedGlyph, TextRequestContext } from './textLayout'
import type { useTextKerningControls } from './useTextKerningControls'
import type { useKerningGhosts } from './useKerningGhosts'
import type { useSpacingOverlay } from './useSpacingOverlay'
import styles from './TextWorkspaceControls.module.css'

const KERNING_SECTION = 'text.kerning'

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
  spacing,
  onTextChange,
  onChange,
  onSample,
  onProofSelect,
  proofId,
  stepRef,
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
  spacing: ReturnType<typeof useSpacingOverlay>
  onTextChange: (text: string) => void
  onChange: (changes: Partial<TextTabState>) => void
  onSample: (id: TextSampleId) => void
  onProofSelect: (entry: ProofEntry) => void
  proofId: string | null
  stepRef: RefObject<((direction: -1 | 1) => void) | null>
}) {
  const { t } = useTranslation()
  const kerningId = useId()
  const preference = useUiStore((store) => store.sections[KERNING_SECTION])
  const [revealSelection, setRevealSelection] = useState(false)
  const previousGap = useRef(kerning.selectedGap)
  useEffect(() => {
    if (
      kerning.selectedGap !== null &&
      kerning.selectedGap !== previousGap.current
    )
      setRevealSelection(true)
    previousGap.current = kerning.selectedGap
  }, [kerning.selectedGap])
  const kerningOpen =
    revealSelection ||
    (preference ?? (state.mode === 'text' && kerning.gaps.length > 0))
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
      <div className={styles.secondaryControls}>
        <ProofLibraryPanel
          key={`library-${summary.doc}`}
          context={context}
          title={`${summary.family_name} ${summary.style_name}`}
          inputRef={inputRef}
          selectedId={proofId}
          stepRef={stepRef}
          onSelect={onProofSelect}
        />
        <TextShapingControls
          features={state.features}
          script={state.script ?? null}
          language={state.language ?? null}
          text={state.text}
          glyphs={glyphs}
          onChange={onChange}
        />
        <VariableProofPanel
          key={`variable-${summary.doc}`}
          summary={summary}
          state={state}
        />
        <section className={styles.kerning}>
          <button
            type="button"
            className={styles.sectionToggle}
            aria-expanded={kerningOpen}
            aria-controls={kerningId}
            onClick={() => {
              setRevealSelection(false)
              useUiStore.getState().setSection(KERNING_SECTION, !kerningOpen)
            }}
          >
            {kerningOpen ? (
              <ChevronDown size={14} aria-hidden="true" />
            ) : (
              <ChevronRight size={14} aria-hidden="true" />
            )}
            {t('kerningText.title')}
          </button>
          <div id={kerningId} hidden={!kerningOpen}>
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
              masters={summary.masters}
              generation={context.generation}
              vertical={state.direction === 'ttb'}
              disabled={state.kerningMode === 'off'}
            />
          </div>
        </section>
        <SpacingMatrix
          doc={summary.doc}
          master={state.master}
          unitsPerEm={summary.units_per_em}
          overlay={spacing}
        />
      </div>
    </>
  )
}
