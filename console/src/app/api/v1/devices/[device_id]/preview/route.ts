import { NextResponse } from 'next/server'
import { and, eq } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, display_release_pages, display_releases } from '@/server/schema'
import { fallback_preview_svg } from '@/server/preview'

export async function GET(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'devices:read')) {
    return new NextResponse('unauthorized', { status: 401 })
  }

  const { device_id } = await params
  const [row] = db ? await db
    .select({ preview_svg: display_release_pages.preview_svg })
    .from(devices)
    .leftJoin(display_releases, eq(devices.release_id, display_releases.id))
    .leftJoin(display_release_pages, and(eq(display_release_pages.release_id, display_releases.id), eq(display_release_pages.page_id, devices.active_page_id)))
    .where(eq(devices.id, device_id))
    .limit(1) : []

  return new NextResponse(row?.preview_svg ?? fallback_preview_svg, {
    headers: { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' },
  })
}
