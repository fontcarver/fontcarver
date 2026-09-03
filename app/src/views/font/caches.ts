/**
 * Lifetime of the font view's per-document caches.
 *
 * Glyph outlines, glyph names and master metrics are all keyed by document and
 * all outlive any single grid mount, so they are released in one place when the
 * document's view state is discarded — which `sessionStore.close` does as part
 * of closing a document.
 */

import type { DocId } from '../../api'
import { pathCache } from '../../canvas/pathCache'
import { onDocumentForgotten } from '../../stores/documentStore'

import { forgetGlyphNames } from './glyphNames'
import { forgetFontMetrics } from './useFontMetrics'

/** Releases every per-document cache the font view keeps. */
export function releaseDocumentCaches(doc: DocId): void {
  pathCache.clearDoc(doc)
  forgetGlyphNames(doc)
  forgetFontMetrics(doc)
}

onDocumentForgotten(releaseDocumentCaches)
