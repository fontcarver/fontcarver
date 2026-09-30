import { expect } from '@playwright/test'

import {
  applySource,
  editor,
  featureState,
  openFeatures,
  replaceCode,
  shape,
  test,
} from './p2bSupport'

const panel = (page: Parameters<typeof openFeatures>[0]) =>
  page.getByRole('complementary', { name: 'Automatic features', exact: true })

const names = (glyphs: { name: string | null }[]) =>
  glyphs.map((glyph) => glyph.name)

/** Row 48: every enumerated recipe generates rules that really shape. */
test('generates every enumerated feature family from the glyph inventory', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  for (const label of [
    'Small capitals (.sc → smcp)',
    'Capitals to small capitals (c2sc)',
    'Stylistic sets (.ss01–.ss20)',
    'Case forms (.case → case)',
    'Vertical forms (.vert → vert/vrt2)',
    'Standard ligatures (underscore names → liga)',
    'Figure styles (.lf/.osf/.tf/.pf → lnum/onum/tnum/pnum)',
    'Positional figures (.sups/.sinf/.numr/.dnom)',
    'Fractions (.numr and .dnom → frac)',
    'Ordinals after figures (ordn)',
    'Access all alternates (aalt)',
  ])
    await panel(page)
      .getByRole('checkbox', { name: label, exact: true })
      .check()
  await applySource(page, doc)

  const state = await featureState(page, doc)
  const tags = state.active_source.features.map((block) => block.tag).sort()
  expect(tags).toEqual(
    [
      'aalt',
      'c2sc',
      'case',
      'dnom',
      'frac',
      'liga',
      'lnum',
      'numr',
      'onum',
      'ordn',
      'pnum',
      'sinf',
      'smcp',
      'ss01',
      'sups',
      'tnum',
      'vert',
      'vrt2',
    ].sort(),
  )
  expect(state.active_source.features.every((block) => block.automatic)).toBe(
    true,
  )

  // Single substitutions from suffixes.
  for (const [tag, text, expected] of [
    ['smcp', 'a', 'a.sc'],
    ['c2sc', 'A', 'a.sc'],
    ['ss01', 'A', 'A.ss01'],
    ['case', 'A', 'A.case'],
    ['lnum', '1', 'one.lf'],
    ['onum', '1', 'one.osf'],
    ['tnum', '1', 'one.tf'],
    ['pnum', '1', 'one.pnum'],
    ['sups', '1', 'one.sups'],
    ['sinf', '1', 'one.sinf'],
    ['numr', '1', 'one.numr'],
    ['dnom', '1', 'one.dnom'],
  ] as const) {
    expect(
      names(await shape(page, doc, text, [[tag, true]])),
      `${tag} substitution`,
    ).toEqual([expected])
  }
  // Ligature, ordinals and multi-digit fractions.
  expect(names(await shape(page, doc, 'fi', [['liga', true]]))).toEqual(['f_i'])
  expect(names(await shape(page, doc, '1a', [['ordn', true]]))).toEqual([
    'one',
    'ordfeminine',
  ])
  expect(names(await shape(page, doc, '1.', [['ordn', true]]))).toEqual([
    'one',
    'period.ordn',
  ])
  expect(names(await shape(page, doc, '12/12', [['frac', true]]))).toEqual([
    'one.numr',
    'two.numr',
    'fraction',
    'one.dnom',
    'two.dnom',
  ])
  // Row 51: vertical forms apply by default in a vertical run only.
  expect(names(await shape(page, doc, 'A', [], null, 'ttb'))).toEqual([
    'A.vert',
  ])
  expect(names(await shape(page, doc, 'A', []))).toEqual(['A'])
  // aalt references the alternate features rather than copying their rules.
  const aalt = state.active_source.features.find(
    (block) => block.tag === 'aalt',
  )
  expect(aalt?.code).toContain('feature ss01;')
  expect(aalt?.code).toContain('feature smcp;')
})

