import { describe, expect, mock, test } from 'bun:test'

const updates: Array<Record<string, unknown>> = []
const where = mock(async () => undefined)
const set = mock((value: Record<string, unknown>) => {
  updates.push(value)
  return { where }
})
const update = mock(() => ({ set }))

mock.module('./db', () => ({ db: { update } }))

const { consume_device_state, consume_ota_state } = await import('./mqtt')
const { consume_ota_check } = await import('./mqtt')

describe('MQTT state consumers', () => {
  test('persists valid device state and command confirmation', async () => {
    updates.length = 0
    await consume_device_state('glance_deck/desk-1/state', Buffer.from(JSON.stringify({ version: 1, page_id: 'usage', wifi_rssi: -55, firmware_version: '1.0.0', power: { source: 'usb_and_battery', charging: true, battery_percent: 82, battery_mv: 3975 }, command_id: 'command-1', command_status: 'confirmed' })))
    expect(updates).toHaveLength(2)
    expect(updates[0]).toMatchObject({ status: 'online', active_page_id: 'usage', wifi_rssi: -55, firmware_version: '1.0.0', power_source: 'usb_and_battery', charging: true, battery_percent: 82, battery_mv: 3975, power_updated_at: expect.any(Date) })
    expect(updates[1]).toMatchObject({ status: 'confirmed', confirmed_at: expect.any(Date) })
  })

  test('ignores malformed, oversized, and wrong-topic device state', async () => {
    updates.length = 0
    await consume_device_state('glance_deck/invalid!/state', Buffer.from('{}'))
    await consume_device_state('glance_deck/desk-1/state', Buffer.from('invalid JSON'))
    await consume_device_state('glance_deck/desk-1/state', Buffer.alloc(4097))
    await consume_device_state('glance_deck/desk-1/state', Buffer.from(JSON.stringify({ version: 1, page_id: 'usage', wifi_rssi: 'bad' })))
    await consume_device_state('glance_deck/desk-1/state', Buffer.from(JSON.stringify({ version: 1, page_id: 'usage', wifi_rssi: -55, power: { source: 'battery', battery_percent: 101 } })))
    expect(updates).toHaveLength(0)
  })

  test('persists terminal OTA states and applies an error fallback', async () => {
    updates.length = 0
    await consume_ota_state('glance_deck/desk-1/ota/state', Buffer.from(JSON.stringify({ job_id: 'job-1', phase: 'failed' })))
    await consume_ota_state('glance_deck/desk-1/ota/state', Buffer.from(JSON.stringify({ job_id: 'job-2', phase: 'healthy' })))
    expect(updates).toHaveLength(2)
    expect(updates[0]).toMatchObject({ status: 'failed', error_message: 'device_ota_failed', completed_at: expect.any(Date) })
    expect(updates[1]).toMatchObject({ status: 'healthy', error_message: null, completed_at: expect.any(Date) })
  })

  test('ignores invalid OTA payloads', async () => {
    updates.length = 0
    await consume_ota_state('glance_deck/desk-1/state', Buffer.from('{}'))
    await consume_ota_state('glance_deck/desk-1/ota/state', Buffer.from('bad JSON'))
    await consume_ota_state('glance_deck/desk-1/ota/state', Buffer.from(JSON.stringify({ job_id: 'job-1', phase: 'missing' })))
    expect(updates).toHaveLength(0)
  })

  test('ignores invalid local OTA check requests', async () => {
    const database = { select: () => ({ from: () => ({ where: () => ({ limit: async () => [] }) }) }) }
    const client = { publish: mock((_topic: string, _payload: string, _options: unknown, callback: (error?: Error) => void) => callback()) }
    await consume_ota_check('glance_deck/invalid!/ota/check', Buffer.from('{}'), database as never, client as never)
    await consume_ota_check('glance_deck/desk-1/ota/check', Buffer.from('bad'), database as never, client as never)
    expect(client.publish).not.toHaveBeenCalled()
  })
})
