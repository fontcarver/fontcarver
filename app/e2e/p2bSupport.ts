import { basename } from 'node:path'
import { expect, test as base, type Page, type Route } from '@playwright/test'
import type { DocSummary } from '../src/api/generated/DocSummary'
import type { FeatureState } from '../src/api/generated/FeatureState'
import type { FeatureApplyResult } from '../src/api/generated/FeatureApplyResult'
import type { ShapedRun } from '../src/api/generated/ShapedRun'
import { menuItem } from './acceptanceSupport'
import { featureFixture } from './p2bFixtures'
import { api } from './support'

export type SourceFixture = ReturnType<typeof featureFixture> & {
  doc: DocSummary
}
export const test = base.extend<{ source: SourceFixture }>({
  source: async ({ page }, runTest) => {
    const files = featureFixture()
    let inherited = new Set<number>()
    try {
      // This suite owns the server/profile. Timed recovery snapshots must not
      // appear as unrelated startup dialogs between independent scenarios.
      const prefs = (
        await (await page.request.post('/api/prefs.get', { data: {} })).json()
      ).result
      await page.request.post('/api/prefs.set', {
        data: { prefs: { ...prefs, autosave_seconds: 0 } },
      })
      await page.goto('/')
      await dismissRecovery(page)
      // Whatever the session already held stays open: under the default
      // configuration this suite shares a kernel that was started with the
      // DejaVu fixture, and the specs that run after it need that document.
      inherited = new Set(
        (await api<DocSummary[]>(page, 'session.list', {})).map(
          (document) => document.doc,
        ),
      )
      const doc = await openFont(page, files.input)
      await runTest({ ...files, doc })
    } finally {
      try {
        if (!page.isClosed()) {
          if (test.info().status !== test.info().expectedStatus) {
            await page.screenshot({
              path: test.info().outputPath('before-cleanup.png'),
            })
            await test.info().attach('page-before-cleanup', {
              body: await page.locator('body').ariaSnapshot(),
              contentType: 'text/plain',
            })
          }
          for (const document of await api<DocSummary[]>(
            page,
            'session.list',
            {},
          ))
            if (!inherited.has(document.doc))
              await api(page, 'session.close', {
                doc: document.doc,
                force: true,
              })
        }
      } finally {
        files.cleanup()
      }
    }
  },
})

/**
 * Dismiss a recovery prompt left by an earlier scenario in this worker.
 *
 * Each test opens its own font; an unsaved document from the previous test must
 * not block the menu bar with a modal.
 */
export async function dismissRecovery(page: Page) {
  const later = page.getByRole('button', { name: 'Later', exact: true })
  // The prompt is decided after the app asks the kernel, so wait briefly for
  // it rather than racing the first render.
  await later.waitFor({ state: 'visible', timeout: 4_000 }).catch(() => {})
  if (await later.isVisible().catch(() => false)) {
    await later.click()
    await expect(later).toBeHidden()
  }
}

export async function openFont(page: Page, path: string) {
  const before = new Set(
    (await api<DocSummary[]>(page, 'session.list', {})).map((doc) => doc.doc),
  )
  await menuItem(page, 'File', /^Open Font/)
  const dialog = page.getByRole('dialog', { name: 'Open Font' })
  await dialog.getByRole('textbox', { name: 'File path' }).fill(path)
  await dialog.getByRole('button', { name: 'Open', exact: true }).click()
  let opened: DocSummary | undefined
  await expect
    .poll(
      async () => {
        const documents = await api<DocSummary[]>(page, 'session.list', {})
        // A path the shell already has open is focused rather than opened
        // twice, so an existing document for it counts as the result.
        opened =
          documents.find((doc) => !before.has(doc.doc) && doc.path === path) ??
          documents.find((doc) => doc.path === path)
        return opened?.doc
      },
      { timeout: 20_000 },
    )
    .toBeTruthy()
  await expect(
    page.getByRole('tab', { name: basename(path) }).last(),
  ).toHaveAttribute('aria-selected', 'true')
  return opened!
}

export async function openFeatures(page: Page) {
  await menuItem(page, 'View', /^Features/)
  await expect(
    page.getByRole('tab', { name: 'Features' }).last(),
  ).toHaveAttribute('aria-selected', 'true')
  await expect(
    page.getByRole('button', { name: 'Lookups', exact: true }),
  ).toBeVisible()
}

export async function saveNative(page: Page, path: string) {
  await menuItem(page, 'File', /^Save As/)
  const dialog = page.getByRole('dialog', { name: 'Save As' })
  await dialog.getByRole('textbox', { name: 'Document path' }).fill(path)
  await dialog.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(dialog).toBeHidden()
}