/** Row 47: the per-entry automatic switch works in both directions. */
test('switches one entry between automatic and manual without touching the others', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  await panel(page)
    .getByRole('checkbox', {
      name: 'Stylistic sets (.ss01–.ss20)',
      exact: true,
    })
    .check()
  await panel(page)
    .getByRole('checkbox', { name: 'Small capitals (.sc → smcp)', exact: true })
    .check()
  await applySource(page, doc)
  expect(names(await shape(page, doc, 'A', [['ss01', true]]))).toEqual([
    'A.ss01',
  ])

  // Automatic → manual for `ss01` alone; the recipe stays enabled.
  const tree = page.getByRole('navigation', { name: 'Feature source' })
  await tree.getByRole('button', { name: 'ss01', exact: true }).click()
  const toggle = page.getByRole('checkbox', { name: 'Automatic', exact: true })
  await expect(toggle).toBeChecked()
  // Freezing round-trips through the kernel, so the switch flips on the reply.
  await toggle.click()
  await expect
    .poll(async () => (await featureState(page, doc)).has_unapplied_changes)
    .toBe(true)
  await expect(toggle).not.toBeChecked()
  await applySource(page, doc)
  const frozen = await featureState(page, doc)
  expect(frozen.active_settings.stylistic_sets).toBe(true)
  expect(frozen.active_settings.manual_entries).toContain('ss01')
  const manual = frozen.active_source.features.find(
    (block) => block.tag === 'ss01',
  )
  expect(manual?.automatic).toBe(false)
  expect(manual?.code).toContain('A.ss01')
  // The other recipe entry is untouched and still generated.
  expect(
    frozen.active_source.features.find((block) => block.tag === 'smcp')
      ?.automatic,
  ).toBe(true)

  // Editing the now-manual entry is allowed and changes shaping.
  await tree.getByRole('button', { name: 'ss01', exact: true }).click()
  await replaceCode(page, 'sub \\A by \\A.case;')
  await applySource(page, doc)
  expect(names(await shape(page, doc, 'A', [['ss01', true]]))).toEqual([
    'A.case',
  ])

  // Manual → automatic asks first, then the recipe takes the entry back.
  await page.getByRole('checkbox', { name: 'Automatic', exact: true }).click()
  const dialog = page.getByRole('dialog', {
    name: 'Generate this entry automatically?',
  })
  await expect(dialog).toContainText('ss01')
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click()
  await expect(dialog).toBeHidden()
  expect(names(await shape(page, doc, 'A', [['ss01', true]]))).toEqual([
    'A.case',
  ])

  await page.getByRole('checkbox', { name: 'Automatic', exact: true }).click()
  await page
    .getByRole('button', { name: 'Replace with generated source', exact: true })
    .click()
  await applySource(page, doc)
  const regenerated = await featureState(page, doc)
  expect(regenerated.active_settings.manual_entries).not.toContain('ss01')
  expect(
    regenerated.active_source.features.find((block) => block.tag === 'ss01')
      ?.automatic,
  ).toBe(true)
  expect(names(await shape(page, doc, 'A', [['ss01', true]]))).toEqual([
    'A.ss01',
  ])
})

/** Row 47: a hand-written sibling must not block the entry being switched. */
test('keeps a hand-written sibling while switching one figure style to automatic', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  // The user owns `onum` and `lnum` by hand; only `lnum` is handed over.
  const tagBox = page.getByRole('textbox', { name: 'Feature tag', exact: true })
  for (const [tag, code] of [
    ['onum', 'sub \\one by \\one.tf;'],
    ['lnum', 'sub \\one by \\one.pnum;'],
  ]) {
    await page.getByRole('button', { name: 'Add feature', exact: true }).click()
    // The new entry is selected once its default tag appears; typing before
    // that would edit the previous entry.
    await expect(tagBox).toHaveValue('liga')
    await tagBox.fill(tag)
    await expect(tagBox).toHaveValue(tag)
    await replaceCode(page, code)
    await expect(editor(page)).toContainText(code)
  }
  await applySource(page, doc)
  expect(names(await shape(page, doc, '1', [['onum', true]]))).toEqual([
    'one.tf',
  ])

  const tree = page.getByRole('navigation', { name: 'Feature source' })
  await tree.getByRole('button', { name: 'lnum', exact: true }).click()
  await page.getByRole('checkbox', { name: 'Automatic', exact: true }).click()
  await page
    .getByRole('button', { name: 'Replace with generated source', exact: true })
    .click()
  await applySource(page, doc)

  const state = await featureState(page, doc)
  // The recipe owns `lnum`, the hand-written `onum` survives untouched, and no
  // conflict was raised.
  expect(state.active_settings.figure_styles).toBe(true)
  expect(state.active_settings.manual_entries).toContain('onum')
  const onum = state.active_source.features.find(
    (block) => block.tag === 'onum',
  )
  expect(onum?.automatic).toBe(false)
  expect(onum?.code).toBe('sub \\one by \\one.tf;')
  const lnum = state.active_source.features.find(
    (block) => block.tag === 'lnum',
  )
  expect(lnum?.automatic).toBe(true)
  expect(lnum?.code).toContain('one.lf')
  expect(names(await shape(page, doc, '1', [['lnum', true]]))).toEqual([
    'one.lf',
  ])
  expect(names(await shape(page, doc, '1', [['onum', true]]))).toEqual([
    'one.tf',
  ])
  // `tnum` and `pnum` have no user entry, so the recipe generated them too.
  expect(names(await shape(page, doc, '1', [['tnum', true]]))).toEqual([
    'one.tf',
  ])
})

