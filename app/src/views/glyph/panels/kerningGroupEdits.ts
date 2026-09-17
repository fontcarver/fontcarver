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

/** Registers a group command immediately so save and editor handover await it. */
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
