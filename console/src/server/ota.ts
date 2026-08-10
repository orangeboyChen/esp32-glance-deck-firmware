import { randomBytes } from 'node:crypto'

import { and, asc, eq } from 'drizzle-orm'

import { db } from './db'
import { publish_device_ota } from './mqtt'
import { firmware_releases, ota_jobs } from './schema'

export async function dispatch_queued_ota_jobs() {
  if (!db) return 0
  const jobs = await db.select({ id: ota_jobs.id, device_id: ota_jobs.device_id, nonce: ota_jobs.nonce, version: firmware_releases.version, manifest_url: firmware_releases.manifest_url, image_sha256: firmware_releases.image_sha256 })
    .from(ota_jobs).innerJoin(firmware_releases, eq(ota_jobs.firmware_release_id, firmware_releases.id))
    .where(eq(ota_jobs.status, 'queued')).orderBy(asc(ota_jobs.created_at)).limit(10)
  for (const job of jobs) {
    try {
      await publish_device_ota(job.device_id, job)
      await db.update(ota_jobs).set({ status: 'sent' }).where(and(eq(ota_jobs.id, job.id), eq(ota_jobs.status, 'queued')))
    } catch (error) {
      await db.update(ota_jobs).set({ status: 'failed', error_message: error instanceof Error ? error.message : 'mqtt_publish_failed', completed_at: new Date() }).where(eq(ota_jobs.id, job.id))
    }
  }
  return jobs.length
}

export function create_ota_nonce() {
  return randomBytes(24).toString('base64url')
}
