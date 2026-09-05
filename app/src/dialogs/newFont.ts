import { call, type DocSummary, type MasterMetrics } from '../api'
import { i18n } from '../i18n'
import { useSessionStore } from '../stores/sessionStore'

/** Metrics preset offered by the New Font dialog. */
export type MetricsPreset = 'latin' | 'cjk'

/** Starter repertoire offered by the New Font dialog. */
export type StarterSet = 'none' | 'basicLatin' | 'latin1' | 'gb2312'

/** Units per em the CJK preset is designed for. */
export const CJK_UNITS_PER_EM = 1000

/** Units per em offered as presets before the custom field. */
export const UNITS_PER_EM_PRESETS = [1000, 2048] as const

/**
 * Recipes handed to `glyphs.add`. The kernel owns the recipe grammar, so a
 * starter set is a recipe string rather than a second list of glyph names.
 */
export const STARTER_RECIPES: Record<StarterSet, string> = {
  none: '',
  basicLatin:
    'A-Z a-z 0-9 U+0020-U+002F U+003A-U+0040 U+005B-U+0060 U+007B-U+007E',
  latin1: 'U+0020-U+00FF',
  gb2312: '@charset:GB2312',
}

/** Everything the New Font dialog collects. */
export interface NewFontForm {
  readonly family: string
  readonly style: string
  readonly unitsPerEm: number
  readonly preset: MetricsPreset
  readonly starter: StarterSet
}

/** Vertical metrics of a preset, scaled to the chosen units per em. */
export function metricsFor(
  preset: MetricsPreset,
  unitsPerEm: number,
): MasterMetrics {
  const scale = (fraction: number) => Math.round(fraction * unitsPerEm)
  const [ascender, descender] =
    preset === 'cjk' ? [880, -120] : [scale(0.8), -scale(0.2)]
  return {
    ascender,
    descender,
    x_height: scale(0.5),
    cap_height: scale(0.7),
    italic_angle: 0,
    // Every override stays unset: the compiler derives hhea, OS/2 and vhea
    // values from the four metrics above unless a designer overrides them.
    hhea_ascender: null,
    hhea_descender: null,
    hhea_line_gap: null,
    typo_ascender: null,
    typo_descender: null,
    typo_line_gap: null,
    win_ascent: null,
    win_descent: null,
    vhea_ascender: null,
    vhea_descender: null,
    vhea_line_gap: null,
    default_vertical_origin: null,
    default_vertical_advance: null,
  }
}

/**
 * Creates the document the New Font dialog describes and opens it in a tab.
 *
 * Three kernel calls, in order: `session.new` mints the document, one
 * `doc.apply` batch writes the style name and the metrics preset as a single
 * undo step, and `glyphs.add` fills the starter repertoire from its recipe.
 */
export async function createFont(
  form: NewFontForm,
): Promise<DocSummary | null> {
  const session = useSessionStore.getState()
  const summary = await session.createDocument(form.family, form.unitsPerEm)
  if (!summary) return null
  const doc = summary.doc

  const view = await call('doc.font_info', { doc })
  const master = view.masters[0]
  if (master)
    await call('doc.apply', {
      doc,
      command: {
        Batch: [
          {
            SetFontInfo: {
              info: {
                ...view.info,
                family_name: form.family,
                style_name: form.style,
                units_per_em: form.unitsPerEm,
              },
            },
          },
          {
            SetMasterMetrics: {
              master: master.id,
              metrics: {
                ...master.metrics,
                ...metricsFor(form.preset, form.unitsPerEm),
              },
            },
          },
        ],
      },
      label: i18n.t('undo.newFont'),
    })

  const recipe = STARTER_RECIPES[form.starter]
  if (recipe)
    await call('glyphs.add', { doc, recipe, master: null, dry_run: false })

  await useSessionStore.getState().refreshSummary(doc)
  return summary
}
