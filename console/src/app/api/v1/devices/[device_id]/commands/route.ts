import { NextResponse } from 'next/server'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { device_commands } from '@/server/schema'

const command_schema = z.object({
  action: z.enum(['show_page', 'next_page', 'previous_page', 'set_rotation', 'refresh_release', 'enter_maintenance']),
  payload: z.record(z.string(), z.unknown()).default({}),
})

export async function POST(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'devices:command')) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  }
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })

  const body = command_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_command', issues: body.error.issues }, { status: 400 })

  const { device_id } = await params
  const [command] = await db.insert(device_commands).values({
    device_id,
    action: body.data.action,
    payload: body.data.payload,
  }).returning()

  return NextResponse.json({ command }, { status: 202 })
}
