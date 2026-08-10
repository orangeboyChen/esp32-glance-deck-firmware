import { describe, expect, test } from 'bun:test'

import { command_message, is_device_state, is_ota_state, MAX_DEVICE_MQTT_PAYLOAD_BYTES, ota_message, release_message, validate_mqtt_url } from './mqtt'

describe('MQTT transport validation', () => {
  test('requires TLS outside an explicit trusted-internal exception', () => {
    expect(validate_mqtt_url('mqtts://broker.example').protocol).toBe('mqtts:')
    expect(() => validate_mqtt_url('mqtt://broker.example')).toThrow('mqtt_tls_required')
    expect(validate_mqtt_url('mqtt://broker.example', true).protocol).toBe('mqtt:')
  })

  test('serializes command and OTA messages without changing payloads', () => {
    expect(JSON.parse(command_message({ id: 'command-1', action: 'show_page', payload: { page_id: 'usage' } }))).toEqual({ command_id: 'command-1', action: 'show_page', payload: { page_id: 'usage' } })
    expect(JSON.parse(ota_message({ id: 'job-1', nonce: 'nonce', version: '1.2.3', manifest_url: 'https://releases.example/manifest.json', image_sha256: 'abc' }))).toEqual({ job_id: 'job-1', nonce: 'nonce', version: '1.2.3', manifest_url: 'https://releases.example/manifest.json', image_sha256: 'abc' })
  })

  test('builds only complete HTTPS release documents', () => {
    process.env.DEVICE_ASSET_SIGNING_KEY = 'unit-test-key'
    const release = { id: 'release-1', active_page_id: 'usage', pages: [{ page_id: 'usage', image_format: 'mono1-msb', image_width: 400, image_height: 300, image_sha256: 'a'.repeat(64), image_bytes: 15000 }] }
    const message = JSON.parse(release_message(release, 'https://console.example'))
    expect(message.pages[0].image_url).toContain('/api/v1/releases/release-1/pages/usage/image')
    expect(() => release_message({ ...release, pages: [] }, 'https://console.example')).toThrow('release_pages_invalid')
    expect(() => release_message(release, 'http://console.example')).toThrow('device_asset_url_https_required')
  })

  test('rejects a release document beyond the device memory limit', () => {
    process.env.DEVICE_ASSET_SIGNING_KEY = 'unit-test-key'
    const oversized_base_url = `https://${'a'.repeat(MAX_DEVICE_MQTT_PAYLOAD_BYTES)}.example`
    const release = { id: 'release-1', active_page_id: 'usage', pages: [{ page_id: 'usage', image_format: 'mono1-msb', image_width: 400, image_height: 300, image_sha256: 'a'.repeat(64), image_bytes: 15000 }] }
    expect(() => release_message(release, oversized_base_url)).toThrow('release_message_too_large')
  })

  test('accepts only complete device and OTA state messages', () => {
    expect(is_device_state({ version: 1, page_id: 'usage', wifi_rssi: -60 })).toBe(true)
    expect(is_device_state({ version: '1', page_id: 'usage', wifi_rssi: -60 })).toBe(false)
    expect(is_device_state({ version: 1, page_id: 'usage', wifi_rssi: -60, command_status: 'queued' })).toBe(false)
    expect(is_ota_state({ job_id: 'job-1', phase: 'healthy' })).toBe(true)
    expect(is_ota_state({ job_id: 'job-1', phase: 'unknown' })).toBe(false)
    expect(is_ota_state({ job_id: 3, phase: 'failed', error_message: 'bad' })).toBe(false)
    expect(is_ota_state({ job_id: 'job-1', phase: 'failed', error_message: 3 })).toBe(false)
  })
})
