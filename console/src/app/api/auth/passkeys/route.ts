import { NextResponse } from 'next/server'
import { eq } from 'drizzle-orm'

import { current_administrator } from '@/server/session'
import { db } from '@/server/db'
import { passkeys } from '@/server/schema'

export async function GET() {
  const administrator = await current_administrator()
  if (!administrator) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const records = await db.select({ id: passkeys.id, created_at: passkeys.created_at, transports: passkeys.transports })
    .from(passkeys).where(eq(passkeys.administrator_id, administrator.id)).orderBy(passkeys.created_at)
  return NextResponse.json({ passkeys: records })
}
