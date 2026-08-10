import { createHash } from 'node:crypto'

import { NextResponse } from 'next/server'
import { asc, eq, sql } from 'drizzle-orm'
import { z } from 'zod'

import { db } from '@/server/db'
import { publish_device_release } from '@/server/mqtt'
import { MONO1_IMAGE_FORMAT, render_device_bitmap } from '@/server/preview'
import { current_administrator } from '@/server/session'
import { devices, display_release_pages, display_releases } from '@/server/schema'

const document_schema = z.object({ title: z.string().min(1).max(48), subtitle: z.string().max(80).optional(), lines: z.array(z.object({ label: z.string().max(48), value: z.string().max(48) })).max(7).optional() })
const page_schema = z.object({ page_id: z.string().regex(/^[a-z0-9-]{1,64}$/), document: document_schema })
const release_schema = z.object({
  active_page_id: z.string().regex(/^[a-z0-9-]{1,64}$/),
  pages: z.array(page_schema).min(1).max(10),
  device_ids: z.array(z.string().regex(/^[a-z0-9-]{1,64}$/)).min(1),
}).superRefine((release, context) => {
  if (!release.pages.some((page) => page.page_id === release.active_page_id)) {
    context.addIssue({ code: 'custom', message: 'active_page_not_found', path: ['active_page_id'] })
  }
  const page_ids = new Set(release.pages.map((page) => page.page_id))
  if (page_ids.size !== release.pages.length) context.addIssue({ code: 'custom', message: 'page_ids_must_be_unique', path: ['pages'] })
})

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = release_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_release', issues: body.error.issues }, { status: 400 })
  const rendered_pages = body.data.pages.map((page, position) => {
    const rendered = render_device_bitmap(page.document)
    return { ...page, position, ...rendered, content_sha256: createHash('sha256').update(rendered.device_image).digest('hex') }
  })
  const active_page = rendered_pages.find((page) => page.page_id === body.data.active_page_id)
  if (!active_page) return NextResponse.json({ error: 'invalid_release' }, { status: 400 })
  const release = await db.transaction(async (transaction) => {
    const [{ next_version }] = await transaction.select({ next_version: sql<number>`coalesce(max(${display_releases.version}), 0) + 1` }).from(display_releases)
    const [created] = await transaction.insert(display_releases).values({
      version: next_version,
      page_id: active_page.page_id,
      document: active_page.document,
      preview_svg: active_page.preview_svg,
      device_image: active_page.device_image,
      image_format: MONO1_IMAGE_FORMAT,
      image_width: 400,
      image_height: 300,
      content_sha256: active_page.content_sha256,
    }).returning()
    await transaction.insert(display_release_pages).values(rendered_pages.map((page) => ({
      release_id: created.id,
      page_id: page.page_id,
      position: page.position,
      document: page.document,
      preview_svg: page.preview_svg,
      device_image: page.device_image,
      image_format: MONO1_IMAGE_FORMAT,
      image_width: 400,
      image_height: 300,
      content_sha256: page.content_sha256,
    })))
    const targets = await transaction.select({ id: devices.id }).from(devices).where(sql`${devices.id} = ANY(${body.data.device_ids})`)
    if (targets.length !== body.data.device_ids.length) throw new Error('device_not_found')
    await transaction.update(devices).set({ release_id: created.id, desired_page_id: body.data.active_page_id, enabled_page_ids: rendered_pages.map((page) => page.page_id) }).where(sql`${devices.id} = ANY(${body.data.device_ids})`)
    return { created, targets }
  })
  const metadata = {
    id: release.created.id,
    version: release.created.version,
    active_page_id: body.data.active_page_id,
    pages: rendered_pages.map((page) => ({ page_id: page.page_id, image_format: MONO1_IMAGE_FORMAT, image_width: 400, image_height: 300, image_sha256: page.content_sha256, image_bytes: page.device_image.length })),
  }
  const published = await Promise.allSettled(release.targets.map(({ id }) => publish_device_release(id, metadata)))
  const failed_devices = release.targets.filter((_device, index) => published[index].status === 'rejected').map(({ id }) => id)
  return NextResponse.json({ release: metadata, failed_devices }, { status: failed_devices.length ? 202 : 201 })
}

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  return NextResponse.json({ releases: await db.select().from(display_releases).orderBy(asc(display_releases.version)) })
}
