import { NextResponse } from 'next/server'
import { eq } from 'drizzle-orm'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { validate_device_page_command } from '@/server/device-pages'
import { device_commands, devices } from '@/server/schema'

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
  const [device] = await db.select({ id: devices.id }).from(devices).where(eq(devices.id, device_id)).limit(1)
  if (!device) return NextResponse.json({ error: 'device_not_found' }, { status: 404 })
  try {
    await validate_device_page_command(device_id, body.data.action, body.data.payload)
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'invalid_command' }, { status: 400 })
  }
  const [command] = await db.insert(device_commands).values({
    device_id,
    action: body.data.action,
    payload: body.data.payload,
  }).returning()

  if (body.data.action === 'show_page') {
    await db.update(devices).set({ desired_page_id: body.data.payload.page_id as string }).where(eq(devices.id, device_id))
  }

  return NextResponse.json({ command }, { status: 202 })
}
