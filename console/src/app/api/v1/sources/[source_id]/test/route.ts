import { NextResponse } from 'next/server'

import { current_administrator } from '@/server/session'
import { refresh_usage_source } from '@/server/usage-source'

export async function POST(_request: Request, { params }: { params: Promise<{ source_id: string }> }) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  try {
    return NextResponse.json({ values: await refresh_usage_source((await params).source_id) })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'source_test_failed' }, { status: 400 })
  }
}
