import { and, eq, inArray } from 'drizzle-orm'
import { NextResponse } from 'next/server'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, firmware_releases, ota_jobs } from '@/server/schema'
import { create_ota_nonce } from '@/server/ota'

const rollout_schema = z.object({
  firmware_release_id: z.uuid(), device_ids: z.array(z.string().regex(/^[A-Za-z0-9_-]{1,64}$/)).min(1).max(100),
  percentage: z.number().int().min(1).max(100).default(100),
})

export async function POST(request: Request) {
  if (!await require_api_scope(request, 'ota:install')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = rollout_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_rollout', issues: body.error.issues }, { status: 400 })
  const [release] = await db.select().from(firmware_releases).where(eq(firmware_releases.id, body.data.firmware_release_id)).limit(1)
  const candidates = await db.select({ id: devices.id, board_model: devices.board_model, power_source: devices.power_source, battery_percent: devices.battery_percent })
    .from(devices).where(inArray(devices.id, body.data.device_ids))
  if (!release) return NextResponse.json({ error: 'firmware_release_not_found' }, { status: 404 })
  if (candidates.length !== body.data.device_ids.length) return NextResponse.json({ error: 'device_not_found' }, { status: 404 })
  const eligible = candidates.filter((device) => device.board_model === release.board_model && (device.power_source === 'usb' || device.power_source === 'usb_and_battery' || (device.battery_percent ?? 0) >= 30))
  const target_count = Math.max(1, Math.ceil(eligible.length * body.data.percentage / 100))
  const selected = eligible.slice(0, target_count)
  if (!selected.length) return NextResponse.json({ error: 'no_power_safe_devices' }, { status: 409 })
  const jobs = await db.insert(ota_jobs).values(selected.map((device) => ({ device_id: device.id, firmware_release_id: release.id, nonce: create_ota_nonce() }))).returning()
  return NextResponse.json({ jobs, selected_count: selected.length, eligible_count: eligible.length }, { status: 202 })
}
