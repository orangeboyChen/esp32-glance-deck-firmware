import { NextResponse } from 'next/server'
import { desc } from 'drizzle-orm'
import { z } from 'zod'

import { db } from '@/server/db'
import { current_administrator } from '@/server/session'
import { firmware_releases } from '@/server/schema'
import { verify_firmware_manifest } from '@/server/firmware-signature'

const release_schema = z.object({ version: z.string().min(1).max(64), board_model: z.literal('ESP32-S3-RLCD-4.2'), channel: z.enum(['stable', 'beta', 'test']).default('stable'), manifest_url: z.url().refine((url) => url.startsWith('https://')), image_url: z.url().refine((url) => url.startsWith('https://')), image_sha256: z.string().regex(/^[a-fA-F0-9]{64}$/), manifest_signature: z.string().regex(/^[a-fA-F0-9]{128}$/) })

export async function GET() {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  return NextResponse.json({ releases: await db.select().from(firmware_releases).orderBy(desc(firmware_releases.created_at)) })
}

export async function POST(request: Request) {
  if (!await current_administrator()) return NextResponse.json({ error: 'unauthorized' }, { status: 401 })
  if (!db) return NextResponse.json({ error: 'database_unavailable' }, { status: 503 })
  const body = release_schema.safeParse(await request.json())
  if (!body.success) return NextResponse.json({ error: 'invalid_release', issues: body.error.issues }, { status: 400 })
  try {
    if (!verify_firmware_manifest(body.data)) return NextResponse.json({ error: 'firmware_signature_invalid' }, { status: 400 })
  } catch (error) {
    return NextResponse.json({ error: error instanceof Error ? error.message : 'firmware_signature_invalid' }, { status: 503 })
  }
  const [release] = await db.insert(firmware_releases).values(body.data).returning()
  return NextResponse.json({ release }, { status: 201 })
}
