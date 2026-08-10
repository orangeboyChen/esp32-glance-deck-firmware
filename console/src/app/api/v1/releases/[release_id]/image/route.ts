import { NextResponse } from 'next/server'
import { eq } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { verify_release_image_signature } from '@/server/assets'
import { db } from '@/server/db'
import { display_releases } from '@/server/schema'

export async function GET(request: Request, { params }: { params: Promise<{ release_id: string }> }) {
  const { release_id } = await params
  const url = new URL(request.url)
  const signed_request = verify_release_image_signature(release_id, url.searchParams.get('expires_at'), url.searchParams.get('signature'))
  if (!signed_request && !await require_api_scope(request, 'devices:read')) return new NextResponse('unauthorized', { status: 401 })
  if (!db) return new NextResponse('database_unavailable', { status: 503 })
  const [release] = await db.select({ device_image: display_releases.device_image, content_sha256: display_releases.content_sha256 }).from(display_releases).where(eq(display_releases.id, release_id)).limit(1)
  if (!release) return new NextResponse('release_not_found', { status: 404 })
  return new NextResponse(release.device_image, { headers: { 'content-type': 'application/vnd.glance-deck.mono1', 'cache-control': 'public, immutable, max-age=31536000', etag: `"${release.content_sha256}"` } })
}
