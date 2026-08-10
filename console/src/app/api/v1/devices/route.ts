import { NextResponse } from 'next/server'

import { require_api_scope } from '@/server/auth'
import { list_devices } from '@/server/devices'

export async function GET(request: Request) {
  if (!await require_api_scope(request, 'devices:read')) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  }

  return NextResponse.json({ devices: await list_devices() })
}
