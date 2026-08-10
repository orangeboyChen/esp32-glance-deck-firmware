import { createHmac, timingSafeEqual } from 'node:crypto'

const asset_ttl_seconds = 60 * 60

function asset_key() {
  const key = process.env.DEVICE_ASSET_SIGNING_KEY
  if (!key) throw new Error('device_asset_signing_key_missing')
  return key
}

function signature(release_id: string, expires_at: number) {
  return createHmac('sha256', asset_key()).update(`${release_id}.${expires_at}`).digest('base64url')
}

export function signed_release_image_url(base_url: string, release_id: string) {
  const expires_at = Math.floor(Date.now() / 1000) + asset_ttl_seconds
  const url = new URL(`/api/v1/releases/${release_id}/image`, base_url)
  url.searchParams.set('expires_at', String(expires_at))
  url.searchParams.set('signature', signature(release_id, expires_at))
  return url.toString()
}

export function verify_release_image_signature(release_id: string, expires_at: string | null, provided_signature: string | null) {
  if (!expires_at || !provided_signature || !/^\d+$/.test(expires_at)) return false
  const expires = Number(expires_at)
  if (!Number.isSafeInteger(expires) || expires < Math.floor(Date.now() / 1000)) return false
  const expected = Buffer.from(signature(release_id, expires))
  const actual = Buffer.from(provided_signature)
  return expected.length === actual.length && timingSafeEqual(expected, actual)
}
