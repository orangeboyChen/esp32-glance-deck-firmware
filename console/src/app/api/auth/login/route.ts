import { NextResponse } from 'next/server'
import { z } from 'zod'

import { authenticate_administrator, create_session } from '@/server/session'

const login_schema = z.object({
  email: z.email(),
  password: z.string().min(1),
})

export async function POST(request: Request) {
  const body = login_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_login' }, { status: 400 })

  const administrator = await authenticate_administrator(body.data.email, body.data.password)
  if (!administrator) return NextResponse.json({ error: 'invalid_credentials' }, { status: 401 })

  await create_session(administrator.id)
  return NextResponse.json({ administrator: { id: administrator.id, email: administrator.email } })
}
