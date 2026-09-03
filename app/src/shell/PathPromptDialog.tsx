import { useUiStore } from '../stores/uiStore'

import { PathPromptForm } from './PathPromptForm'

/**
 * Path prompt used when no native file dialog is available (the browser/HTTP
 * transport, and end-to-end tests). Under Tauri the native dialog is used
 * instead and the form never mounts.
 */
export function PathPromptDialog() {
  const prompt = useUiStore((state) => state.pathPrompt)
  return prompt ? <PathPromptForm key={prompt.mode} prompt={prompt} /> : null
}
