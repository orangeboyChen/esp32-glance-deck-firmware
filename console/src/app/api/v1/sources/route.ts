import { NextResponse } from 'next/server'
import { desc } from 'drizzle-orm'
import { z } from 'zod'

import { db } from '@/server/db'
import { encrypt_secret } from '@/server/secrets'
import { current_administrator } from '@/server/session'
import { usage_sources } from '@/server/schema'

const source_schema = z.object({
  name: z.string().min(1).max(128), base_url: z.url(), request_path: z.string().min(1),
  method: z.enum(['GET', 'POST']).default('GET'), headers: z.record(z.string(), z.string()).default({}),
  body_template: z.string().max(8192).optional(), secrets: z.record(z.string().regex(/^[A-Za-z][A-Za-z0-9_]*$/), z.string()).default({}),
  mapper: z.record(z.string(), z.string().max(256)).refine((mapper) => Object.keys(mapper).every((key) => ['plan_name', 'used', 'remaining', 'total', 'unit', 'resets_at', 'status'].includes(key))),
  refresh_interval_seconds: z.number().int().min(60).max(86_400).default(900),
})

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const sources = await db.select({ id: usage_sources.id, name: usage_sources.name, base_url: usage_sources.base_url, request_path: usage_sources.request_path, method: usage_sources.method, mapper: usage_sources.mapper, refresh_interval_seconds: usage_sources.refresh_interval_seconds, status: usage_sources.status, last_success_at: usage_sources.last_success_at, last_error: usage_sources.last_error }).from(usage_sources).orderBy(desc(usage_sources.created_at))
  return NextResponse.json({ sources })
}

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = source_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_source', issues: body.error.issues }, { status: 400 })
  try {
    const [source] = await db.insert(usage_sources).values({ ...body.data, secret_ciphertext: encrypt_secret(body.data.secrets) }).returning({ id: usage_sources.id, name: usage_sources.name })
    return NextResponse.json({ source }, { status: 201 })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'source_create_failed' }, { status: 400 })
  }
}
