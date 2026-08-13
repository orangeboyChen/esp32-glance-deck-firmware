import { createRequire } from 'node:module'
import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const script_directory = dirname(fileURLToPath(import.meta.url))
const firmware_directory = resolve(script_directory, '..')
const output_directory = resolve(firmware_directory, 'assets/local-icons')
const console_require = createRequire(resolve(firmware_directory, '../console/package.json'))
const React = console_require('react')
const { renderToStaticMarkup } = console_require('react-dom/server')
const lucide = console_require('lucide-react')
const sharp = console_require('sharp')

const icons = {
  check: lucide.CircleCheck,
  'check-mark': lucide.Check,
  checking: lucide.LoaderCircle,
  download: lucide.ArrowDownToLine,
  error: lucide.TriangleAlert,
  failed: lucide.CircleX,
  maintenance: lucide.Cog,
  pairing: lucide.Link,
  'short-press': lucide.MousePointerClick,
  'long-press': lucide.Timer,
  update: lucide.RefreshCw,
  wifi: lucide.Wifi,
}

await mkdir(output_directory, { recursive: true })
for (const [name, Icon] of Object.entries(icons)) {
  const svg = renderToStaticMarkup(React.createElement(Icon, { color: '#000000', size: 32, strokeWidth: 2 }))
  const { data, info } = await sharp(Buffer.from(svg)).flatten({ background: '#ffffff' }).raw().toBuffer({ resolveWithObject: true })
  if (info.width !== 32 || info.height !== 32 || info.channels < 3) throw new Error(`unexpected ${name} icon dimensions`)
  const packed = Buffer.alloc(128)
  for (let pixel = 0; pixel < 32 * 32; pixel += 1) {
    const offset = pixel * info.channels
    const luminance = (data[offset] * 299 + data[offset + 1] * 587 + data[offset + 2] * 114) / 1000
    if (luminance < 160) packed[pixel >> 3] |= 0x80 >> (pixel & 7)
  }
  await writeFile(resolve(output_directory, `${name}.mono1`), packed)
}
