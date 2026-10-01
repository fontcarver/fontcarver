/**
 * The built-in grouped, script-aware proof samples.
 *
 * These are the starting library every project can adopt: pangrams and phrases
 * for reading, and pairs and control words for spacing. Inserting them writes
 * ordinary project entries through the undoable proof-library command, so they
 * are then filtered, stepped, exported and saved exactly like a hand-written
 * string. The identifiers are deterministic, so adopting them twice adds
 * nothing the second time and a renamed or edited copy is never replaced.
 */

import type { ProofEntry } from './proofLibrary'

/** One built-in sample before its name and group are translated. */
export interface BuiltinProofSample {
  id: string
  /** `proof.samples.names.*` key. */
  nameKey: string
  /** `proof.samples.groups.*` key. */
  groupKey: string
  /** ISO 15924 script, which also drives the library's script filter. */
  script: string
  /** BCP 47 language, or an empty string for the shaper default. */
  language: string
  direction: ProofEntry['direction']
  text: string
}

/** Script-grouped samples covering Latin, Cyrillic, Greek, CJK and Arabic. */
export const BUILTIN_PROOF_SAMPLES: readonly BuiltinProofSample[] = [
  {
    id: 'fc.sample.latin-pangram',
    nameKey: 'latinPangram',
    groupKey: 'latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'The quick brown fox jumps over the lazy dog.',
  },
  {
    id: 'fc.sample.latin-uppercase',
    nameKey: 'latinUppercase',
    groupKey: 'latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
  },
  {
    id: 'fc.sample.latin-lowercase',
    nameKey: 'latinLowercase',
    groupKey: 'latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'abcdefghijklmnopqrstuvwxyz',
  },
  {
    id: 'fc.sample.latin-figures',
    nameKey: 'latinFigures',
    groupKey: 'latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: '0123456789 .,:;!?¿¡ ()[]{} &@#$%‰ +-×÷=<>',
  },
  {
    id: 'fc.sample.latin-diacritics',
    nameKey: 'latinDiacritics',
    groupKey: 'latin',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'Áâãäåæçèéêëìíîïñòóôõöøùúûüýÿ ĀČĐĘĞŁŃŐŘŠŢŮŽ',
  },
  {
    id: 'fc.sample.latin-caps-pairs',
    nameKey: 'latinCapitalPairs',
    groupKey: 'latinPairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'AV AW AY FA LT LV LY PA TA VA WA YA Av Aw Ay Fa Ta To Tr Tu Tw Ty Va Vo Wa We Wo Ya Yo',
  },
  {
    id: 'fc.sample.latin-lowercase-pairs',
    nameKey: 'latinLowercasePairs',
    groupKey: 'latinPairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'av aw ay fa fo gj rc rd re rg ro rt ry ty va vy wa wy ya yo',
  },
  {
    id: 'fc.sample.latin-punctuation-pairs',
    nameKey: 'latinPunctuationPairs',
    groupKey: 'latinPairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'r. r, v. v, w. w, y. y, f. f, T. T, Y. Y, “A” ‘A’ (A) [A] /A/ A-A A–A',
  },
  {
    id: 'fc.sample.latin-spacing-words',
    nameKey: 'latinSpacingWords',
    groupKey: 'latinPairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'nnnonnn ooonooo HHHOHHH OOOHOOO nono oono nn oo no on',
  },
  {
    id: 'fc.sample.latin-phrases',
    nameKey: 'latinPhrases',
    groupKey: 'latinPairs',
    script: 'Latn',
    language: 'en',
    direction: 'ltr',
    text: 'Hamburgefonstiv · adhesion · Wavy Yacht Flags · TYPEFACE WORKSHOP',
  },
  {
    id: 'fc.sample.cyrillic-pangram',
    nameKey: 'cyrillicPangram',
    groupKey: 'cyrillic',
    script: 'Cyrl',
    language: 'ru',
    direction: 'ltr',
    text: 'Съешь же ещё этих мягких французских булок, да выпей чаю.',
  },
  {
    id: 'fc.sample.cyrillic-uppercase',
    nameKey: 'cyrillicUppercase',
    groupKey: 'cyrillic',
    script: 'Cyrl',
    language: 'ru',
    direction: 'ltr',
    text: 'АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ',
  },
  {
    id: 'fc.sample.cyrillic-pairs',
    nameKey: 'cyrillicPairs',
    groupKey: 'cyrillic',
    script: 'Cyrl',
    language: 'ru',
    direction: 'ltr',
    text: 'АУ АТ ГА ГО ЛУ РА ТА УА ог ру та ул нпнопнп оооноо',
  },
  {
    id: 'fc.sample.greek-pangram',
    nameKey: 'greekPangram',
    groupKey: 'greek',
    script: 'Grek',
    language: 'el',
    direction: 'ltr',
    text: 'Ξεσκεπάζω την ψυχοφθόρα βδελυγμία.',
  },
  {
    id: 'fc.sample.greek-uppercase',
    nameKey: 'greekUppercase',
    groupKey: 'greek',
    script: 'Grek',
    language: 'el',
    direction: 'ltr',
    text: 'ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ',
  },
  {
    id: 'fc.sample.greek-pairs',
    nameKey: 'greekPairs',
    groupKey: 'greek',
    script: 'Grek',
    language: 'el',
    direction: 'ltr',
    text: 'ΑΥ ΑΤ ΓΑ ΛΥ ΡΑ ΤΑ ΥΑ αυ γα λυ τα ονονο οοονοοο',
  },
  {
    id: 'fc.sample.cjk-phrase',
    nameKey: 'cjkPhrase',
    groupKey: 'cjk',
    script: 'Hani',
    language: 'zh-hans',
    direction: 'ltr',
    text: '天地玄黄，宇宙洪荒。日月盈昃，辰宿列张。',
  },
  {
    id: 'fc.sample.cjk-vertical',
    nameKey: 'cjkVertical',
    groupKey: 'cjk',
    script: 'Hani',
    language: 'zh-hans',
    direction: 'ttb',
    text: '永東國書道：一二三四五六七八九十（「字」）',
  },
  {
    id: 'fc.sample.cjk-mixed',
    nameKey: 'cjkMixed',
    groupKey: 'cjk',
    script: 'Hani',
    language: 'zh-hans',
    direction: 'ltr',
    text: '字体设计 FontCarver 2026 年，中西混排。',
  },
  {
    id: 'fc.sample.arabic-phrase',
    nameKey: 'arabicPhrase',
    groupKey: 'arabic',
    script: 'Arab',
    language: 'ar',
    direction: 'rtl',
    text: 'نص حكيم له سر قاطع وذو شأن عظيم مكتوب على ثوب أخضر ومغلف بجلد أزرق.',
  },
  {
    id: 'fc.sample.arabic-joining',
    nameKey: 'arabicJoining',
    groupKey: 'arabic',
    script: 'Arab',
    language: 'ar',
    direction: 'rtl',
    text: 'بـ ـبـ ـب لا لم لح مـم عـع سـس كـك هـه ١٢٣٤٥٦٧٨٩٠',
  },
]

/** Translator shape accepted by {@link builtinProofEntries}. */
export type SampleTranslator = (key: string) => string

/**
 * The built-in samples as project entries, named in the current language.
 *
 * Names and groups are translated once, when the samples are adopted, because
 * what is stored afterwards is the project's own data and must not change
 * underneath the person who saved it.
 */
export function builtinProofEntries(t: SampleTranslator): ProofEntry[] {
  return BUILTIN_PROOF_SAMPLES.map((sample) => ({
    id: sample.id,
    name: t(`proof.samples.names.${sample.nameKey}`),
    group: t(`proof.samples.groups.${sample.groupKey}`),
    script: sample.script,
    language: sample.language,
    direction: sample.direction,
    text: sample.text,
  }))
}

/** Adds the samples a project does not already hold, retaining edited copies. */
export function withBuiltinSamples(
  entries: readonly ProofEntry[],
  samples: readonly ProofEntry[],
): ProofEntry[] {
  const known = new Set(entries.map((entry) => entry.id))
  return [...entries, ...samples.filter((sample) => !known.has(sample.id))]
}
