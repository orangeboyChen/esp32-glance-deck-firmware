import { NextResponse } from 'next/server'

import { begin_passkey_authentication } from '@/server/webauthn'

export async function POST() {
  try {
    return NextResponse.json(await begin_passkey_authentication())
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'authentication_unavailable' }, { status: 503 })
  }
}
