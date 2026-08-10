import { NextResponse } from 'next/server'
import { eq } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, display_releases } from '@/server/schema'

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
      content_sha256: display_releases.content_sha256,
      created_at: display_releases.created_at,
    })
    .from(devices)
    .innerJoin(display_releases, eq(devices.release_id, display_releases.id))
    .where(eq(devices.id, device_id))
    .limit(1)

  if (!display) return NextResponse.json({ error: 'display_not_found' }, { status: 404 })
  return NextResponse.json(display)
}
