import { createHash } from 'node:crypto'

import { NextResponse } from 'next/server'
import { asc, eq, sql } from 'drizzle-orm'
import { z } from 'zod'

import { db } from '@/server/db'
import { publish_device_release } from '@/server/mqtt'
import { MONO1_IMAGE_FORMAT, render_device_bitmap } from '@/server/preview'
import { current_administrator } from '@/server/session'
import { devices, display_releases } from '@/server/schema'

const document_schema = z.object({ title: z.string().min(1).max(48), subtitle: z.string().max(80).optional(), lines: z.array(z.object({ label: z.string().max(48), value: z.string().max(48) })).max(7).optional() })
const release_schema = z.object({ page_id: z.string().regex(/^[a-z0-9-]{1,64}$/), document: document_schema, device_ids: z.array(z.string().regex(/^[a-z0-9-]{1,64}$/)).min(1) })

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = release_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_release', issues: body.error.issues }, { status: 400 })
  const rendered = render_device_bitmap(body.data.document)
  const content_sha256 = createHash('sha256').update(rendered.device_image).digest('hex')
  const release = await db.transaction(async (transaction) => {
    const [{ next_version }] = await transaction.select({ next_version: sql<number>`coalesce(max(${display_releases.version}), 0) + 1` }).from(display_releases)
    const [created] = await transaction.insert(display_releases).values({
      version: next_version,
      page_id: body.data.page_id,
      document: body.data.document,
      preview_svg: rendered.preview_svg,
      device_image: rendered.device_image,
      image_format: MONO1_IMAGE_FORMAT,
      image_width: 400,
      image_height: 300,
      content_sha256,
    }).returning()
    const targets = await transaction.select({ id: devices.id }).from(devices).where(sql`${devices.id} = ANY(${body.data.device_ids})`)
    if (targets.length !== body.data.device_ids.length) throw new Error('device_not_found')
    await transaction.update(devices).set({ release_id: created.id, active_page_id: body.data.page_id }).where(sql`${devices.id} = ANY(${body.data.device_ids})`)
    return { created, targets }
  })
  const metadata = { id: release.created.id, version: release.created.version, page_id: release.created.page_id, image_format: MONO1_IMAGE_FORMAT, image_width: 400, image_height: 300, image_sha256: content_sha256, image_bytes: rendered.device_image.length }
  const published = await Promise.allSettled(release.targets.map(({ id }) => publish_device_release(id, metadata)))
  const failed_devices = release.targets.filter((_device, index) => published[index].status === 'rejected').map(({ id }) => id)
  return NextResponse.json({ release: metadata, failed_devices }, { status: failed_devices.length ? 202 : 201 })
}

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  return NextResponse.json({ releases: await db.select().from(display_releases).orderBy(asc(display_releases.version)) })
}
