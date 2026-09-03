import { useTranslation } from 'react-i18next'

import styles from './glyphView.module.css'
import { formatUnicodes, pointCount, type GlyphScene } from './glyphScene'

export interface GlyphHudProps {
  scene: GlyphScene
}

function metric(value: number | null): string {
  return value === null ? '—' : String(Math.round(value))
}

/**
 * The corner readout: the identity and horizontal metrics of the glyph on
 * screen. Glyphs and FontLab both keep this permanently visible, and it is the
 * fastest way to check spacing without opening the inspector.
 */
export function GlyphHud({ scene }: GlyphHudProps) {
  const { t } = useTranslation()
  const { metrics } = scene
  const entries: [string, string][] = [
    [t('glyphView.hud.unicode'), formatUnicodes(scene.unicodes)],
    [t('glyphView.hud.width'), String(Math.round(metrics.width))],
    [t('glyphView.hud.lsb'), metric(metrics.lsb)],
    [t('glyphView.hud.rsb'), metric(metrics.rsb)],
    [t('glyphView.hud.points'), String(pointCount(scene))],
  ]

  return (
    <div className={styles.hud} aria-label={t('glyphView.hud.label')}>
      <div className={styles.hudName}>{scene.name}</div>
      <dl className={styles.hudList}>
        {entries.map(([label, value]) => (
          <div key={label} className={styles.hudRow}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
