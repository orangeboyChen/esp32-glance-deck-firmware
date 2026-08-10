import { describe, expect, test } from 'bun:test'

import { derive_usage_metrics } from './usage-stats'

describe('derived usage metrics', () => {
  test('calculates today and week deltas against persisted baselines', () => {
    const now = new Date('2026-08-12T15:00:00')
    const metrics = derive_usage_metrics(
      { used: 72, total: 100 },
      [
        { fetched_at: new Date('2026-08-10T09:00:00'), values: { used: 20 } },
        { fetched_at: new Date('2026-08-11T09:00:00'), values: { used: 40 } },
        { fetched_at: new Date('2026-08-12T09:00:00'), values: { used: 60 } },
      ],
      now,
    )
    expect(metrics).toEqual({ today_used: 12, today_percent: 12, week_used: 52, week_percent: 52 })
  })

  test('returns unavailable metrics when a baseline is missing or usage resets', () => {
    const now = new Date('2026-08-12T15:00:00')
    expect(derive_usage_metrics({ used: 10, total: 100 }, [], now)).toEqual({ today_used: null, today_percent: null, week_used: null, week_percent: null })
    expect(derive_usage_metrics({ used: 10, total: 0 }, [{ fetched_at: new Date('2026-08-12T09:00:00'), values: { used: 20 } }], now)).toEqual({ today_used: null, today_percent: null, week_used: null, week_percent: null })
  })
})
