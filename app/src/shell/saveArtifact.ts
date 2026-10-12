import { choosePath, hasNativeFileDialog } from './nativeDialogs'

/** Saves a generated artifact through a scoped native dialog or a browser download. */
export async function saveArtifact(
  name: string,
  bytes: Uint8Array,
  mimeType: string,
  filterName: string,
): Promise<boolean> {
  if (hasNativeFileDialog()) {
    const extension = name.split('.').at(-1) ?? ''
    const choice = await choosePath({
      kind: 'save',
      defaultPath: name,
      extensions: [extension],
      filterName,
    })
    if (choice.kind !== 'chosen') return false
    const { writeFile } = await import('@tauri-apps/plugin-fs')
    await writeFile(choice.path, bytes)
    return true
  }
  const url = URL.createObjectURL(
    new Blob([new Uint8Array(bytes)], { type: mimeType }),
  )
  try {
    const link = document.createElement('a')
    link.href = url
    link.download = name
    link.hidden = true
    document.body.append(link)
    link.click()
    link.remove()
  } finally {
    // Let browsers consume the URL before revoking it; no font data is retained.
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }
  return true
}
