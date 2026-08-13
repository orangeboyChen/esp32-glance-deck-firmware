import { mkdir, readFile, readdir, unlink, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

import sharp from '../../console/node_modules/sharp/lib/index.js'

const exec_file = promisify(execFile)
const script_directory = dirname(fileURLToPath(import.meta.url))
const firmware_directory = resolve(script_directory, '..')
const output_directory = resolve(firmware_directory, '../docs/image')
const temporary_directory = resolve('/private/tmp', 'glance-deck-local-screens')
const width = 400
const height = 300

await mkdir(temporary_directory, { recursive: true })
await exec_file('cargo', ['+stable', 'run', '--quiet', '--target', 'aarch64-apple-darwin', '--bin', 'render-local-screens', '--', temporary_directory], {
  cwd: firmware_directory,
})

for (const file of await readdir(temporary_directory)) {
  if (!file.endsWith('.mono1')) continue
  const packed = await readFile(resolve(temporary_directory, file))
  if (packed.length !== width * height / 8) throw new Error(`invalid frame: ${file}`)
  const pixels = Buffer.alloc(width * height)
  for (let pixel = 0; pixel < pixels.length; pixel += 1) {
    pixels[pixel] = packed[pixel >> 3] & (0x80 >> (pixel & 7)) ? 38 : 242
  }
  const png = await sharp(pixels, { raw: { width, height, channels: 1 } }).png().toBuffer()
  await writeFile(resolve(output_directory, `${file.slice(0, -6)}.png`), png)
  await unlink(resolve(temporary_directory, file))
}
