import { createPublicKey, verify } from 'node:crypto'

export function firmware_manifest_payload(input: { version: string; board_model: string; image_url: string; image_sha256: string }) {
  return JSON.stringify({ board_model: input.board_model, image_sha256: input.image_sha256.toLowerCase(), image_url: input.image_url, version: input.version })
}

export function verify_firmware_manifest(input: { version: string; board_model: string; image_url: string; image_sha256: string; manifest_signature: string }) {
  const public_key = process.env.FIRMWARE_MANIFEST_PUBLIC_KEY
  if (!public_key) throw new Error('firmware_signing_key_missing')
  try {
    return verify(null, Buffer.from(firmware_manifest_payload(input)), createPublicKey(public_key), Buffer.from(input.manifest_signature, 'hex'))
  } catch {
    return false
  }
}
