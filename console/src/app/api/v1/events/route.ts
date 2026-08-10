import { NextResponse } from 'next/server'

import { require_api_scope } from '@/server/auth'

export async function GET(request: Request) {
  if (!await require_api_scope(request, 'devices:read')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode('event: ready\\ndata: {"status":"connected"}\\n\\n'))
      controller.close()
    },
  })
  return new Response(stream, { headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' } })
}
