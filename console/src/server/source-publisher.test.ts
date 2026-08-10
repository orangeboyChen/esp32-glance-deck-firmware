import { describe, expect, test } from 'bun:test'

import { render_bound_document } from './source-publisher'

describe('bound source document rendering', () => {
  test('interpolates only persisted source values and marks unavailable values', () => {
    expect(render_bound_document({
      title: '{{plan_name}} usage',
      subtitle: 'Resets {{resets_at}}',
      lines: [{ label: 'Today', value: '{{used}} / {{total}} {{unit}}' }, { label: 'Remaining', value: '{{remaining}}' }],
    }, {
      plan_name: 'Pro', used: 72, total: 100, unit: '%', remaining: null, resets_at: 'tomorrow',
    })).toEqual({
      title: 'Pro usage',
      subtitle: 'Resets tomorrow',
      lines: [{ label: 'Today', value: '72 / 100 %' }, { label: 'Remaining', value: '—' }],
    })
  })
})
