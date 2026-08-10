import { NextResponse } from 'next/server'
import { z } from 'zod'

import { require_api_scope } from '@/server/auth'
import { get_device_page_configuration, set_device_page_configuration } from '@/server/device-pages'

const page_configuration_schema = z.object({
  enabled_page_ids: z.array(z.string().regex(/^[a-z0-9-]{1,64}$/)).min(1).max(10),
  desired_page_id: z.string().regex(/^[a-z0-9-]{1,64}$/),
})

export async function GET(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'devices:read')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  const { device_id } = await params
  const configuration = await get_device_page_configuration(device_id)
  return configuration ? NextResponse.json(configuration) : NextResponse.json({ error: 'device_release_not_found' }, { status: 404 })
}

export async function PUT(request: Request, { params }: { params: Promise<{ device_id: string }> }) {
  if (!await require_api_scope(request, 'devices:command')) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  const body = page_configuration_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_page_configuration', issues: body.error.issues }, { status: 400 })
  try {
    const { device_id } = await params
    return NextResponse.json(await set_device_page_configuration(device_id, body.data.enabled_page_ids, body.data.desired_page_id))
  } catch (error) {
    const code = error instanceof Error ? error.message : 'page_configuration_failed'
    const status = code === 'database_unavailable' ? 503 : code === 'device_release_not_found' ? 404 : 400
    return NextResponse.json({ error: code }, { status })
  }
}
