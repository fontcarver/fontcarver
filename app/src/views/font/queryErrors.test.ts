import { beforeEach, expect, it } from 'vitest'

import { ApiCallError } from '../../transport'

import {
  classifyQueryError,
  selectQueryError,
  useQueryErrors,
} from './queryErrors'

beforeEach(() => {
  useQueryErrors.setState({ byDoc: {} })
})

it('recognises a rejected search and a rejected smart expression', () => {
  expect(
    classifyQueryError(
      new ApiCallError(
        'invalid_params',
        'invalid glyph search regex: unclosed group',
      ),
    ),
  ).toEqual({
    kind: 'search',
    message: 'invalid glyph search regex: unclosed group',
  })
  expect(
    classifyQueryError(
      new ApiCallError('invalid_params', 'unknown smart-filter field `nope`'),
    ),
  ).toMatchObject({ kind: 'smartFilter' })
  // Anything else stays a transport failure for the toast to report.
  expect(
    classifyQueryError(new ApiCallError('io', 'invalid glyph search regex')),
  ).toBeNull()
  expect(
    classifyQueryError(
      new ApiCallError('invalid_params', 'master `x` is gone'),
    ),
  ).toBeNull()
  expect(classifyQueryError(new Error('offline'))).toBeNull()
})

it('reports per document and clears when a query succeeds', () => {
  const store = useQueryErrors.getState()
  expect(store.report(1, new Error('offline'))).toBe(false)
  expect(
    store.report(
      1,
      new ApiCallError('invalid_params', 'invalid glyph search regex: ['),
    ),
  ).toBe(true)
  expect(selectQueryError(1, 'search')(useQueryErrors.getState())).toBe(
    'invalid glyph search regex: [',
  )
  // The same error again is idempotent, and other kinds stay empty.
  expect(
    store.report(
      1,
      new ApiCallError('invalid_params', 'invalid glyph search regex: ['),
    ),
  ).toBe(true)
  expect(
    selectQueryError(1, 'smartFilter')(useQueryErrors.getState()),
  ).toBeNull()
  expect(selectQueryError(2, 'search')(useQueryErrors.getState())).toBeNull()

  useQueryErrors.getState().clear(1)
  expect(selectQueryError(1, 'search')(useQueryErrors.getState())).toBeNull()
})
