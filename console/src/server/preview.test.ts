import { describe, expect, test } from 'bun:test'

import { fallback_preview_svg } from './preview'

describe('fallback preview', () => {
  test('uses the physical display dimensions', () => {
    expect(fallback_preview_svg).toContain('width="300"')
    expect(fallback_preview_svg).toContain('height="400"')
  })
})
