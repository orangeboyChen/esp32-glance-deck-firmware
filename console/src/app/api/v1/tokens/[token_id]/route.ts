import { NextResponse } from 'next/server'
import { and, eq, isNull } from 'drizzle-orm'

import { db } from '@/server/db'
import { current_administrator } from '@/server/session'
import { api_tokens } from '@/server/schema'

export async function DELETE(_request: Request, { params }: { params: Promise<{ token_id: string }> }) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const { token_id } = await params
  const [token] = await db.update(api_tokens).set({ revoked_at: new Date() })
    .where(and(eq(api_tokens.id, token_id), isNull(api_tokens.revoked_at))).returning({ id: api_tokens.id })
  if (!token) return NextResponse.json({ error: 'token_not_found' }, { status: 404 })
  return new NextResponse(null, { status: 204 })
}
