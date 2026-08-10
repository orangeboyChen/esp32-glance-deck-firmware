import { describe, expect, test } from 'bun:test'

import { matches_alert } from './alerts'

describe('alert threshold evaluation', () => {
  test('compares numeric values with inclusive boundaries', () => {
    expect(matches_alert(80, 'gte', '80')).toBe(true)
    expect(matches_alert(79, 'gte', '80')).toBe(false)
    expect(matches_alert('12.5', 'lt', '13')).toBe(true)
    expect(matches_alert('not-a-number', 'gt', '1')).toBe(false)
  })

  test('compares text and supports contains', () => {
    expect(matches_alert('Critical quota', 'contains', 'quota')).toBe(true)
    expect(matches_alert('ready', 'eq', 'ready')).toBe(true)
    expect(matches_alert('ready', 'neq', 'ready')).toBe(false)
    expect(matches_alert(null, 'eq', '')).toBe(true)
  })
})
