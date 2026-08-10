import { NextResponse } from 'next/server'
import { z } from 'zod'

import { db } from '@/server/db'
import { current_administrator } from '@/server/session'
import { display_bindings } from '@/server/schema'

const document_schema = z.object({
  title: z.string().min(1).max(48),
  subtitle: z.string().max(80).optional(),
  lines: z.array(z.object({ label: z.string().max(48), value: z.string().max(48) })).max(7).optional(),
})
const binding_schema = z.object({
  source_id: z.uuid(),
  page_id: z.string().regex(/^[a-z0-9-]{1,64}$/),
  document_template: document_schema,
  device_ids: z.array(z.string().regex(/^[a-z0-9-]{1,64}$/)).min(1),
})

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  return NextResponse.json({ bindings: await db.select().from(display_bindings) })
}

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = binding_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_display_binding', issues: body.error.issues }, { status: 400 })
  const [binding] = await db.insert(display_bindings).values(body.data).returning()
  return NextResponse.json({ binding }, { status: 201 })
}
