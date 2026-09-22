import { useTranslation } from 'react-i18next'

import type { MasterSummary } from '../../../api'
import { useEditorStore } from '../../../stores/editorStore'
import { selectActiveTab, useUiStore } from '../../../stores/uiStore'
import styles from '../glyphView.module.css'

import { AnchorsPanel } from './AnchorsPanel'
import { ComponentsPanel } from './ComponentsPanel'
import { GlyphPanel } from './GlyphPanel'
import { GuidesPanel } from './GuidesPanel'
import { KerningGroupsPanel } from './KerningGroupsPanel'
import { LayersPanel } from './LayersPanel'
import { UnihanPanel } from './UnihanPanel'
import { GlyphVariationPanel } from './GlyphVariationPanel'
import { MetricsPanel } from './MetricsPanel'
import { PanelSection } from './PanelSection'
import type { PanelContext } from './panelContext'
import { TransformPanel } from './TransformPanel'

export interface EditInspectorProps {
  masters: readonly MasterSummary[]
  hasAxes?: boolean
}

/**
 * The Edit tab's inspector: the seven sections of design spec §7, in the order
 * that spec gives, each remembering whether it is open and showing a count
 * where a count means something.
 *
 * Everything it renders comes from the one record the mounted glyph view
 * publishes, so the panels never issue a second request for data the canvas
 * already has.
 */
export function EditInspector({
  masters,
  hasAxes = false,
}: EditInspectorProps) {
  const { t } = useTranslation()
  const active = useEditorStore((state) => state.activeGlyph)
  const tab = useUiStore(selectActiveTab)

  if (!active?.outline)
    return (
      <p className={styles.inspectorEmpty}>
        {t(
          tab?.kind === 'glyph' && tab.textState?.mode === 'text'
            ? 'kerningGroups.selectGlyph'
            : 'glyphView.loading',
        )}
      </p>
    )
  const { outline } = active

  const context: PanelContext = {
    target: {
      doc: active.doc,
      glyph: active.glyph.name,
      layer: outline.layer,
    },
    gid: active.gid,
    glyph: active.glyph,
    cell: active.cell,
    scene: active.scene,
    outline,
    master: active.master,
    masterGuides: active.masterGuides,
    reload: active.reload,
  }

  return (
    <>
      <PanelSection id="glyph" title={t('glyphInspector.glyph')}>
        <GlyphPanel {...context} />
      </PanelSection>
      <UnihanPanel unicodes={active.glyph.unicodes} />
      {hasAxes && (
        <GlyphVariationPanel
          key={`${active.doc}:${active.gid}`}
          doc={active.doc}
          glyph={active.glyph.name}
        />
      )}
      <PanelSection id="metrics" title={t('glyphInspector.metrics')}>
        <MetricsPanel {...context} />
      </PanelSection>
      <PanelSection id="kerning-groups" title={t('kerningGroups.title')}>
        <KerningGroupsPanel target={context.target} />
      </PanelSection>
      <PanelSection
        id="layers"
        title={t('glyphInspector.layers')}
        count={active.glyph.layers.length}
      >
        <LayersPanel {...context} masters={masters} />
      </PanelSection>
      <PanelSection
        id="components"
        title={t('glyphInspector.components')}
        count={outline.components.length}
      >
        <ComponentsPanel {...context} />
      </PanelSection>
      <PanelSection
        id="anchors"
        title={t('glyphInspector.anchors')}
        count={outline.anchors.length}
      >
        <AnchorsPanel {...context} />
      </PanelSection>
      <PanelSection
        id="guides"
        title={t('panel.guides.title')}
        count={outline.guides.length + active.masterGuides.length}
      >
        <GuidesPanel {...context} />
      </PanelSection>
      <PanelSection
        id="transform"
        title={t('panel.transform.title')}
        defaultOpen={false}
      >
        <TransformPanel {...context} />
      </PanelSection>
    </>
  )
}
