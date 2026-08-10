export const fallback_preview_svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">
  <rect width="400" height="300" fill="#f2f4ed"/>
  <rect x="8" y="8" width="384" height="284" fill="none" stroke="#26322a" stroke-width="4"/>
  <text x="28" y="48" font-family="Noto Sans CJK" font-size="12" font-weight="700" fill="#26322a">GLANCE DECK</text>
  <text x="28" y="95" font-family="Noto Sans CJK" font-size="12" fill="#627168">WAITING FOR PAIRING</text>
  <text x="28" y="157" font-family="Noto Sans CJK" font-size="42" fill="#26322a">—</text>
  <text x="28" y="204" font-family="Noto Sans CJK" font-size="13" fill="#627168">Pair a device, then publish</text>
  <text x="28" y="224" font-family="Noto Sans CJK" font-size="13" fill="#627168">its first display release.</text>
  <text x="28" y="275" font-family="Noto Sans CJK" font-size="10" font-weight="700" fill="#26322a">400 × 300</text>
</svg>`.trim()

export type Display_document = {
  title: string
  subtitle?: string
  lines?: Array<{ label: string; value: string }>
}

export type Rendered_display = {
  device_image: Buffer
  preview_svg: string
}

function escape_xml(value: string) {
  return value.replace(/[<>&"']/g, (character) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' })[character] ?? character)
}

export function render_display_preview(document: Display_document) {
  const title = escape_xml(document.title)
  const subtitle = document.subtitle ? escape_xml(document.subtitle) : ''
  const lines = (document.lines ?? []).slice(0, 7).map((line, index) => {
    const y = 135 + index * 24
    return `<text x="28" y="${y}" font-family="Noto Sans CJK" font-size="13" fill="#627168">${escape_xml(line.label)}</text><text x="372" y="${y}" text-anchor="end" font-family="Noto Sans CJK" font-size="16" font-weight="700" fill="#26322a">${escape_xml(line.value)}</text>`
  }).join('')
  return `<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><rect width="400" height="300" fill="#f2f4ed"/><rect x="8" y="8" width="384" height="284" fill="none" stroke="#26322a" stroke-width="4"/><text x="28" y="48" font-family="Noto Sans CJK" font-size="12" font-weight="700" fill="#26322a">GLANCE DECK</text><text x="28" y="88" font-family="Noto Sans CJK" font-size="27" fill="#26322a">${title}</text><text x="28" y="111" font-family="Noto Sans CJK" font-size="12" fill="#627168">${subtitle}</text>${lines}<line x1="28" x2="372" y1="254" y2="254" stroke="#9ba89f"/><text x="28" y="274" font-family="Noto Sans CJK" font-size="10" fill="#627168">IMMUTABLE DISPLAY RELEASE</text></svg>`
}

/**
 * Rasterizes text with bundled CJK font subsets before publishing it to
 * a device. Firmware receives pixels, not an SVG or a font name, so its CJK
 * support is independent of the ESP32 font catalog.
 */
export function render_device_bitmap(document: Display_document): Rendered_display {
  const preview_svg = render_display_preview(document)
  const pixels = new Resvg(preview_svg, {
    background: '#f2f4ed',
    font: {
      fontDirs: bundled_cjk_font_dirs,
      loadSystemFonts: false,
      sansSerifFamily: 'Noto Sans CJK',
    },
    shapeRendering: 2,
    textRendering: 2,
  }).render().pixels
  const device_image = Buffer.alloc(MONO1_IMAGE_BYTES)

  for (let pixel = 0; pixel < DISPLAY_WIDTH * DISPLAY_HEIGHT; pixel += 1) {
    const offset = pixel * 4
    const luminance = (pixels[offset] * 299 + pixels[offset + 1] * 587 + pixels[offset + 2] * 114) / 1000
    const opaque = pixels[offset + 3] > 127
    if (opaque && luminance < 160) device_image[pixel >> 3] |= 0x80 >> (pixel & 7)
  }
  return { preview_svg, device_image }
}
import { join } from 'node:path'

import { Resvg } from '@resvg/resvg-js'

export const DISPLAY_WIDTH = 400
export const DISPLAY_HEIGHT = 300
export const MONO1_IMAGE_BYTES = DISPLAY_WIDTH * DISPLAY_HEIGHT / 8
export const MONO1_IMAGE_FORMAT = 'mono1-msb'

const bundled_cjk_font_dirs = [
  join(process.cwd(), 'node_modules', '@fontsource', 'noto-sans-sc', 'files'),
  join(process.cwd(), 'node_modules', '@fontsource', 'noto-sans-jp', 'files'),
]
