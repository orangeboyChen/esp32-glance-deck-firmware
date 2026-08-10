import { and, eq, inArray } from 'drizzle-orm'
import { NextResponse } from 'next/server'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { devices, firmware_releases, ota_jobs } from '@/server/schema'
import { create_ota_nonce } from '@/server/ota'

const action_schema = z.object({ action: z.enum(['cancel', 'rollback']) }).default({ action: 'cancel' })

export async function PATCH(request: Request, { params }: { params: Promise<{ job_id: string }> }) {
  if (!await require_api_scope(request, 'ota:install')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const { job_id } = await params
  const body = action_schema.safeParse(await request.json().catch(() => ({})))
  if (!body.success) return NextResponse.json({ error: 'invalid_ota_job_action' }, { status: 400 })
  if (body.data.action === 'rollback') {
    const [job] = await db.select({ device_id: ota_jobs.device_id, release_id: devices.last_good_firmware_release_id, power_source: devices.power_source, battery_percent: devices.battery_percent })
      .from(ota_jobs).innerJoin(devices, eq(devices.id, ota_jobs.device_id)).where(eq(ota_jobs.id, job_id)).limit(1)
    if (!job?.release_id) return NextResponse.json({ error: 'known_good_release_not_found' }, { status: 409 })
    if (job.power_source !== 'usb' && job.power_source !== 'usb_and_battery' && (job.battery_percent ?? 0) < 30) return NextResponse.json({ error: 'power_unsafe_for_ota' }, { status: 409 })
    const [rollback] = await db.insert(ota_jobs).values({ device_id: job.device_id, firmware_release_id: job.release_id, nonce: create_ota_nonce() }).returning()
    return NextResponse.json({ job: rollback }, { status: 202 })
  }
  const [job] = await db.update(ota_jobs).set({ status: 'cancelled', completed_at: new Date(), error_message: 'cancelled_by_operator' })
    .where(and(eq(ota_jobs.id, job_id), inArray(ota_jobs.status, ['queued', 'awaiting_confirmation']))).returning()
  return job ? NextResponse.json({ job }) : NextResponse.json({ error: 'job_not_cancellable' }, { status: 409 })
}
