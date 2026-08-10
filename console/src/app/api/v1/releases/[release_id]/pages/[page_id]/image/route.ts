import { NextResponse } from 'next/server'
import { and, eq } from 'drizzle-orm'

import { require_api_scope } from '@/server/auth'
import { verify_release_page_image_signature } from '@/server/assets'
import { db } from '@/server/db'
import { display_release_pages } from '@/server/schema'

export async function GET(request: Request, { params }: { params: Promise<{ release_id: string; page_id: string }> }) {
  const { release_id, page_id } = await params
  const url = new URL(request.url)
  const signed_request = verify_release_page_image_signature(release_id, page_id, url.searchParams.get('expires_at'), url.searchParams.get('signature'))
  if (!signed_request && !await require_api_scope(request, 'devices:read')) return new NextResponse('unauthorized', { status: 401 })
  if (!db) return new NextResponse('database_unavailable', { status: 503 })
  const [page] = await db.select({ device_image: display_release_pages.device_image, content_sha256: display_release_pages.content_sha256 })
    .from(display_release_pages)
    .where(and(eq(display_release_pages.release_id, release_id), eq(display_release_pages.page_id, page_id)))
    .limit(1)
  if (!page) return new NextResponse('release_page_not_found', { status: 404 })
  return new NextResponse(page.device_image, {
    headers: {
      'content-type': 'application/vnd.glance-deck.mono1',
      'cache-control': 'public, immutable, max-age=31536000',
      etag: `"${page.content_sha256}"`,
    },
  })
}
