import { describe, expect, it } from 'vitest'

import { ApiCallError } from '../transport'

import { errorNotice } from './errors'
import { i18n } from './index'

const t = (key: string, options: { defaultValue: string }) =>
  i18n.t(key, options)

describe('errorNotice', () => {
  it('translates a known kernel code and keeps the raw message as detail', () => {
    const notice = errorNotice(
      new ApiCallError('not_found', 'no glyph with handle 7'),
      t,
    )

    expect(notice.message).toBe(i18n.t('errors.not_found'))
    expect(notice.message).not.toBe('no glyph with handle 7')
    expect(notice.details).toBe('no glyph with handle 7')
  })

  it('falls back to the raw message for a code it has no key for', () => {
    const notice = errorNotice(
      new ApiCallError('brand_new_code', 'the kernel grew a new failure'),
      t,
    )

    expect(notice.message).toBe('the kernel grew a new failure')
    expect(notice.details).toBeNull()
  })

  it('reports a plain error with its own message and no detail', () => {
    const notice = errorNotice(new Error('the network went away'), t)

    expect(notice.message).toBe('the network went away')
    expect(notice.details).toBeNull()
  })

  it('stringifies a thrown value that is not an error at all', () => {
    expect(errorNotice('nope', t)).toEqual({ message: 'nope', details: null })
  })

  it('has a key for every code the kernel and the transports produce', () => {
    const codes = [
      'not_found',
      'invalid_params',
      'io',
      'parse',
      'internal',
      'geometry',
      'export',
      'shaping',
      'invalid_response',
    ]
    for (const code of codes)
      expect(
        errorNotice(new ApiCallError(code, `raw ${code}`), t).details,
      ).toBe(`raw ${code}`)
  })
})
