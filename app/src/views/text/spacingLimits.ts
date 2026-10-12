/**
 * Bound shared by the Spacing panel and the `metrics.spacing` API.
 *
 * It lives in its own module so the pure matrix helpers do not have to import
 * generated API code, which keeps them usable in isolation.
 */

/** Largest number of glyph rows one spacing request may ask for. */
export const SPACING_MATRIX_LIMIT = 256
