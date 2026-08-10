import { randomBytes } from 'node:crypto'

import argon2 from 'argon2'
import { and, eq, gt } from 'drizzle-orm'
import { cookies } from 'next/headers'

import { db } from './db'
import { administrators, sessions } from './schema'

const session_cookie_name = 'glance_deck_session'
const session_duration_ms = 1000 * 60 * 60 * 24 * 30

export async function administrator_exists() {
  if (!db) return false
  const [administrator] = await db.select({ id: administrators.id }).from(administrators).limit(1)
  return Boolean(administrator)
}

export async function create_administrator(email: string, password: string) {
  if (!db) throw new Error('database_unavailable')
  if (await administrator_exists()) throw new Error('administrator_exists')

  const password_hash = await argon2.hash(password, { type: argon2.argon2id })
  const [administrator] = await db.insert(administrators).values({ email, password_hash }).returning()
  return administrator
}

export async function authenticate_administrator(email: string, password: string) {
  if (!db) return undefined
  const [administrator] = await db.select().from(administrators).where(eq(administrators.email, email)).limit(1)
  if (!administrator || !await argon2.verify(administrator.password_hash, password)) return undefined
  return administrator
}

export async function create_session(administrator_id: string) {
  if (!db) throw new Error('database_unavailable')
  const token = randomBytes(32).toString('base64url')
  const token_hash = await argon2.hash(token, { type: argon2.argon2id })
  const expires_at = new Date(Date.now() + session_duration_ms)
  await db.insert(sessions).values({ administrator_id, token_hash, expires_at })

  const cookie_store = await cookies()
  cookie_store.set(session_cookie_name, token, {
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    expires: expires_at,
    path: '/',
  })
}

export async function current_administrator() {
  if (!db) return undefined
  const token = (await cookies()).get(session_cookie_name)?.value
  if (!token) return undefined

  const candidates = await db
    .select({ session_id: sessions.id, token_hash: sessions.token_hash, administrator: administrators })
    .from(sessions)
    .innerJoin(administrators, eq(sessions.administrator_id, administrators.id))
    .where(and(gt(sessions.expires_at, new Date())))

  for (const candidate of candidates) {
    if (await argon2.verify(candidate.token_hash, token)) return candidate.administrator
  }
  return undefined
}

export async function clear_session() {
  const cookie_store = await cookies()
  cookie_store.delete(session_cookie_name)
}
