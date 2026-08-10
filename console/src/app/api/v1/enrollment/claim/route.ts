import { NextResponse } from 'next/server'
import { z } from 'zod'

import { claim_enrollment } from '@/server/enrollment'

const claim_schema = z.object({ pairing_code: z.string().regex(/^\d{6}$/), claim_secret: z.string().regex(/^[a-f0-9]{64}$/) })

export async function POST(request: Request) {
  const body = claim_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_pairing_code' }, { status: 400 })
  try { return NextResponse.json(await claim_enrollment(body.data.pairing_code, body.data.claim_secret)) }
  catch (error) { return NextResponse.json({ error: error instanceof Error ? error.message : 'claim_failed' }, { status: 401 }) }
}
