import { eq } from 'drizzle-orm'

import { db } from './db'
import { devices, display_releases } from './schema'

export type DeviceSummary = {
  id: string
  name: string
  status: 'enrolling' | 'online' | 'offline' | 'error'
  firmware_version: string | null
  active_page_id: string
  wifi_rssi: number | null
  last_seen_at: Date | null
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
      wifi_rssi: devices.wifi_rssi,
      last_seen_at: devices.last_seen_at,
      preview_svg: display_releases.preview_svg,
    })
    .from(devices)
    .leftJoin(display_releases, eq(devices.release_id, display_releases.id))

  return rows
}
