import { connect, type MqttClient } from 'mqtt'
import { and, eq } from 'drizzle-orm'

import { db } from './db'
import { signed_release_image_url } from './assets'
import { device_commands, devices, ota_jobs } from './schema'

let mqtt_client: MqttClient | undefined
let state_consumer_started = false

function get_client() {
  if (mqtt_client) return mqtt_client
  const url = process.env.MQTT_URL
  if (!url) throw new Error('mqtt_url_missing')
  mqtt_client = connect(url, { reconnectPeriod: 5_000 })
  return mqtt_client
}

export async function publish_device_command(device_id: string, command: { id: string; action: string; payload: unknown }) {
  const client = get_client()
  const topic = `glance_deck/${device_id}/command`
  const message = JSON.stringify({ command_id: command.id, action: command.action, payload: command.payload })

  await new Promise<void>((resolve, reject) => {
    client.publish(topic, message, { qos: 1 }, (error) => error ? reject(error) : resolve())
  })
}

export async function publish_device_ota(device_id: string, job: { id: string; nonce: string; version: string; manifest_url: string; image_sha256: string }) {
  const client = get_client()
  const topic = `${TOPIC_PREFIX}/${device_id}/ota`
  const message = JSON.stringify({ job_id: job.id, nonce: job.nonce, version: job.version, manifest_url: job.manifest_url, image_sha256: job.image_sha256 })
  await new Promise<void>((resolve, reject) => {
    client.publish(topic, message, { qos: 1 }, (error) => error ? reject(error) : resolve())
  })
}

export async function publish_device_release(device_id: string, release: { id: string; version: number; page_id: string; image_sha256: string; image_bytes: number }) {
  const client = get_client()
  const base_url = process.env.DEVICE_ASSET_URL ?? process.env.APP_URL
  if (!base_url?.startsWith('https://')) throw new Error('device_asset_url_https_required')
  const message = JSON.stringify({ release_id: release.id, document_version: 1, image_url: signed_release_image_url(base_url, release.id), image_sha256: release.image_sha256, image_bytes: release.image_bytes, active_page_id: release.page_id })
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

function is_device_state(value: unknown): value is DeviceStateMessage {
  if (!value || typeof value !== 'object') return false
  const state = value as Record<string, unknown>
  return typeof state.version === 'number'
    && typeof state.page_id === 'string'
    && typeof state.wifi_rssi === 'number'
    && (state.command_id === undefined || typeof state.command_id === 'string')
    && (state.command_status === undefined || state.command_status === 'confirmed' || state.command_status === 'failed')
    && (state.firmware_version === undefined || typeof state.firmware_version === 'string')
}

async function consume_device_state(topic: string, payload: Buffer) {
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

async function consume_ota_state(topic: string, payload: Buffer) {
  const match = /^glance_deck\/([a-z0-9-]{1,64})\/ota\/state$/.exec(topic)
  if (!match || !db || payload.length > 4096) return
  let state: unknown
  try { state = JSON.parse(payload.toString('utf8')) } catch { return }
  if (!state || typeof state !== 'object') return
  const message = state as Record<string, unknown>
  const valid_phase = ['downloading', 'verifying', 'rebooting', 'healthy', 'rolled_back', 'failed'].includes(String(message.phase))
  if (typeof message.job_id !== 'string' || !valid_phase || (message.error_message !== undefined && typeof message.error_message !== 'string')) return
  const phase = message.phase as OtaStateMessage['phase']
  await db.update(ota_jobs).set({
    status: phase,
    error_message: phase === 'failed' ? (message.error_message as string | undefined) ?? 'device_ota_failed' : null,
    completed_at: phase === 'healthy' || phase === 'rolled_back' || phase === 'failed' ? new Date() : null,
  }).where(and(eq(ota_jobs.id, message.job_id), eq(ota_jobs.device_id, match[1])))
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
