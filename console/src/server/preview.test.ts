import { describe, expect, test } from 'bun:test'

import { fallback_preview_svg, render_display_preview } from './preview'

describe('fallback preview', () => {
  test('uses the physical display dimensions', () => {
    expect(fallback_preview_svg).toContain('width="300"')
    expect(fallback_preview_svg).toContain('height="400"')
  })

  test('renders escaped display document content at the physical size', () => {
    const svg = render_display_preview({ title: 'Usage <today>', subtitle: 'Subscription', lines: [{ label: 'Today', value: '72%' }] })
    expect(svg).toContain('width="300"')
    expect(svg).toContain('height="400"')
    expect(svg).toContain('Usage &lt;today&gt;')
    expect(svg).toContain('Today')
    expect(svg).toContain('72%')
  })
})