/** Row 47: automatic glyph classes are entries with the same switch. */
test('generates glyph classes from metadata and keeps a frozen class editable', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  await panel(page)
    .getByText('Automatic glyph classes', { exact: true })
    .click()
  await panel(page)
    .getByRole('checkbox', { name: '@Figures', exact: true })
    .check()
  await applySource(page, doc)
  const generated = await featureState(page, doc)
  const figures = generated.active_source.classes.find(
    (entry) => entry.name === 'Figures',
  )
  expect(figures?.automatic).toBe(true)
  expect(figures?.code).toContain('\\one')

  const tree = page.getByRole('navigation', { name: 'Feature source' })
  await tree.getByRole('button', { name: '@Figures', exact: true }).click()
  await page.getByRole('checkbox', { name: 'Automatic', exact: true }).click()
  await expect
    .poll(async () => (await featureState(page, doc)).has_unapplied_changes)
    .toBe(true)
  await applySource(page, doc)
  const frozen = await featureState(page, doc)
  const kept = frozen.active_source.classes.find(
    (entry) => entry.name === 'Figures',
  )
  expect(kept?.automatic).toBe(false)
  expect(kept?.code).toBe(figures?.code)
  expect(frozen.active_settings.automatic_classes).not.toContain('Figures')
})

/** Row 100: tokens and `#ifdef VARIABLE` compile through the editor. */
test('compiles feature tokens and target conditionals from the editor', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  await page.getByRole('button', { name: 'Add feature', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Feature tag', exact: true })
    .fill('kern')
  await replaceCode(
    page,
    "#ifdef VARIABLE\npos \\A ${-A:width / 5};\n#else\npos [$[name == 'A']] ${-A:width / 10};\n#endif\n",
  )
  await applySource(page, doc)
  const state = await featureState(page, doc)
  // The dialect is stored unexpanded, so it stays editable.
  expect(
    state.active_source.features.find((block) => block.tag === 'kern')?.code,
  ).toContain('${-A:width / 10}')
  const shaped = await shape(page, doc, 'A', [['kern', true]])
  const plain = await shape(page, doc, 'A', [['kern', false]])
  expect(shaped[0].x_advance).toBeLessThan(plain[0].x_advance)

  // An unknown condition name fails visibly and does not activate.
  await replaceCode(page, '#ifdef WHATEVER\npos \\A -10;\n#endif\n')
  await page.getByRole('button', { name: 'Check source', exact: true }).click()
  await expect(
    page.getByRole('region', { name: 'Diagnostics', exact: true }),
  ).toContainText(/VARIABLE/)
  expect(
    (await featureState(page, doc)).active_source.features[0].code,
  ).toContain('${-A:width / 10}')
})

/** Row 50: Alt-click on a class name previews its members. */
test('previews class members from the source editor with Alt-click', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  await page.getByRole('button', { name: 'Add class', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Class name', exact: true })
    .fill('Caps')
  await replaceCode(page, '\\A \\V')
  await page.getByRole('button', { name: 'Add feature', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Feature tag', exact: true })
    .fill('ss02')
  await replaceCode(page, 'sub @Caps by [\\A.sc \\a.sc];')
  await applySource(page, doc)
  expect(names(await shape(page, doc, 'A', [['ss02', true]]))).toEqual(['A.sc'])

  // Alt+Enter with the caret inside the reference is the keyboard equivalent.
  await editor(page).click()
  await editor(page).press('Control+Home')
  for (let index = 0; index < 9; index += 1)
    await editor(page).press('ArrowRight')
  await editor(page).press('Alt+Enter')
  const preview = page.getByRole('region', { name: /class/i })
  await expect(preview).toContainText('A')
  await expect(preview).toContainText('V')
})

/** Row 99: one rule per supported lookup type, authored visually. */
test('authors one rule per supported lookup type in the visual editor', async ({
  page,
  source,
}) => {
  const doc = source.doc.doc
  await openFeatures(page)
  await page.getByRole('button', { name: 'Visual editor', exact: true }).click()
  const region = page.getByRole('region', {
    name: 'Visual lookup editor',
    exact: true,
  })
  await expect(region).toBeVisible()

  const fill = async (label: string, value: string) => {
    await region.getByRole('textbox', { name: label, exact: true }).fill(value)
  }
  const cases: [string, string, () => Promise<void>, string][] = [
    [
      'GSUB 1 · Single substitution',
      'ss03',
      async () => {
        await fill('Input glyphs', 'A')
        await fill('Output glyphs', 'A.sc')
      },
      'sub \\A by \\A.sc;',
    ],
    [
      'GSUB 2 · Multiple substitution',
      'ss04',
      async () => {
        await fill('Input glyphs', 'f_i')
        await fill('Output glyphs', 'f i')
      },
      'sub \\f_i by \\f \\i;',
    ],
    [
      'GSUB 3 · Alternate substitution',
      'ss05',
      async () => {
        await fill('Input glyphs', 'A')
        await fill('Output glyphs', 'A.sc A.ss01')
      },
      'sub \\A from [\\A.sc \\A.ss01];',
    ],
    [
      'GSUB 4 · Ligature substitution',
      'ss06',
      async () => {
        await fill('Input glyphs', 'f i')
        await fill('Output glyphs', 'f_i')
      },
      'sub \\f \\i by \\f_i;',
    ],
    [
      'GSUB 6 · Contextual substitution',
      'ss07',
      async () => {
        await fill('Preceding glyphs', 'V')
        await fill('Input glyphs', 'A')
        await fill('Output glyphs', 'A.sc')
      },
      "sub \\V \\A' by \\A.sc;",
    ],
    [
      'GPOS 1 · Single positioning',
      'ss08',
      async () => {
        await fill('Input glyphs', 'A')
        await region
          .getByRole('spinbutton', { name: 'X advance', exact: true })
          .fill('-15')
      },
      'pos \\A <0 0 -15 0>;',
    ],
    [
      'GPOS 2 · Pair positioning',
      'ss09',
      async () => {
        await fill('First glyphs', 'A')
        await fill('Second glyphs', 'V')
        await region
          .getByRole('group', { name: 'First glyph adjustment', exact: true })
          .getByRole('spinbutton', { name: 'X advance', exact: true })
          .fill('-25')
      },
      'pos \\A <0 0 -25 0> \\V <0 0 0 0>;',
    ],
    [
      'GPOS 4 · Mark to base',
      'ss10',
      async () => {
        await fill('Mark glyphs', 'acutecomb')
        await fill('Attachment glyphs', 'A')
        await region
          .getByRole('group', { name: 'Attachment anchor', exact: true })
          .getByRole('spinbutton', { name: 'Y', exact: true })
          .fill('700')
      },
      'pos base \\A <anchor 0 700> mark @visual_',
    ],
    [
      'GPOS 6 · Mark to mark',
      'ss11',
      async () => {
        await fill('Mark glyphs', 'acutecomb')
        await fill('Attachment glyphs', 'acutecomb')
        await region
          .getByRole('group', { name: 'Attachment anchor', exact: true })
          .getByRole('spinbutton', { name: 'Y', exact: true })
          .fill('900')
      },
      'pos mark \\acutecomb <anchor 0 900> mark @visual_',
    ],
  ]
  for (const [kind, tag, operands, expected] of cases) {
    await region
      .getByRole('combobox', { name: 'New lookup type', exact: true })
      .selectOption({ label: kind })
    await region
      .getByRole('button', { name: 'Add lookup', exact: true })
      .click()
    await region
      .getByRole('textbox', { name: 'Feature tag', exact: true })
      .fill(tag)
    await operands()
    await expect(
      region.getByLabel('Generated feature source', { exact: true }),
      kind,
    ).toContainText(expected)
  }
  await applySource(page, doc)
  const state = await featureState(page, doc)
  expect(state.active_source.features.map((block) => block.tag).sort()).toEqual(
    [
      'ss03',
      'ss04',
      'ss05',
      'ss06',
      'ss07',
      'ss08',
      'ss09',
      'ss10',
      'ss11',
    ].sort(),
  )
  // The compiled result really substitutes and positions.
  expect(names(await shape(page, doc, 'A', [['ss03', true]]))).toEqual(['A.sc'])
  const positioned = await shape(page, doc, 'A', [['ss08', true]])
  const unpositioned = await shape(page, doc, 'A', [['ss08', false]])
  expect(positioned[0].x_advance).toBe(unpositioned[0].x_advance - 15)
})
