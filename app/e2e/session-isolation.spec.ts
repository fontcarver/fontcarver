import { expect } from '@playwright/test'
import type { DocSummary } from '../src/api/generated/DocSummary'
import type { Prefs } from '../src/api/generated/Prefs'
import { test } from './sessionFixtures'
import { fixture, requestApi } from './support'

test('owns dirty documents and preferences even after its page closes', async ({
  page,
  request,
}) => {
  await page.goto('/')
  const documents = await requestApi<DocSummary[]>(request, 'session.list', {})
  expect(documents).toHaveLength(1)
  await requestApi(request, 'glyphs.add', {
    doc: documents[0].doc,
    recipe: 'A.isolation',
    master: null,
    dry_run: false,
  })
  await requestApi(request, 'session.new', {
    family: 'Must not survive this test',
    units_per_em: 1000,
  })
  const prefs = await requestApi<Prefs>(request, 'prefs.get', {})
  await requestApi(request, 'prefs.set', {
    prefs: { ...prefs, language: 'zh-CN', theme: 'dark' },
  })
  expect(await requestApi(request, 'session.list', {})).toHaveLength(2)
  const opening = page.waitForRequest('**/api/session.open')
  await page.evaluate((path) => {
    // Match a test timing out while its Open Font request is still parsing.
    void fetch('/api/session.open', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ path, face_index: null }),
    }).catch(() => {})
  }, fixture('NotoSans-Regular.ttf'))
  await opening
  await page.close()
  // Deliberately leave everything dirty: fixture teardown owns the cleanup,
  // including when a test has already closed its browser page.
})

test('starts with a pristine font and preferences after the preceding test', async ({
  request,
}) => {
  const documents = await requestApi<DocSummary[]>(request, 'session.list', {})
  expect(documents).toHaveLength(1)
  expect(documents[0]).toMatchObject({
    family_name: 'DejaVu Sans',
    glyph_count: 6253,
    dirty: false,
    undo_label: null,
    redo_label: null,
  })
  expect(await requestApi(request, 'prefs.get', {})).toMatchObject({
    language: 'en',
    theme: 'light',
    autosave_seconds: 0,
  })
})
