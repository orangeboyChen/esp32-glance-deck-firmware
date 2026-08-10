import { eq } from 'drizzle-orm'
import { NextResponse } from 'next/server'

import { db } from '@/server/db'
import { alert_rules } from '@/server/schema'
import { current_administrator } from '@/server/session'

export async function DELETE(_request: Request, { params }: { params: Promise<{ alert_id: string }> }) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const { alert_id } = await params
  const [rule] = await db.update(alert_rules).set({ enabled: false, active: false }).where(eq(alert_rules.id, alert_id)).returning({ id: alert_rules.id })
  return rule ? NextResponse.json({ rule }) : NextResponse.json({ error: 'alert_not_found' }, { status: 404 })
}
