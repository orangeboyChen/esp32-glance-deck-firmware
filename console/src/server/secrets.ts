import { createCipheriv, createDecipheriv, randomBytes } from 'node:crypto'

const algorithm = 'aes-256-gcm'

function encryption_key() {
  const encoded = process.env.APP_MASTER_KEY
  if (!encoded) throw new Error('app_master_key_missing')
  const key = Buffer.from(encoded, 'base64url')
  if (key.length !== 32) throw new Error('app_master_key_invalid')
  return key
}

export function encrypt_secret(value: Record<string, string>) {
  const iv = randomBytes(12)
  const cipher = createCipheriv(algorithm, encryption_key(), iv)
  const encrypted = Buffer.concat([cipher.update(JSON.stringify(value), 'utf8'), cipher.final()])
  return `${iv.toString('base64url')}.${cipher.getAuthTag().toString('base64url')}.${encrypted.toString('base64url')}`
}

export function decrypt_secret(value: string): Record<string, string> {
  const [iv_encoded, tag_encoded, content_encoded] = value.split('.')
  if (!iv_encoded || !tag_encoded || !content_encoded) throw new Error('secret_ciphertext_invalid')
  const decipher = createDecipheriv(algorithm, encryption_key(), Buffer.from(iv_encoded, 'base64url'))
  decipher.setAuthTag(Buffer.from(tag_encoded, 'base64url'))
  const plain = Buffer.concat([decipher.update(Buffer.from(content_encoded, 'base64url')), decipher.final()]).toString('utf8')
  const decoded: unknown = JSON.parse(plain)
  if (!decoded || typeof decoded !== 'object' || Array.isArray(decoded) || Object.values(decoded).some((item) => typeof item !== 'string')) {
    throw new Error('secret_value_invalid')
  }
  return decoded as Record<string, string>
}
