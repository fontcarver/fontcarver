import {
  call,
  type Command,
  type DocId,
  type KerningGroupSide,
} from '../../../api'
import { activeEditor } from '../../../editor/activeEditor'
import {
  holdDocumentEdits,
  pendingDocumentEdits,
} from '../../../editor/editSession'

/** Converts a visible group suffix to its canonical UFO group name. */
export function canonicalGroupName(
  side: KerningGroupSide,
  input: string,
): string {
  const text = input.trim()
  return text.startsWith('public.kern')
    ? text
    : `${side === 'side1' ? 'public.kern1.' : 'public.kern2.'}${text}`
}

/**
 * Registers a command immediately so save and editor handover await it.
 *
 * Kerning-group edits need this and named it; the spacing matrix and the
 * kerning table need exactly the same barriers, so the alias below reads
 * correctly at those call sites without a second implementation.
 */
export function commitKerningGroup(
  doc: DocId,
  command: Command,
  label: string,
): Promise<void> {
  const settling = activeEditor()?.settle()
  const earlier = pendingDocumentEdits(doc)
  const work = Promise.all([settling, earlier]).then(async () => {
    // The gate above was captured before publishing this operation. Calling
    // applyCommand here would wait for our own published barrier forever.
    await call('doc.apply', { doc, command, label })
  })
  holdDocumentEdits(doc, work)
  return work
}

/** `commitKerningGroup` under a name that fits non-group spacing commands. */
export const commitSpacingEdit = commitKerningGroup
