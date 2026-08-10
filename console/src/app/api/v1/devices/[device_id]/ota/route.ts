import { NextResponse } from 'next/server'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { device_commands } from '@/server/schema'

export async function POST(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'ota:install')) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  }
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })

  const { device_id } = await params
  const [command] = await db.insert(device_commands).values({
    device_id,
    action: 'start_ota',
    payload: { requested_by: 'api' },
  }).returning()

  return NextResponse.json({ command }, { status: 202 })
}
