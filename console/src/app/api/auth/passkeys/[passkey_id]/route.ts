import { NextResponse } from 'next/server'
import { and, eq } from 'drizzle-orm'

import { current_administrator } from '@/server/session'
import { db } from '@/server/db'
import { passkeys } from '@/server/schema'

export async function DELETE(_request: Request, { params }: { params: Promise<{ passkey_id: string }> }) {
  const administrator = await current_administrator()
  if (!administrator) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const { passkey_id } = await params
  const [removed] = await db.delete(passkeys).where(and(eq(passkeys.id, passkey_id), eq(passkeys.administrator_id, administrator.id))).returning({ id: passkeys.id })
  if (!removed) return NextResponse.json({ error: 'passkey_not_found' }, { status: 404 })
  return new NextResponse(null, { status: 204 })
}
