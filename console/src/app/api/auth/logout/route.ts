import { NextResponse } from 'next/server'

import { clear_session } from '@/server/session'

export async function POST() {
  await clear_session()
  return new NextResponse(null, { status: 204 })
}
