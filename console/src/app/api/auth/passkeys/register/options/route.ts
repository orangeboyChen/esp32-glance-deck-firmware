import { NextResponse } from 'next/server'

import { current_administrator } from '@/server/session'
import { begin_passkey_registration } from '@/server/webauthn'

export async function POST() {
  const administrator = await current_administrator()
  if (!administrator) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  return NextResponse.json(await begin_passkey_registration(administrator))
}
