import { NextResponse } from 'next/server'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, firmware_releases, ota_jobs } from '@/server/schema'
import { create_ota_nonce } from '@/server/ota'
import { and, desc, eq } from 'drizzle-orm'
import { z } from 'zod'

const ota_schema = z.object({ firmware_release_id: z.uuid().optional() })

export async function POST(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'ota:install')) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  }
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })

  const body = ota_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_ota_request' }, { status: 400 })
  const { device_id } = await params
  const [device] = await db.select().from(devices).where(eq(devices.id, device_id)).limit(1)
  if (!device) return NextResponse.json({ error: 'device_or_release_not_found' }, { status: 404 })
  const release_query = body.data.firmware_release_id
    ? db.select().from(firmware_releases).where(eq(firmware_releases.id, body.data.firmware_release_id)).limit(1)
    : db.select().from(firmware_releases)
      .where(and(eq(firmware_releases.channel, 'stable'), eq(firmware_releases.board_model, device.board_model)))
      .orderBy(desc(firmware_releases.created_at))
      .limit(1)
  const [release] = await release_query
  if (!release) return NextResponse.json({ error: 'device_or_release_not_found' }, { status: 404 })
  if (release.board_model !== device.board_model) return NextResponse.json({ error: 'incompatible_release' }, { status: 409 })
  const [duplicate] = await db.select({ id: ota_jobs.id }).from(ota_jobs)
    .where(and(eq(ota_jobs.device_id, device_id), eq(ota_jobs.firmware_release_id, release.id), eq(ota_jobs.status, 'queued'))).limit(1)
  if (duplicate) return NextResponse.json({ error: 'ota_already_queued', job: duplicate }, { status: 409 })
  const has_external_power = device.power_source === 'usb' || device.power_source === 'usb_and_battery'
  if (!has_external_power && (device.battery_percent === null || device.battery_percent === undefined || device.battery_percent < 30)) {
    return NextResponse.json({ error: 'power_unsafe_for_ota' }, { status: 409 })
  }
  const [previous_release] = device.firmware_version
    ? await db.select({ id: firmware_releases.id }).from(firmware_releases).where(and(eq(firmware_releases.board_model, device.board_model), eq(firmware_releases.version, device.firmware_version))).limit(1)
    : []
  if (previous_release) await db.update(devices).set({ last_good_firmware_release_id: previous_release.id }).where(eq(devices.id, device_id))
  const [job] = await db.insert(ota_jobs).values({ device_id, firmware_release_id: release.id, nonce: create_ota_nonce() }).returning()

  return NextResponse.json({ job }, { status: 202 })
}
