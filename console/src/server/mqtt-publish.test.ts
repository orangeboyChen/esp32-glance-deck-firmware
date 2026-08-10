import { afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import type { MqttClient } from 'mqtt'

const publish = mock((_: string, __: string, ___: unknown, callback: (error?: Error) => void) => callback())

describe('MQTT device publishing', () => {
  beforeEach(() => {
    process.env.MQTT_URL = 'mqtts://broker.example'
    process.env.DEVICE_ASSET_URL = 'https://console.example'
    process.env.DEVICE_ASSET_SIGNING_KEY = 'mqtt-publish-test-key'
    publish.mockClear()
  })

  afterEach(() => {
    delete process.env.MQTT_URL
    delete process.env.DEVICE_ASSET_URL
    delete process.env.DEVICE_ASSET_SIGNING_KEY
  })

  test('publishes commands, OTA jobs, and retained bitmap releases', async () => {
    const mqtt = await import('./mqtt')
    const client = { publish } as unknown as MqttClient
    await mqtt.publish_device_command('desk-1', { id: 'command-1', action: 'next_page', payload: {} }, client)
    await mqtt.publish_device_ota('desk-1', { id: 'job-1', nonce: 'nonce', version: '1.0.0', manifest_url: 'https://releases.example/manifest.json', image_sha256: 'a'.repeat(64) }, client)
    await mqtt.publish_device_ota_check_state('desk-1', { status: 'available', job_id: 'job-2', nonce: 'nonce-2', version: '1.1.0', manifest_url: 'https://releases.example/manifest.json', image_sha256: 'b'.repeat(64) }, client)
    await mqtt.publish_device_release('desk-1', {
      id: 'release-1', version: 1, active_page_id: 'usage',
      pages: [{ page_id: 'usage', image_format: 'mono1-msb', image_width: 400, image_height: 300, image_sha256: 'b'.repeat(64), image_bytes: 15000 }],
    }, client)

    expect(publish).toHaveBeenCalledTimes(4)
    expect(publish.mock.calls[0]?.[0]).toBe('glance_deck/desk-1/command')
    expect(JSON.parse(publish.mock.calls[1]?.[1] as string)).toMatchObject({ job_id: 'job-1', version: '1.0.0' })
    expect(publish.mock.calls[2]?.[0]).toBe('glance_deck/desk-1/ota/check/state')
    expect(publish.mock.calls[3]?.[0]).toBe('glance_deck/desk-1/release')
    expect(publish.mock.calls[3]?.[2]).toEqual({ qos: 1, retain: true })
  })
})
