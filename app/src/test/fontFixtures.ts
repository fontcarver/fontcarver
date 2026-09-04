/**
 * Builders for the model types the glyph and font-info views render.
 *
 * `FontInfo`, `Master` and `Layer` have a lot of fields, almost all optional in
 * practice, so tests construct them through these helpers and override only
 * what the assertion is about.
 */

import type {
  Contour,
  FontInfo,
  FontInfoView,
  Glyph,
  GlyphCell,
  GlyphDetail,
  Instance,
  Layer,
  LayerOutline,
  Master,
  MasterMetrics,
  Point,
  PointKind,
} from '../api'

/** A point of a contour; on-curve unless the kind says otherwise. */
export function makePoint(
  x: number,
  y: number,
  kind: PointKind = 'Line',
  smooth = false,
): Point {
  return { x, y, kind, smooth, name: null, tt_index: 'Unnumbered' }
}

/** A closed contour from a list of points. */
export function makeContour(points: Point[], closed = true): Contour {
  return { closed, points, name: null, id: null }
}

/** A master layer with no components, anchors or guides. */
export function makeLayer(overrides: Partial<Layer> = {}): Layer {
  return {
    id: 'layer-1',
    kind: { Master: 'm1' },
    name: 'Regular',
    width: 600,
    vertical_width: null,
    vertical_origin: null,
    contours: [],
    components: [],
    anchors: [],
    guides: [],
    background_image: null,
    hints: {
      h_stems: [],
      v_stems: [],
      d_stems: [],
      flex: false,
      hint_masks: [],
    },
    tt_instructions: null,
    color_layers: null,
    user_data: null,
    ...overrides,
  }
}

/** A glyph with one master layer. */
export function makeGlyph(overrides: Partial<Glyph> = {}): Glyph {
  return {
    name: 'a',
    unicodes: [0x61],
    variation_selectors: [],
    production_name: null,
    category: 'Letter',
    sub_category: 'Lowercase',
    script: 'latin',
    export: true,
    color_label: null,
    tags: [],
    note: '',
    kern_group_side1: null,
    kern_group_side2: null,
    metrics_keys: {
      left: null,
      right: null,
      width: null,
      vertical_origin: null,
      vertical_width: null,
    },
    layers: [makeLayer()],
    locked: false,
    user_data: null,
    ...overrides,
  }
}

/** A glyph detail whose flattened path matches the first layer. */
export function makeGlyphDetail(
  glyph: Glyph = makeGlyph(),
  flattened = 'M0 0L600 0L600 700Z',
): GlyphDetail {
  return {
    glyph,
    paths: glyph.layers.map((layer) => ({ layer: layer.id, flattened })),
  }
}

/** A grid cell for the same glyph. */
export function makeCell(overrides: Partial<GlyphCell> = {}): GlyphCell {
  return {
    gid: 1,
    name: 'a',
    unicodes: [0x61],
    category: 'Letter',
    sub_category: 'Lowercase',
    script: 'Latn',
    width: 600,
    bbox: [40, 0, 560, 700],
    path: 'M0 0L600 0L600 700Z',
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 0,
    ...overrides,
  }
}

/** Master metrics with everything the editor draws. */
export function makeMasterMetrics(
  overrides: Partial<MasterMetrics> = {},
): MasterMetrics {
  return {
    ascender: 800,
    descender: -200,
    x_height: 500,
    cap_height: 700,
    italic_angle: 0,
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
    ...overrides,
  }
}

/** One interpolation master. */
export function makeMaster(overrides: Partial<Master> = {}): Master {
  return {
    id: 'm1',
    name: 'Regular',
    location: {},
    metrics: makeMasterMetrics(),
    alignment_zones: [],
    stems: [],
    guides: [],
    kerning_h: [],
    kerning_v: [],
    private_dict: {
      blue_values: [],
      other_blues: [],
      family_blues: [],
      family_other_blues: [],
      blue_scale: null,
      blue_shift: null,
      blue_fuzz: null,
      std_hw: null,
      std_vw: null,
      stem_snap_h: [],
      stem_snap_v: [],
      force_bold: null,
      language_group: null,
      other: {},
    },
    tt: {} as Master['tt'],
    user_data: null,
    ...overrides,
  }
}

