import { NextResponse } from 'next/server'
import { z } from 'zod'

import { create_api_token, hash_secret } from '@/server/auth'
import { db } from '@/server/db'
import { current_administrator } from '@/server/session'
import { api_tokens } from '@/server/schema'
import { desc, isNull } from 'drizzle-orm'

const token_schema = z.object({
  label: z.string().min(1).max(128),
  scopes: z.array(z.enum(['devices:read', 'devices:command', 'alerts:read', 'ota:install'])).min(1),
})

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = token_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_token_request' }, { status: 400 })

  const token = create_api_token()
  const [record] = await db.insert(api_tokens).values({
    label: body.data.label,
    token_hash: hash_secret(token),
    scopes: body.data.scopes,
  }).returning({ id: api_tokens.id, label: api_tokens.label, scopes: api_tokens.scopes })

  return NextResponse.json({ token, record }, { status: 201 })
}

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const tokens = await db.select({ id: api_tokens.id, label: api_tokens.label, scopes: api_tokens.scopes, created_at: api_tokens.created_at })
    .from(api_tokens).where(isNull(api_tokens.revoked_at)).orderBy(desc(api_tokens.created_at))
  return NextResponse.json({ tokens })
}
