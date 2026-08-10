import { eq } from 'drizzle-orm'

import { db } from './db'
import { devices, display_releases } from './schema'

export type DeviceSummary = {
  id: string
  name: string
  status: 'enrolling' | 'online' | 'offline' | 'error'
  firmware_version: string | null
  active_page_id: string
  preview_svg: string | null
}

export async function list_devices(): Promise<DeviceSummary[]> {
  if (!db) return []

  const rows = await db
    .select({
      id: devices.id,
      name: devices.name,
      status: devices.status,
      firmware_version: devices.firmware_version,
      active_page_id: devices.active_page_id,
      preview_svg: display_releases.preview_svg,
    })
    .from(devices)
    .leftJoin(display_releases, eq(devices.release_id, display_releases.id))

  return rows
}
