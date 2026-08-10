import { connect, type MqttClient } from 'mqtt'
import { and, eq } from 'drizzle-orm'

import { db } from './db'
import { signed_release_image_url } from './assets'
import { device_commands, devices, ota_jobs } from './schema'

let mqtt_client: MqttClient | undefined
let state_consumer_started = false

export function validate_mqtt_url(url: string, allow_plaintext_internal = false) {
  const parsed = new URL(url)
  if (parsed.protocol === 'mqtts:' || parsed.protocol === 'wss:') return parsed
  if (allow_plaintext_internal && (parsed.protocol === 'mqtt:' || parsed.protocol === 'ws:')) return parsed
  throw new Error('mqtt_tls_required')
}

export function command_message(command: { id: string; action: string; payload: unknown }) {
  return JSON.stringify({ command_id: command.id, action: command.action, payload: command.payload })
}

export function ota_message(job: { id: string; nonce: string; version: string; manifest_url: string; image_sha256: string }) {
  return JSON.stringify({ job_id: job.id, nonce: job.nonce, version: job.version, manifest_url: job.manifest_url, image_sha256: job.image_sha256 })
}

export function release_message(release: { id: string; active_page_id: string; pages: ReleasePageMetadata[] }, base_url: string) {
  if (!base_url.startsWith('https://')) throw new Error('device_asset_url_https_required')
  if (release.pages.length === 0 || !release.pages.some((page) => page.page_id === release.active_page_id)) throw new Error('release_pages_invalid')
  return JSON.stringify({
    release_id: release.id,
    document_version: 1,
    active_page_id: release.active_page_id,
    pages: release.pages.map((page) => ({ ...page, image_url: signed_release_image_url(base_url, release.id) })),
  })
}

function get_client() {
  if (mqtt_client) return mqtt_client
  const url = process.env.MQTT_URL
  if (!url) throw new Error('mqtt_url_missing')
  const endpoint = validate_mqtt_url(url, process.env.MQTT_ALLOW_PLAINTEXT_INTERNAL === 'true')
  mqtt_client = connect(endpoint.toString(), {
    reconnectPeriod: 5_000,
    rejectUnauthorized: endpoint.protocol === 'mqtts:' || endpoint.protocol === 'wss:',
  })
  return mqtt_client
}

export async function publish_device_command(device_id: string, command: { id: string; action: string; payload: unknown }, client = get_client()) {
  const topic = `glance_deck/${device_id}/command`
  const message = command_message(command)

  await new Promise<void>((resolve, reject) => {
    client.publish(topic, message, { qos: 1 }, (error) => error ? reject(error) : resolve())
  })
}

export async function publish_device_ota(device_id: string, job: { id: string; nonce: string; version: string; manifest_url: string; image_sha256: string }, client = get_client()) {
  const topic = `${TOPIC_PREFIX}/${device_id}/ota`
  const message = ota_message(job)
  await new Promise<void>((resolve, reject) => {
    client.publish(topic, message, { qos: 1 }, (error) => error ? reject(error) : resolve())
  })
}

export type ReleasePageMetadata = {
  page_id: string
  image_format: string
  image_width: number
  image_height: number
  image_sha256: string
  image_bytes: number
}

export async function publish_device_release(device_id: string, release: { id: string; version: number; active_page_id: string; pages: ReleasePageMetadata[] }, client = get_client()) {
  const base_url = process.env.DEVICE_ASSET_URL ?? process.env.APP_URL
  if (!base_url) throw new Error('device_asset_url_https_required')
  const message = release_message(release, base_url)
  await new Promise<void>((resolve, reject) => client.publish(`${TOPIC_PREFIX}/${device_id}/release`, message, { qos: 1, retain: true }, (error) => error ? reject(error) : resolve()))
}

type DeviceStateMessage = {
  version: number
  page_id: string
  wifi_rssi: number
  display_release_id?: string
  command_id?: string
  command_status?: 'confirmed' | 'failed'
  error_message?: string
  firmware_version?: string
}

type OtaStateMessage = {
  job_id: string
  phase: 'downloading' | 'verifying' | 'rebooting' | 'healthy' | 'rolled_back' | 'failed'
  error_message?: string
}

export function is_device_state(value: unknown): value is DeviceStateMessage {
  if (!value || typeof value !== 'object') return false
  const state = value as Record<string, unknown>
  return typeof state.version === 'number'
    && typeof state.page_id === 'string'
    && typeof state.wifi_rssi === 'number'
    && (state.command_id === undefined || typeof state.command_id === 'string')
    && (state.command_status === undefined || state.command_status === 'confirmed' || state.command_status === 'failed')
    && (state.firmware_version === undefined || typeof state.firmware_version === 'string')
}

export function is_ota_state(value: unknown): value is OtaStateMessage {
  if (!value || typeof value !== 'object') return false
  const message = value as Record<string, unknown>
  const valid_phase = ['downloading', 'verifying', 'rebooting', 'healthy', 'rolled_back', 'failed'].includes(String(message.phase))
  return typeof message.job_id === 'string'
    && valid_phase
    && (message.error_message === undefined || typeof message.error_message === 'string')
}

export async function consume_device_state(topic: string, payload: Buffer) {
  const match = /^glance_deck\/([a-z0-9-]{1,64})\/state$/.exec(topic)
  if (!match || !db || payload.length > 4096) return
  let state: unknown
  try {
    state = JSON.parse(payload.toString('utf8'))
  } catch {
    return
  }
  if (!is_device_state(state)) return
  const device_id = match[1]
  await db.update(devices).set({
    status: 'online',
    active_page_id: state.page_id,
    wifi_rssi: Math.trunc(state.wifi_rssi),
    firmware_version: state.firmware_version,
    last_seen_at: new Date(),
  }).where(eq(devices.id, device_id))

  if (state.command_id && state.command_status) {
    await db.update(device_commands).set({
      status: state.command_status,
      error_message: state.command_status === 'failed' ? state.error_message ?? 'device_command_failed' : null,
      confirmed_at: state.command_status === 'confirmed' ? new Date() : null,
    }).where(and(eq(device_commands.id, state.command_id), eq(device_commands.device_id, device_id), eq(device_commands.status, 'sent')))
  }
}

export async function consume_ota_state(topic: string, payload: Buffer) {
  const match = /^glance_deck\/([a-z0-9-]{1,64})\/ota\/state$/.exec(topic)
  if (!match || !db || payload.length > 4096) return
  let state: unknown
  try { state = JSON.parse(payload.toString('utf8')) } catch { return }
  if (!is_ota_state(state)) return
  const phase = state.phase
  await db.update(ota_jobs).set({
    status: phase,
    error_message: phase === 'failed' ? state.error_message ?? 'device_ota_failed' : null,
    completed_at: phase === 'healthy' || phase === 'rolled_back' || phase === 'failed' ? new Date() : null,
  }).where(and(eq(ota_jobs.id, state.job_id), eq(ota_jobs.device_id, match[1])))
}

export function start_device_state_consumer() {
  if (state_consumer_started) return
  const client = get_client()
  state_consumer_started = true
  client.subscribe([`${TOPIC_PREFIX}/+/state`, `${TOPIC_PREFIX}/+/ota/state`], { qos: 1 })
  client.on('message', (topic, payload) => {
    const handler = topic.endsWith('/ota/state') ? consume_ota_state : consume_device_state
    void handler(topic, payload).catch((error) => console.error('device MQTT state consume failed', error))
  })
}

const TOPIC_PREFIX = 'glance_deck'
