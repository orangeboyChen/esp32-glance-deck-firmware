import { asc, eq, sql } from 'drizzle-orm'

import { db } from './db'
import { publish_device_release, type ReleasePageMetadata } from './mqtt'
import { devices, display_release_pages, display_releases } from './schema'

export type DevicePageConfiguration = {
  device_id: string
  release_id: string
  release_version: number
  /** Last page rendered and reported by the device. */
  active_page_id: string
  /** Console-selected page to render next. */
  desired_page_id: string
  enabled_page_ids: string[]
  pages: ReleasePageMetadata[]
  available_pages: ReleasePageMetadata[]
}

type DeviceRelease = {
  device_id: string
  release_id: string
  release_version: number
  active_page_id: string
  desired_page_id: string | null
  enabled_page_ids: string[] | null
  pages: ReleasePageMetadata[]
}

async function get_device_release(device_id: string): Promise<DeviceRelease | undefined> {
  if (!db) return undefined
  const [device] = await db.select({
    id: devices.id,
    release_id: devices.release_id,
    active_page_id: devices.active_page_id,
    desired_page_id: devices.desired_page_id,
    enabled_page_ids: devices.enabled_page_ids,
  }).from(devices).where(eq(devices.id, device_id)).limit(1)
  if (!device?.release_id) return undefined

  const [release] = await db.select({ id: display_releases.id, version: display_releases.version })
    .from(display_releases).where(eq(display_releases.id, device.release_id)).limit(1)
  if (!release) return undefined

  const pages = await db.select({
    page_id: display_release_pages.page_id,
    image_format: display_release_pages.image_format,
    image_width: display_release_pages.image_width,
    image_height: display_release_pages.image_height,
    image_sha256: display_release_pages.content_sha256,
    image_bytes: sql<number>`octet_length(${display_release_pages.device_image})`,
  }).from(display_release_pages).where(eq(display_release_pages.release_id, release.id)).orderBy(asc(display_release_pages.position))
  if (!pages.length) return undefined
  return {
    device_id: device.id,
    release_id: release.id,
    release_version: release.version,
    active_page_id: device.active_page_id,
    desired_page_id: device.desired_page_id,
    enabled_page_ids: device.enabled_page_ids,
    pages,
  }
}

export async function get_device_page_configuration(device_id: string): Promise<DevicePageConfiguration | undefined> {
  const device_release = await get_device_release(device_id)
  if (!device_release) return undefined
  const available_ids = new Set(device_release.pages.map((page) => page.page_id))
  const enabled_page_ids = (device_release.enabled_page_ids?.length ? device_release.enabled_page_ids : device_release.pages.map((page) => page.page_id))
    .filter((page_id) => available_ids.has(page_id))
  if (!enabled_page_ids.length) return undefined
  const desired_page_id = enabled_page_ids.includes(device_release.desired_page_id ?? '')
    ? device_release.desired_page_id!
    : enabled_page_ids[0]
  const page_by_id = new Map(device_release.pages.map((page) => [page.page_id, page]))
  return {
    device_id,
    release_id: device_release.release_id,
    release_version: device_release.release_version,
    active_page_id: device_release.active_page_id,
    desired_page_id,
    enabled_page_ids,
    pages: enabled_page_ids.flatMap((page_id) => page_by_id.get(page_id) ?? []),
    available_pages: device_release.pages,
  }
}

export async function set_device_page_configuration(device_id: string, enabled_page_ids: string[], desired_page_id: string) {
  if (!db) throw new Error('database_unavailable')
  if (enabled_page_ids.length === 0 || enabled_page_ids.length > 10 || new Set(enabled_page_ids).size !== enabled_page_ids.length || !enabled_page_ids.includes(desired_page_id)) {
    throw new Error('device_page_configuration_invalid')
  }
  const device_release = await get_device_release(device_id)
  if (!device_release) throw new Error('device_release_not_found')
  const page_by_id = new Map(device_release.pages.map((page) => [page.page_id, page]))
  if (enabled_page_ids.some((page_id) => !page_by_id.has(page_id))) throw new Error('device_page_not_in_release')

  const pages = enabled_page_ids.map((page_id) => page_by_id.get(page_id)!)
  await db.update(devices).set({ enabled_page_ids, desired_page_id }).where(eq(devices.id, device_id))
  await publish_device_release(device_id, {
    id: device_release.release_id,
    version: device_release.release_version,
    active_page_id: desired_page_id,
    pages,
  })
  return {
    device_id,
    release_id: device_release.release_id,
    release_version: device_release.release_version,
    active_page_id: device_release.active_page_id,
    desired_page_id,
    enabled_page_ids,
    pages,
    available_pages: device_release.pages,
  } satisfies DevicePageConfiguration
}

export async function validate_device_page_command(device_id: string, action: string, payload: Record<string, unknown>) {
  if (action !== 'show_page') return
  if (typeof payload.page_id !== 'string') throw new Error('page_id_required')
  const configuration = await get_device_page_configuration(device_id)
  if (!configuration) throw new Error('device_release_not_found')
  if (!configuration.enabled_page_ids.includes(payload.page_id)) throw new Error('page_not_enabled')
}
