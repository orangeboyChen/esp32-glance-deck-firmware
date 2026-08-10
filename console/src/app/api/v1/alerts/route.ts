import { desc, inArray } from 'drizzle-orm'
import { NextResponse } from 'next/server'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { db } from '@/server/db'
import { alert_rules, devices } from '@/server/schema'
import { current_administrator } from '@/server/session'

const alert_schema = z.object({
  name: z.string().trim().min(1).max(128), source_id: z.uuid(),
  field: z.enum(['plan_name', 'used', 'remaining', 'total', 'unit', 'resets_at', 'status']),
  operator: z.enum(['gt', 'gte', 'lt', 'lte', 'eq', 'neq', 'contains']), threshold: z.string().trim().min(1).max(128),
  severity: z.enum(['info', 'warning', 'critical']).default('warning'), message: z.string().trim().min(1).max(256),
  device_ids: z.array(z.string().regex(/^[A-Za-z0-9_-]{1,64}$/)).min(1).max(50),
  page_ids: z.array(z.string().regex(/^[a-z0-9-]{1,64}$/)).min(1).max(10),
  enabled: z.boolean().default(true), test_only: z.boolean().default(false),
})

export async function GET(request: Request) {
  if (!await require_api_scope(request, 'alerts:read')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const rules = await db.select().from(alert_rules).orderBy(desc(alert_rules.created_at))
  return NextResponse.json({ rules, active: rules.filter((rule) => rule.active) })
}

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = alert_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_alert_rule', issues: body.error.issues }, { status: 400 })
  const existing_devices = await db.select({ id: devices.id }).from(devices).where(inArray(devices.id, body.data.device_ids))
  if (existing_devices.length !== body.data.device_ids.length) return NextResponse.json({ error: 'alert_device_not_found' }, { status: 400 })
  const [rule] = await db.insert(alert_rules).values(body.data).returning()
  return NextResponse.json({ rule }, { status: 201 })
}
