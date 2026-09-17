import { join } from 'node:path'
import { expect } from '@playwright/test'
import { api } from './support'
import {
  addPrefix,
  editor,
  featureState,
  holdApi,
  openFeatures,
  openFont,
  replaceCode,
  saveNative,
  test,
} from './p2bSupport'

const CODE = 'feature liga { sub f i by f_i; } liga;'

test('persists pending drafts before master and tab handovers', async ({
  page,
  source,
}) => {
  await openFeatures(page)
  await addPrefix(page, 'Handover', CODE)
  const held = await holdApi(page, 'features.set_draft')
  const master = page.getByRole('combobox', {
    name: 'Preview master',
    exact: true,
  })
  try {
    await master.selectOption(source.doc.masters[1].id)
    await held.wait()
    await expect(master).toHaveValue(source.doc.masters[0].id)
    expect((await featureState(page, source.doc.doc)).draft).toBeNull()
    held.release()
    await expect(master).toHaveValue(source.doc.masters[1].id)
    expect(
      (await featureState(page, source.doc.doc)).draft?.source.prefixes[0].code,
    ).toBe(CODE)
  } finally {
    await held.dispose()
  }
  await replaceCode(page, `${CODE}\n# tab handover`)
  const tabWrite = await holdApi(page, 'features.set_draft')
  try {
    await page
      .getByRole('tab', { name: 'P2b.designspace', exact: true })
      .click()
    await tabWrite.wait()
    await expect(
      page.getByRole('tab', { name: 'Features', exact: true }),
    ).toHaveAttribute('aria-selected', 'true')
    tabWrite.release()
    await expect(
      page.getByRole('tab', { name: 'P2b.designspace', exact: true }),
    ).toHaveAttribute('aria-selected', 'true')
    expect(
      (await featureState(page, source.doc.doc)).draft?.source.prefixes[0].code,
    ).toBe(`${CODE}\n# tab handover`)
  } finally {
    await tabWrite.dispose()
  }
})

test('blocks a tab handover on a failed draft write and keeps the text available for retry', async ({
  page,
  source,
}) => {
  await openFeatures(page)
  await addPrefix(page, 'Recovery', `${CODE}\n# unsaved 中文`)
  const held = await holdApi(page, 'features.set_draft', { fail: true })
  try {
    await page
      .getByRole('tab', { name: 'P2b.designspace', exact: true })
      .click()
    await held.wait()
    held.release()
    await expect(
      page.getByRole('alert').filter({
        hasText: 'Draft could not be saved. Your text is retained here.',
      }),
    ).toBeVisible()
    await expect(
      page.getByRole('tab', { name: 'Features', exact: true }),
    ).toHaveAttribute('aria-selected', 'true')
    await expect(editor(page)).toContainText('unsaved 中文')
    expect((await featureState(page, source.doc.doc)).draft).toBeNull()
  } finally {
    await held.dispose()
  }
  await page.getByRole('tab', { name: 'P2b.designspace', exact: true }).click()
  await expect(
    page.getByRole('tab', { name: 'P2b.designspace', exact: true }),
  ).toHaveAttribute('aria-selected', 'true')
  expect(
    (await featureState(page, source.doc.doc)).draft?.source.prefixes[0].code,
  ).toContain('unsaved 中文')
})

test('finishes composition before Ctrl+S persists the complete invalid draft', async ({
  page,
  source,
}) => {
  const native = join(source.directory, 'Composition.fcf')
  await saveNative(page, native)
  await openFeatures(page)
  await addPrefix(page, 'Composition', CODE)
  await page.getByRole('button', { name: 'Check source', exact: true }).click()
  await expect(
    page.getByRole('region', { name: 'Diagnostics', exact: true }),
  ).toContainText('Source compiles in 2 masters.')
  const before = await featureState(page, source.doc.doc)
  const composing = `${CODE}\n# 中文输入 😀\nfeature kern { pos A V ; } kern;`
  const held = await holdApi(page, 'features.set_draft')
  let requested = false
  const observe = (request: import('@playwright/test').Request) => {
    if (request.url().endsWith('/api/features.set_draft')) requested = true
  }
  page.on('request', observe)
  try {
    await editor(page).dispatchEvent('compositionstart', { data: '中文输入' })
    await replaceCode(page, composing)
    await editor(page).press('Control+s')
    // A real composition has no completion event yet; a persistence attempt must wait.
    await page.waitForTimeout(100)
    expect(requested).toBe(false)
    expect((await featureState(page, source.doc.doc)).draft_revision).toBe(
      before.draft_revision,
    )
    await editor(page).dispatchEvent('compositionend', { data: '中文输入' })
    await held.wait()
    held.release()
    await expect
      .poll(
        async () =>
          (await featureState(page, source.doc.doc)).draft?.source.prefixes[0]
            .code,
      )
      .toBe(composing)
    await expect
      .poll(
        async () =>
          (
            await api<{ dirty: boolean }>(page, 'doc.summary', {
              doc: source.doc.doc,
            })
          ).dirty,
      )
      .toBe(false)
  } finally {
    page.off('request', observe)
    await held.dispose()
  }
  const reopened = await openFont(page, native)
  expect(
    (await featureState(page, reopened.doc)).draft?.source.prefixes[0].code,
  ).toBe(composing)
})
