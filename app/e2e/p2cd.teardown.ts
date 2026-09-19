import { rmSync } from 'node:fs'
import { basename } from 'node:path'
import type { FullConfig } from '@playwright/test'

/** Remove only the temporary preferences directory allocated by this suite. */
export default function teardown(config: FullConfig) {
  const directory: unknown = config.metadata.configDir
  if (
    typeof directory === 'string' &&
    basename(directory).startsWith('fc-p2cd-e2e-config-')
  )
    rmSync(directory, { recursive: true, force: true })
}
