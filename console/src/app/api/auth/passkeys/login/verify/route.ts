import { NextResponse } from 'next/server'
import type { AuthenticationResponseJSON } from '@simplewebauthn/types'
import { z } from 'zod'

import { create_session } from '@/server/session'
import { finish_passkey_authentication } from '@/server/webauthn'

const response_schema = z.object({
  id: z.string(), rawId: z.string(), type: z.literal('public-key'),
  response: z.object({ clientDataJSON: z.string(), authenticatorData: z.string(), signature: z.string(), userHandle: z.string().optional() }),
  clientExtensionResults: z.record(z.string(), z.unknown()),
})

export async function POST(request: Request) {
  const response = response_schema.safeParse(await request.json())
  if (!response.success) return NextResponse.json({ error: 'invalid_response' }, { status: 400 })
  try {
    const administrator_id = await finish_passkey_authentication(response.data as AuthenticationResponseJSON)
    await create_session(administrator_id)
    return NextResponse.json({ verified: true })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'verification_failed' }, { status: 401 })
  }
}
