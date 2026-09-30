import { beforeEach, expect, it, vi } from 'vitest'

import type { ExportOptions, ExportProfile, VariableExportConfig } from '../api'
import { setTransport } from '../api'
import { createMockTransport, type MockTransport } from '../test/mockTransport'
import {
  ExportProfileError,
  MAX_EXPORT_PROFILES,
  useExportProfileStore,
} from './exportProfileStore'

function options(overrides: Partial<ExportOptions> = {}): ExportOptions {
  return {
    format: 'Ttf',
    master: 'm1',
    variable: false,
    remove_overlap: true,
    autohint: false,
    keep_tt_instructions: true,
    preserve_untouched_tables: true,
    production_names: false,
    table_subset: null,
    ...overrides,
  }
}

function profile(name: string, overrides: Partial<ExportProfile> = {}) {
  return {
    name,
    subfolder: '',
    file_name: '',
    options: options(),
    ...overrides,
  }
}

/** A transport that keeps one collection per document, like the kernel does. */
function projectTransport(): MockTransport {
  const stored = new Map<
    number,
    { profiles: ExportProfile[]; variable_configs: VariableExportConfig[] }
  >()
  const empty = () => ({ profiles: [], variable_configs: [] })
  return createMockTransport({
    'export.profiles_get': (params) =>
      structuredClone(stored.get(params.doc) ?? empty()),
    'export.profiles_set': (params) => {
      const next = {
        profiles: structuredClone(params.profiles),
        variable_configs: structuredClone(params.variable_configs),
      }
      stored.set(params.doc, next)
      return structuredClone(next)
    },
  })
}

beforeEach(() => {
  vi.restoreAllMocks()
  useExportProfileStore.setState({
    docs: {},
    loading: {},
    error: null,
    detail: null,
  })
})

it('stores profiles per document through the kernel, not in browser storage', async () => {
  const transport = projectTransport()
  setTransport(transport)
  const store = useExportProfileStore.getState()
  await store.load(1)
  expect(useExportProfileStore.getState().stored(1).profiles).toEqual([])

  const source = profile(' Web ', { subfolder: 'web/', file_name: 'MyFont' })
  const saved = await store.saveProfile(1, source)
  expect(saved.name).toBe('Web')
  expect(saved.subfolder).toBe('web')
  expect(useExportProfileStore.getState().stored(1).profiles).toHaveLength(1)
  // The write goes to the document, and nothing is left in localStorage.
  expect(
    transport.calls.filter((call) => call.method === 'export.profiles_set'),
  ).toHaveLength(1)
  expect(localStorage.getItem('fontcarver.exportProfiles')).toBeNull()

  // A second document has its own collection.
  await store.load(2)
  expect(useExportProfileStore.getState().stored(2).profiles).toEqual([])
})

it('rejects blank, overlong, duplicate and excess profile names', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  await expect(store.saveProfile(1, profile('   '))).rejects.toThrow(
    'blankName',
  )
  await expect(store.saveProfile(1, profile('x'.repeat(81)))).rejects.toThrow(
    'nameTooLong',
  )
  await store.saveProfile(1, profile('Web'))
  await expect(store.saveProfile(1, profile(' web '))).rejects.toThrow(
    'duplicateName',
  )
  for (let index = 1; index < MAX_EXPORT_PROFILES; index += 1)
    await store.saveProfile(1, profile(`Profile ${index}`))
  await expect(store.saveProfile(1, profile('Overflow'))).rejects.toThrow(
    'limitReached',
  )
  expect(useExportProfileStore.getState().stored(1).profiles).toHaveLength(
    MAX_EXPORT_PROFILES,
  )
})

it('replaces a profile only when the caller names the one it replaces', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  await store.saveProfile(1, profile('Web'))
  await store.saveProfile(
    1,
    profile('Web', { options: options({ format: 'Woff2' }) }),
    'Web',
  )
  const stored = useExportProfileStore.getState().stored(1).profiles
  expect(stored).toHaveLength(1)
  expect(stored[0]?.options.format).toBe('Woff2')
})

it('refuses a subfolder or file name that could leave the destination', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  for (const subfolder of [
    '../up',
    '/absolute',
    'C:/drive',
    'a//b',
    './here',
  ]) {
    await expect(
      store.saveProfile(1, profile('Escape', { subfolder })),
    ).rejects.toThrow('invalidSubfolder')
  }
  for (const fileName of ['a/b', 'a\\b', 'C:name', '.hidden']) {
    await expect(
      store.saveProfile(1, profile('Escape', { file_name: fileName })),
    ).rejects.toThrow('invalidFileName')
  }
  expect(useExportProfileStore.getState().stored(1).profiles).toEqual([])
})

it('does not report a saved profile when the kernel rejects the write', async () => {
  const transport = createMockTransport({
    'export.profiles_get': () => ({ profiles: [], variable_configs: [] }),
    'export.profiles_set': () => {
      throw new Error('invalid stored export profiles')
    },
  })
  setTransport(transport)
  const store = useExportProfileStore.getState()
  await expect(store.saveProfile(1, profile('Web'))).rejects.toMatchObject({
    code: 'kernelRejected',
  })
  expect(useExportProfileStore.getState().stored(1).profiles).toEqual([])
})

it('removes only an existing profile and reports a missing one', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  await store.saveProfile(1, profile('Web'))
  await expect(store.removeProfile(1, 'Nope')).rejects.toThrow('notFound')
  await store.removeProfile(1, ' WEB ')
  expect(useExportProfileStore.getState().stored(1).profiles).toEqual([])
})

it('stores variable-font configurations beside the profiles', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  await store.saveProfile(1, profile('VF'))
  const config: VariableExportConfig = {
    name: 'Text',
    file_name: 'MyFont-Text',
    masters: ['m1'],
    axes: ['wght'],
    instances: ['Bold'],
  }
  await store.saveConfig(1, config)
  const stored = useExportProfileStore.getState().stored(1)
  expect(stored.profiles).toHaveLength(1)
  expect(stored.variableConfigs).toEqual([config])
  await expect(
    store.saveConfig(1, { ...config, name: ' text ' }),
  ).rejects.toThrow('duplicateName')
  await store.removeConfig(1, 'Text')
  expect(useExportProfileStore.getState().stored(1).variableConfigs).toEqual([])
  expect(useExportProfileStore.getState().stored(1).profiles).toHaveLength(1)
})

it('drops a closed document\u2019s cached configurations', async () => {
  setTransport(projectTransport())
  const store = useExportProfileStore.getState()
  await store.saveProfile(1, profile('Web'))
  expect(useExportProfileStore.getState().stored(1).profiles).toHaveLength(1)
  useExportProfileStore.getState().forget(1)
  expect(useExportProfileStore.getState().stored(1).profiles).toEqual([])
})

it('surfaces a failed read without pretending the document has no profiles', async () => {
  setTransport(
    createMockTransport({
      'export.profiles_get': () => {
        throw new Error('invalid stored export profiles: bad shape')
      },
    }),
  )
  await expect(useExportProfileStore.getState().load(3)).rejects.toBeInstanceOf(
    ExportProfileError,
  )
  expect(useExportProfileStore.getState().error).toBe('kernelRejected')
  expect(useExportProfileStore.getState().detail).toContain('bad shape')
})
