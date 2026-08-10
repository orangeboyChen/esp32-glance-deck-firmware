import { NextResponse } from 'next/server'
import { z } from 'zod'

import { approve_enrollment } from '@/server/enrollment'
import { current_administrator } from '@/server/session'

const enrollment_schema = z.object({ name: z.string().min(1).max(128), pairing_code: z.string().regex(/^\d{6}$/), board_model: z.literal('ESP32-S3-RLCD-4.2') })

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  const body = enrollment_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_enrollment' }, { status: 400 })
  try { return NextResponse.json(await approve_enrollment(body.data.name, body.data.pairing_code, body.data.board_model), { status: 201 }) }
  catch (error) { return NextResponse.json({ error: error instanceof Error ? error.message : 'enrollment_failed' }, { status: 503 }) }
}
