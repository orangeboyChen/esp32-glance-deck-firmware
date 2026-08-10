import { describe, expect, test } from 'bun:test'

import { DISPLAY_HEIGHT, DISPLAY_WIDTH, MONO1_IMAGE_BYTES, fallback_preview_svg, render_device_bitmap, render_display_preview } from './preview'

describe('fallback preview', () => {
  test('uses the physical display dimensions', () => {
    expect(fallback_preview_svg).toContain('width="400"')
    expect(fallback_preview_svg).toContain('height="300"')
  })

  test('renders escaped display document content at the physical size', () => {
    const svg = render_display_preview({ title: 'Usage <today>', subtitle: 'Subscription', lines: [{ label: 'Today', value: '72%' }] })
    expect(svg).toContain('width="400"')
    expect(svg).toContain('height="300"')
    expect(svg).toContain('Usage &lt;today&gt;')
    expect(svg).toContain('Today')
    expect(svg).toContain('72%')
  })

  test('rasterizes Chinese text into a fixed-size firmware bitmap', () => {
    const rendered = render_device_bitmap({ title: '今日用量', subtitle: '订阅窗口', lines: [{ label: '剩余时间', value: '2 小时' }] })
    expect(rendered.device_image).toHaveLength(MONO1_IMAGE_BYTES)
    expect(rendered.device_image.some((byte) => byte !== 0)).toBe(true)
    expect(rendered.preview_svg).toContain('今日用量')
    expect(DISPLAY_WIDTH * DISPLAY_HEIGHT / 8).toBe(MONO1_IMAGE_BYTES)
  })

  test('rasterizes Japanese text into a fixed-size firmware bitmap', () => {
    const rendered = render_device_bitmap({ title: '今日の使用量', subtitle: 'サブスクリプション', lines: [{ label: '残り時間', value: '2 時間' }] })
    expect(rendered.device_image).toHaveLength(MONO1_IMAGE_BYTES)
    expect(rendered.device_image.some((byte) => byte !== 0)).toBe(true)
    expect(rendered.preview_svg).toContain('今日の使用量')
  })
})
