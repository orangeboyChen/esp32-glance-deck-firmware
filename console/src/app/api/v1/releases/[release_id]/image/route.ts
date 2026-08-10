import { NextResponse } from 'next/server'
import { eq } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { display_releases } from '@/server/schema'

export async function GET(request: Request, { params }: { params: Promise<{ release_id: string }> }) {
  if (!await require_api_scope(request, 'devices:read')) return new NextResponse('unauthorized', { status: 401 })
  if (!db) return new NextResponse('database_unavailable', { status: 503 })
  const [release] = await db.select({ preview_svg: display_releases.preview_svg, content_sha256: display_releases.content_sha256 }).from(display_releases).where(eq(display_releases.id, (await params).release_id)).limit(1)
  if (!release) return new NextResponse('release_not_found', { status: 404 })
  return new NextResponse(release.preview_svg, { headers: { 'content-type': 'image/svg+xml', 'cache-control': 'public, immutable, max-age=31536000', etag: `"${release.content_sha256}"` } })
}
