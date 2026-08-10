import { afterEach, describe, expect, test } from 'bun:test'

import { signed_release_image_url, verify_release_image_signature } from './assets'

const previous_key = process.env.DEVICE_ASSET_SIGNING_KEY

afterEach(() => {
  if (previous_key) process.env.DEVICE_ASSET_SIGNING_KEY = previous_key
  else delete process.env.DEVICE_ASSET_SIGNING_KEY
})

describe('device image signatures', () => {
  test('creates a signed release URL accepted by verification', () => {
    process.env.DEVICE_ASSET_SIGNING_KEY = 'test-signing-key'
    const url = new URL(signed_release_image_url('https://console.example/base', 'release-1'))
    expect(url.pathname).toBe('/api/v1/releases/release-1/image')
    expect(verify_release_image_signature('release-1', url.searchParams.get('expires_at'), url.searchParams.get('signature'))).toBe(true)
  })

  test('rejects missing, malformed, expired, and altered signatures', () => {
    process.env.DEVICE_ASSET_SIGNING_KEY = 'test-signing-key'
    expect(verify_release_image_signature('release-1', null, null)).toBe(false)
    expect(verify_release_image_signature('release-1', 'bad', 'signature')).toBe(false)
    expect(verify_release_image_signature('release-1', '1', 'signature')).toBe(false)
    const url = new URL(signed_release_image_url('https://console.example', 'release-1'))
    expect(verify_release_image_signature('another-release', url.searchParams.get('expires_at'), url.searchParams.get('signature'))).toBe(false)
    expect(verify_release_image_signature('release-1', url.searchParams.get('expires_at'), 'invalid')).toBe(false)
  })

  test('requires a signing key', () => {
    delete process.env.DEVICE_ASSET_SIGNING_KEY
    expect(() => signed_release_image_url('https://console.example', 'release-1')).toThrow('device_asset_signing_key_missing')
  })
})
