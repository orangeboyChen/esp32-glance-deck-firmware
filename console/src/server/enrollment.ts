import { createHash, randomBytes, randomInt } from 'node:crypto'

import { and, eq, gt } from 'drizzle-orm'

import { db } from './db'
import { encrypt_secret } from './secrets'
import { devices } from './schema'

const code_hash = (value: string) => createHash('sha256').update(value).digest('hex')

export function create_pairing_code() {
  return String(randomInt(100000, 1_000_000))
}

export async function create_enrollment(name: string, board_model: 'ESP32-S3-RLCD-4.2') {
  if (!db) throw new Error('database_unavailable')
  const device_id = `deck-${randomBytes(6).toString('hex')}`
  const pairing_code = create_pairing_code()
  const [device] = await db.insert(devices).values({
    id: device_id, name, board_model, status: 'enrolling',
    enrollment_code_hash: code_hash(pairing_code), enrollment_expires_at: new Date(Date.now() + 10 * 60 * 1000),
  }).returning({ id: devices.id, name: devices.name, enrollment_expires_at: devices.enrollment_expires_at })
  return { device, pairing_code }
}

export async function claim_enrollment(pairing_code: string) {
  if (!db) throw new Error('database_unavailable')
  const [device] = await db.select().from(devices).where(and(eq(devices.enrollment_code_hash, code_hash(pairing_code)), gt(devices.enrollment_expires_at, new Date()))).limit(1)
  if (!device) throw new Error('pairing_code_invalid_or_expired')
  const mqtt_password = randomBytes(32).toString('base64url')
  const mqtt_username = device.id
  await db.update(devices).set({ status: 'offline', enrollment_code_hash: null, enrollment_expires_at: null, mqtt_username, mqtt_password_ciphertext: encrypt_secret({ mqtt_password }) }).where(eq(devices.id, device.id))
  return { device_id: device.id, mqtt: { broker_url: process.env.DEVICE_MQTT_URL ?? 'mqtts://mqtt.example.invalid', username: mqtt_username, password: mqtt_password } }
}
