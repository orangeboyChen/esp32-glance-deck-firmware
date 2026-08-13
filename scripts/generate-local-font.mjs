import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { Resvg } from '../../console/node_modules/@resvg/resvg-js/index.js'

const script_directory = dirname(fileURLToPath(import.meta.url))
const firmware_directory = resolve(script_directory, '..')
const font_file = resolve(firmware_directory, '../console/assets/fonts/NotoSansSC-Regular.ttf')
const output_directory = resolve(firmware_directory, 'assets/local-font')
const variants = [
  { scale: 1, font_size: 14, width: 12, height: 20, baseline: 15 },
  { scale: 2, font_size: 18, width: 16, height: 26, baseline: 20 },
  { scale: 3, font_size: 26, width: 22, height: 36, baseline: 28 },
  { scale: 5, font_size: 42, width: 36, height: 56, baseline: 45 },
]

function glyph_svg(character, variant) {
  const escaped = character === '&' ? '&amp;' : character === '<' ? '&lt;' : character
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${variant.width}" height="${variant.height}" viewBox="0 0 ${variant.width} ${variant.height}"><rect width="100%" height="100%" fill="#f2f4ed"/><text x="0" y="${variant.baseline}" font-family="Noto Sans CJK" font-size="${variant.font_size}" fill="#26322a">${escaped}</text></svg>`
}

function glyph_bitmap(character, variant) {
  const pixels = new Resvg(glyph_svg(character, variant), {
    background: '#f2f4ed',
    font: { fontFiles: [font_file], loadSystemFonts: false, sansSerifFamily: 'Noto Sans CJK' },
    shapeRendering: 2,
    textRendering: 2,
  }).render().pixels
  const bitmap = Buffer.alloc(Math.ceil(variant.width * variant.height / 8))
  for (let pixel = 0; pixel < variant.width * variant.height; pixel += 1) {
    const offset = pixel * 4
    const luminance = (pixels[offset] * 299 + pixels[offset + 1] * 587 + pixels[offset + 2] * 114) / 1000
    if (pixels[offset + 3] > 127 && luminance < 160) bitmap[pixel >> 3] |= 0x80 >> (pixel & 7)
  }
  let rightmost = -1
  for (let pixel = 0; pixel < variant.width * variant.height; pixel += 1) {
    if (bitmap[pixel >> 3] & (0x80 >> (pixel & 7))) rightmost = Math.max(rightmost, pixel % variant.width)
  }
  const advance = character === ' ' ? Math.max(3, Math.round(variant.font_size * 0.28)) : Math.min(variant.width, rightmost + 3)
  return { bitmap, advance }
}

await mkdir(output_directory, { recursive: true })
for (const variant of variants) {
  const glyphs = []
  const advances = []
  for (let code = 32; code <= 126; code += 1) {
    const glyph = glyph_bitmap(String.fromCharCode(code), variant)
    glyphs.push(glyph.bitmap)
    advances.push(glyph.advance)
  }
  await writeFile(resolve(output_directory, `noto-sans-sc-${variant.scale}.bin`), Buffer.concat(glyphs))
  await writeFile(resolve(output_directory, `noto-sans-sc-${variant.scale}.widths`), Buffer.from(advances))
}
