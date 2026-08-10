import { NextResponse } from 'next/server'
import type { RegistrationResponseJSON } from '@simplewebauthn/types'
import { z } from 'zod'

import { current_administrator } from '@/server/session'
import { finish_passkey_registration } from '@/server/webauthn'

const response_schema = z.object({ id: z.string(), rawId: z.string(), response: z.object({ clientDataJSON: z.string(), attestationObject: z.string(), transports: z.array(z.string()).optional() }), type: z.literal('public-key'), clientExtensionResults: z.record(z.string(), z.unknown()) })

export async function POST(request: Request) {
  const administrator = await current_administrator()
  if (!administrator) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  const response = response_schema.safeParse(await request.json())
  if (!response.success) return NextResponse.json({ error: 'invalid_response' }, { status: 400 })

  try {
    await finish_passkey_registration(administrator.id, response.data as RegistrationResponseJSON)
    return NextResponse.json({ verified: true }, { status: 201 })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'verification_failed' }, { status: 400 })
  }
}
