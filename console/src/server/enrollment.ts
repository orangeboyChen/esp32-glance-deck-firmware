import { createHash, randomBytes, randomInt } from 'node:crypto'

import { and, eq, gt, isNull } from 'drizzle-orm'

import { db } from './db'
import { decrypt_secret, encrypt_secret } from './secrets'
import { device_enrollment_requests, devices } from './schema'

const code_hash = (value: string) => createHash('sha256').update(value).digest('hex')

export function create_pairing_code() {
  return String(randomInt(100000, 1_000_000))
}

export function valid_claim_secret(value: string) {
  return /^[a-f0-9]{64}$/.test(value)
}

export async function announce_enrollment(pairing_code: string, claim_secret: string, board_model: 'ESP32-S3-RLCD-4.2') {
  if (!db) throw new Error('database_unavailable')
  if (!/^\d{6}$/.test(pairing_code) || !valid_claim_secret(claim_secret)) throw new Error('invalid_enrollment_request')
  const pairing_code_hash = code_hash(pairing_code)
  const claim_secret_hash = code_hash(claim_secret)
  const [existing] = await db.select().from(device_enrollment_requests).where(eq(device_enrollment_requests.pairing_code_hash, pairing_code_hash)).limit(1)
  if (existing && existing.expires_at > new Date()) {
    if (existing.claim_secret_hash !== claim_secret_hash || existing.board_model !== board_model) throw new Error('pairing_code_in_use')
    return { expires_at: existing.expires_at, status: existing.claimed_device_id ? 'approved' : 'pending' }
  }
  if (existing) await db.delete(device_enrollment_requests).where(eq(device_enrollment_requests.id, existing.id))
  const [request] = await db.insert(device_enrollment_requests).values({ pairing_code_hash, claim_secret_hash, board_model, expires_at: new Date(Date.now() + 10 * 60 * 1000) }).returning({ expires_at: device_enrollment_requests.expires_at })
  return { expires_at: request.expires_at, status: 'pending' }
}

export async function approve_enrollment(name: string, pairing_code: string, board_model: 'ESP32-S3-RLCD-4.2') {
  if (!db) throw new Error('database_unavailable')
  const [request] = await db.select().from(device_enrollment_requests).where(and(eq(device_enrollment_requests.pairing_code_hash, code_hash(pairing_code)), gt(device_enrollment_requests.expires_at, new Date()), isNull(device_enrollment_requests.claimed_device_id))).limit(1)
  if (!request || request.board_model !== board_model) throw new Error('pairing_code_invalid_or_expired')
  const device_id = `deck-${randomBytes(6).toString('hex')}`
  const mqtt_password = randomBytes(32).toString('base64url')
  await db.transaction(async (transaction) => {
    await transaction.insert(devices).values({ id: device_id, name, board_model, status: 'enrolling', mqtt_username: device_id, mqtt_password_ciphertext: encrypt_secret({ mqtt_password }) })
    await transaction.update(device_enrollment_requests).set({ claimed_device_id: device_id }).where(and(eq(device_enrollment_requests.id, request.id), isNull(device_enrollment_requests.claimed_device_id)))
  })
  return { device_id, name, status: 'approved' }
}

export async function claim_enrollment(pairing_code: string, claim_secret: string) {
  if (!db) throw new Error('database_unavailable')
  const [request] = await db.select().from(device_enrollment_requests).where(and(eq(device_enrollment_requests.pairing_code_hash, code_hash(pairing_code)), eq(device_enrollment_requests.claim_secret_hash, code_hash(claim_secret)), gt(device_enrollment_requests.expires_at, new Date()))).limit(1)
  if (!request) throw new Error('pairing_code_invalid_or_expired')
  if (!request.claimed_device_id) return { status: 'pending' as const }
  const [device] = await db.select().from(devices).where(eq(devices.id, request.claimed_device_id)).limit(1)
  if (!device?.mqtt_password_ciphertext || !device.mqtt_username) throw new Error('enrollment_credentials_unavailable')
  const { mqtt_password } = decrypt_secret(device.mqtt_password_ciphertext)
  await db.update(devices).set({ status: 'offline' }).where(eq(devices.id, device.id))
  return { status: 'claimed' as const, device_id: device.id, mqtt: { broker_url: process.env.DEVICE_MQTT_URL ?? 'mqtts://mqtt.example.invalid', username: device.mqtt_username, password: mqtt_password } }
}
