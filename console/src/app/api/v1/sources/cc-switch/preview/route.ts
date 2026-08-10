import { NextResponse } from 'next/server'

import { preview_cc_switch_import } from '@/server/cc-switch-import'
import { current_administrator } from '@/server/session'

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  try {
    return NextResponse.json({ preview: preview_cc_switch_import(await request.json()) })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'cc_switch_export_invalid' }, { status: 400 })
  }
}
