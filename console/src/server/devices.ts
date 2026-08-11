import { and, count, desc, eq, gte } from 'drizzle-orm'

import { db } from './db'
import { alert_rules, devices, display_release_pages, display_releases, ota_jobs, source_snapshots, usage_sources } from './schema'

export type DeviceSummary = {
  id: string
  name: string
  board_model: string
  status: 'enrolling' | 'online' | 'offline' | 'error'
  firmware_version: string | null
  active_page_id: string
  wifi_rssi: number | null
  power_source: string | null
  charging: boolean | null
  battery_percent: number | null
  battery_mv: number | null
  power_updated_at: Date | null
  last_seen_at: Date | null
  preview_svg: string | null
  source_values: Record<string, string | number | null> | null
  ota_status: string | null
  ota_job_id: string | null
}

export async function list_devices(): Promise<DeviceSummary[]> {
  const database = db
  if (!database) return []

  const rows = await database
    .select({
      id: devices.id,
      name: devices.name,
      board_model: devices.board_model,
      status: devices.status,
      firmware_version: devices.firmware_version,
      active_page_id: devices.active_page_id,
      wifi_rssi: devices.wifi_rssi,
      power_source: devices.power_source,
      charging: devices.charging,
      battery_percent: devices.battery_percent,
      battery_mv: devices.battery_mv,
      power_updated_at: devices.power_updated_at,
      last_seen_at: devices.last_seen_at,
      preview_svg: display_release_pages.preview_svg,
    })
    .from(devices)
    .leftJoin(display_releases, eq(devices.release_id, display_releases.id))
    .leftJoin(display_release_pages, and(eq(display_release_pages.release_id, display_releases.id), eq(display_release_pages.page_id, devices.active_page_id)))

  return Promise.all(rows.map(async (row) => {
    const [snapshot] = await database.select({ values: source_snapshots.values })
      .from(source_snapshots).innerJoin(usage_sources, eq(source_snapshots.source_id, usage_sources.id))
      .orderBy(desc(source_snapshots.fetched_at)).limit(1)
    const [ota_job] = await database.select({ id: ota_jobs.id, status: ota_jobs.status }).from(ota_jobs).where(eq(ota_jobs.device_id, row.id)).orderBy(desc(ota_jobs.created_at)).limit(1)
    return { ...row, source_values: snapshot?.values ?? null, ota_status: ota_job?.status ?? null, ota_job_id: ota_job?.id ?? null }
  }))
}

export async function dashboard_summary() {
  if (!db) return { active_alerts: 0, source_updates_today: 0 }
  const start_of_day = new Date()
  start_of_day.setHours(0, 0, 0, 0)
  const [[active], [updates]] = await Promise.all([
    db.select({ value: count() }).from(alert_rules).where(eq(alert_rules.active, true)),
    db.select({ value: count() }).from(source_snapshots).where(gte(source_snapshots.fetched_at, start_of_day)),
  ])
  return { active_alerts: active?.value ?? 0, source_updates_today: updates?.value ?? 0 }
}
