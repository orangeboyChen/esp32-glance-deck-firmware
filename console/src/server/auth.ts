import { createHash, randomBytes } from 'node:crypto'

import { and, eq, isNull } from 'drizzle-orm'

import { db } from './db'
import { current_administrator } from './session'
import { api_tokens } from './schema'

export function hash_secret(secret: string) {
  return createHash('sha256').update(secret).digest('hex')
}

export function create_api_token() {
  return `gld_${randomBytes(32).toString('base64url')}`
}

export async function require_api_scope(request: Request, required_scope: string) {
  if (await current_administrator()) return true
  const header = request.headers.get('authorization')
  const token = header?.startsWith('Bearer ') ? header.slice(7) : undefined

  if (!token || !db) return false

  const [api_token] = await db
    .select()
    .from(api_tokens)
    .where(and(eq(api_tokens.token_hash, hash_secret(token)), isNull(api_tokens.revoked_at)))
    .limit(1)

  return Boolean(api_token?.scopes.includes(required_scope))
}
