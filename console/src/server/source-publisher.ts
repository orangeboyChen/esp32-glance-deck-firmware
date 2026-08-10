import { createHash } from 'node:crypto'

import { eq, sql } from 'drizzle-orm'

import { db } from './db'
import { publish_device_release } from './mqtt'
import { MONO1_IMAGE_FORMAT, render_device_bitmap, type Display_document } from './preview'
import { devices, display_bindings, display_release_pages, display_releases } from './schema'

type SourceValues = Record<string, string | number | null>

export function template_value(value: string, values: SourceValues) {
  return value.replace(/\{\{([a-z_]+)\}\}/g, (_match, key: string) => String(values[key] ?? '—'))
}

export function render_bound_document(template: Display_document, values: SourceValues): Display_document {
  return {
    title: template_value(template.title, values),
    subtitle: template.subtitle ? template_value(template.subtitle, values) : undefined,
    lines: template.lines?.map((line) => ({ label: template_value(line.label, values), value: template_value(line.value, values) })),
  }
}

export async function publish_source_changes(source_id: string, values: SourceValues) {
  if (!db) return 0
  const bindings = await db.select().from(display_bindings).where(eq(display_bindings.source_id, source_id))
  let published = 0

  for (const binding of bindings) {
    const document = render_bound_document(binding.document_template, values)
    const rendered = render_device_bitmap(document)
    const content_sha256 = createHash('sha256').update(rendered.device_image).digest('hex')
    const assigned = await db.select({ id: devices.id, release_id: devices.release_id })
      .from(devices).where(sql`${devices.id} = ANY(${binding.device_ids})`)
    if (assigned.length !== binding.device_ids.length) throw new Error('display_binding_device_not_found')

    const existing = assigned.length > 0
      ? await db.select({ id: display_releases.id }).from(display_releases)
        .where(sql`${display_releases.id} = ANY(${assigned.map((device) => device.release_id).filter(Boolean)}) AND ${display_releases.content_sha256} = ${content_sha256}`)
        .limit(1)
      : []
    if (existing[0] && assigned.every((device) => device.release_id === existing[0].id)) continue

    const created = await db.transaction(async (transaction) => {
      const [{ next_version }] = await transaction.select({ next_version: sql<number>`coalesce(max(${display_releases.version}), 0) + 1` }).from(display_releases)
      const [release] = await transaction.insert(display_releases).values({
        version: next_version,
        page_id: binding.page_id,
        document,
        preview_svg: rendered.preview_svg,
        device_image: rendered.device_image,
        image_format: MONO1_IMAGE_FORMAT,
        image_width: 400,
        image_height: 300,
        content_sha256,
      }).returning()
      await transaction.insert(display_release_pages).values({
        release_id: release.id,
        page_id: binding.page_id,
        position: 0,
        document,
        preview_svg: rendered.preview_svg,
        device_image: rendered.device_image,
        image_format: MONO1_IMAGE_FORMAT,
        image_width: 400,
        image_height: 300,
        content_sha256,
      })
      await transaction.update(devices).set({ release_id: release.id, desired_page_id: binding.page_id, enabled_page_ids: [binding.page_id] })
        .where(sql`${devices.id} = ANY(${binding.device_ids})`)
      return release
    })
    const base_url = process.env.DEVICE_ASSET_URL ?? process.env.APP_URL
    if (!base_url?.startsWith('https://')) throw new Error('device_asset_url_https_required')
    await Promise.all(binding.device_ids.map((device_id) => publish_device_release(device_id, {
      id: created.id,
      version: created.version,
      active_page_id: created.page_id,
      pages: [{ page_id: created.page_id, image_format: created.image_format, image_width: created.image_width, image_height: created.image_height, image_sha256: created.content_sha256, image_bytes: rendered.device_image.length }],
    })))
    published += binding.device_ids.length
  }
  return published
}