/** One named export instance. */
export function makeInstance(overrides: Partial<Instance> = {}): Instance {
  return {
    name: 'Regular',
    location: {},
    style_name: 'Regular',
    family_name: null,
    active: true,
    is_variable_named_instance: false,
    user_data: null,
    ...overrides,
  }
}

/** Font-wide metadata with only the required fields filled in. */
export function makeFontInfo(overrides: Partial<FontInfo> = {}): FontInfo {
  const info: FontInfo = {
    family_name: 'DejaVu Sans',
    style_name: 'Book',
    units_per_em: 2048,
    version_major: 2,
    version_minor: 37,
    copyright: 'Copyright (c) 2003 Bitstream Inc.',
    trademark: null,
    note: null,
    open_type_name_designer: null,
    open_type_name_designer_url: null,
    open_type_name_manufacturer: null,
    open_type_name_manufacturer_url: null,
    open_type_name_license: null,
    open_type_name_license_url: null,
    open_type_name_version: 'Version 2.37',
    open_type_name_unique_id: null,
    open_type_name_description: null,
    open_type_name_preferred_family_name: null,
    open_type_name_preferred_subfamily_name: null,
    open_type_name_compatible_full_name: null,
    open_type_name_sample_text: null,
    open_type_name_wws_family_name: null,
    open_type_name_wws_subfamily_name: null,
    postscript_font_name: 'DejaVuSans',
    postscript_full_name: null,
    names: [],
    open_type_os2_vendor_id: 'PfEd',
    open_type_os2_weight_class: 400,
    open_type_os2_width_class: 5,
    open_type_os2_selection: [6],
    open_type_os2_type: [],
    open_type_os2_panose: null,
    open_type_os2_family_class: null,
    open_type_os2_unicode_ranges: [],
    open_type_os2_code_page_ranges: [],
    open_type_os2_subscript_x_size: null,
    open_type_os2_subscript_y_size: null,
    open_type_os2_subscript_x_offset: null,
    open_type_os2_subscript_y_offset: null,
    open_type_os2_superscript_x_size: null,
    open_type_os2_superscript_y_size: null,
    open_type_os2_superscript_x_offset: null,
    open_type_os2_superscript_y_offset: null,
    open_type_os2_strikeout_size: null,
    open_type_os2_strikeout_position: null,
    open_type_head_created: '2010/01/01 00:00:00',
    open_type_head_flags: [],
    open_type_head_lowest_rec_ppem: 8,
    open_type_gasp_range_records: [],
    open_type_hhea_caret_slope_rise: null,
    open_type_hhea_caret_slope_run: null,
    open_type_hhea_caret_offset: null,
    open_type_vhea_caret_slope_rise: null,
    open_type_vhea_caret_slope_run: null,
    open_type_vhea_caret_offset: null,
    postscript_is_fixed_pitch: false,
    postscript_default_width_x: null,
    postscript_nominal_width_x: null,
    postscript_underline_position: -130,
    postscript_underline_thickness: 90,
    postscript_unique_id: null,
    macintosh_fond_name: null,
    macintosh_fond_family_id: null,
    user_data: null,
  }
  return { ...info, ...overrides }
}

/** A complete `doc.font_info` response. */
export function makeFontInfoView(
  overrides: Partial<FontInfoView> = {},
): FontInfoView {
  return {
    info: makeFontInfo(),
    masters: [makeMaster()],
    tables: [],
    instances: [],
    ...overrides,
  }
}

/** A `glyphs.outline` response for one layer, with matching metrics. */
export function makeLayerOutline(
  layer: Layer,
  overrides: Partial<LayerOutline> = {},
): LayerOutline {
  const xs = layer.contours.flatMap((contour) =>
    contour.points.map((point) => point.x),
  )
  const ys = layer.contours.flatMap((contour) =>
    contour.points.map((point) => point.y),
  )
  const bbox: [number, number, number, number] | null =
    xs.length === 0
      ? null
      : [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)]
  return {
    layer: layer.id,
    master: 'm1',
    contours: layer.contours,
    components: [],
    anchors: layer.anchors,
    guides: layer.guides,
    width: layer.width,
    lsb: bbox ? bbox[0] : 0,
    rsb: bbox ? layer.width - bbox[2] : 0,
    bbox,
    resolved_metrics: {
      lsb: bbox ? bbox[0] : 0,
      rsb: bbox ? layer.width - bbox[2] : 0,
      width: layer.width,
      source: 'Explicit',
    },
    ...overrides,
  }
}
