/**
 * OpenType `name` table identifiers.
 *
 * The Names table shows the raw id next to a readable label, because font
 * engineers reason in ids (`nameID 6` is the PostScript name) while everyone
 * else reads the label.
 */

/** i18n key suffixes for the standard name ids, indexed by id. */
const NAME_ID_KEYS: readonly (string | undefined)[] = [
  'copyright',
  'family',
  'subfamily',
  'uniqueId',
  'fullName',
  'version',
  'postscriptName',
  'trademark',
  'manufacturer',
  'designer',
  'description',
  'vendorUrl',
  'designerUrl',
  'license',
  'licenseUrl',
  undefined,
  'typographicFamily',
  'typographicSubfamily',
  'compatibleFull',
  'sampleText',
  'postscriptCid',
  'wwsFamily',
  'wwsSubfamily',
  'lightBackground',
  'darkBackground',
  'variationsPrefix',
]

/** The i18n key for a name id, or `null` when the id has no standard meaning. */
export function nameIdKey(id: number): string | null {
  const key = NAME_ID_KEYS[id]
  return key === undefined ? null : `fontInfo.nameId.${key}`
}
