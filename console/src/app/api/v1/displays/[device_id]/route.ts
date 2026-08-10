import { NextResponse } from 'next/server'
import { desc, eq, sql } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, display_releases, source_snapshots, usage_sources } from '@/server/schema'

export async function GET(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'devices:read')) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  }
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })

  const { device_id } = await params
  const [display] = await db
    .select({
      release_id: display_releases.id,
      version: display_releases.version,
      page_id: display_releases.page_id,
      document: display_releases.document,
      image_format: display_releases.image_format,
      image_width: display_releases.image_width,
      image_height: display_releases.image_height,
      image_bytes: sql<number>`octet_length(${display_releases.device_image})`,
      content_sha256: display_releases.content_sha256,
      created_at: display_releases.created_at,
    })
    .from(devices)
    .innerJoin(display_releases, eq(devices.release_id, display_releases.id))
    .where(eq(devices.id, device_id))
    .limit(1)

  if (!display) return NextResponse.json({ error: 'display_not_found' }, { status: 404 })
  const [snapshot] = await db.select({ values: source_snapshots.values, fetched_at: source_snapshots.fetched_at, source_name: usage_sources.name })
    .from(source_snapshots).innerJoin(usage_sources, eq(source_snapshots.source_id, usage_sources.id)).orderBy(desc(source_snapshots.fetched_at)).limit(1)
  return NextResponse.json({ ...display, source: snapshot ?? null, stale: !snapshot || Date.now() - snapshot.fetched_at.getTime() > 30 * 60 * 1000 })
}