export function featureState(page: Page, doc: number) {
  return api<FeatureState>(page, 'features.get', { doc })
}

export function revisions(state: FeatureState) {
  return {
    doc: state.doc,
    expected_generation: state.generation,
    expected_compile_revision: state.compile_revision,
    expected_draft_revision: state.draft_revision,
  }
}

export function editor(page: Page) {
  return page.getByRole('textbox', { name: 'Feature source code', exact: true })
}

export async function replaceCode(page: Page, code: string) {
  await editor(page).click()
  await editor(page).press('Control+a')
  await page.keyboard.insertText(code)
}

export async function addPrefix(page: Page, name: string, code: string) {
  await page.getByRole('button', { name: 'Add prefix', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Prefix name', exact: true })
    .fill(name)
  await replaceCode(page, code)
}

export async function applySource(page: Page, doc: number) {
  await page
    .getByRole('button', { name: 'Compile & preview', exact: true })
    .click()
  await expect
    .poll(async () => (await featureState(page, doc)).has_unapplied_changes)
    .toBe(false)
  await expect(
    page.getByRole('region', { name: 'Diagnostics', exact: true }),
  ).toContainText(/Source compiles in \d+ masters?\./)
}

/** Seed an accepted source fixture through the same guarded API as the UI. */
export async function seedSource(page: Page, doc: number, code: string) {
  const before = await featureState(page, doc)
  const source = {
    prefixes: [
      {
        id: 'p2b-fixture-prefix',
        tag: 'Fixture source',
        code,
        automatic: false,
        disabled: false,
        notes: '',
      },
    ],
    classes: [],
    features: [],
    source_of_truth: 'Fea',
  }
  const draft = await api<FeatureState>(page, 'features.set_draft', {
    doc,
    expected_generation: before.generation,
    expected_draft_revision: before.draft_revision,
    source,
    settings: before.active_settings,
    token: null,
  })
  const applied = await api<FeatureApplyResult>(
    page,
    'features.apply',
    revisions(draft),
  )
  expect(applied.applied, JSON.stringify(applied.validation.diagnostics)).toBe(
    true,
  )
  return applied.state
}

export async function shape(
  page: Page,
  doc: number,
  text: string,
  features: Array<[string, boolean]> = [
    ['kern', true],
    ['liga', true],
  ],
  master: string | null = null,
  direction: 'ltr' | 'ttb' = 'ltr',
) {
  let current: ShapedRun | undefined
  await expect
    .poll(
      async () => {
        current = await api<ShapedRun>(page, 'text.shape', {
          doc,
          text,
          master,
          features,
          direction,
        })
        return current.stale
      },
      { timeout: 15_000 },
    )
    .toBe(false)
  return current!.glyphs.map((glyph, index) => ({
    name: current!.names[index],
    cluster: glyph.cluster,
    x_advance: glyph.x_advance,
    y_advance: glyph.y_advance,
    x_offset: glyph.x_offset,
    y_offset: glyph.y_offset,
  }))
}

export async function exportFont(
  page: Page,
  doc: number,
  path: string,
  format = 'Ttf',
) {
  return api(page, 'session.export', { doc, path, options: { format } })
}

/** Delay a real mutation or real response without substituting backend results. */
export async function holdApi(
  page: Page,
  method: string,
  options: { response?: boolean; fail?: boolean } = {},
) {
  let release = () => {}
  const gate = new Promise<void>((resolve) => {
    release = resolve
  })
  let captured = false
  let reached = false
  let complete = () => {}
  const handled = new Promise<void>((resolve) => {
    complete = resolve
  })
  const handler = async (route: Route) => {
    if (captured) {
      await route.continue()
      return
    }
    captured = true
    const response = options.response ? await route.fetch() : null
    reached = true
    await gate
    try {
      if (options.fail)
        await route.fulfill({
          status: 400,
          contentType: 'application/json',
          body: JSON.stringify({
            error: { code: 'io', message: 'P2b delayed draft write failure' },
          }),
        })
      else if (response) await route.fulfill({ response })
      else await route.continue()
    } finally {
      complete()
    }
  }
  const pattern = `**/api/${method}`
  await page.route(pattern, handler)
  return {
    wait: () => expect.poll(() => reached).toBe(true),
    release,
    dispose: async () => {
      release()
      if (captured) await handled
      await page.unroute(pattern, handler)
    },
  }
}
