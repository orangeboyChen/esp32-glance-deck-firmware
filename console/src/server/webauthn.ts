import { generateRegistrationOptions, verifyRegistrationResponse } from '@simplewebauthn/server'
import type { AuthenticatorTransportFuture, RegistrationResponseJSON } from '@simplewebauthn/types'
import { and, eq, gt } from 'drizzle-orm'

import { db } from './db'
import { passkeys, webauthn_challenges } from './schema'

const rp_id = process.env.WEBAUTHN_RP_ID ?? 'localhost'
const rp_name = 'ESP32 Glance Deck'
const origin = process.env.APP_URL ?? 'http://localhost:3000'

export async function begin_passkey_registration(administrator: { id: string; email: string }) {
  if (!db) throw new Error('database_unavailable')
  const existing = await db.select().from(passkeys).where(eq(passkeys.administrator_id, administrator.id))
  const options = await generateRegistrationOptions({
    rpName: rp_name,
    rpID: rp_id,
    userName: administrator.email,
    userID: new TextEncoder().encode(administrator.id),
    attestationType: 'none',
    excludeCredentials: existing.map((key) => ({
      id: key.credential_id,
      transports: (key.transports as AuthenticatorTransportFuture[] | null) ?? undefined,
    })),
  })

  await db.insert(webauthn_challenges).values({
    administrator_id: administrator.id,
    challenge: options.challenge,
    purpose: 'registration',
    expires_at: new Date(Date.now() + 5 * 60 * 1000),
  })
  return options
}

export async function finish_passkey_registration(administrator_id: string, response: RegistrationResponseJSON) {
  if (!db) throw new Error('database_unavailable')
  const [challenge] = await db.select().from(webauthn_challenges)
    .where(and(
      eq(webauthn_challenges.administrator_id, administrator_id),
      eq(webauthn_challenges.purpose, 'registration'),
      gt(webauthn_challenges.expires_at, new Date()),
    ))
    .orderBy(webauthn_challenges.created_at)
    .limit(1)
  if (!challenge) throw new Error('challenge_expired')

  const verification = await verifyRegistrationResponse({
    response,
    expectedChallenge: challenge.challenge,
    expectedOrigin: origin,
    expectedRPID: rp_id,
  })
  if (!verification.verified || !verification.registrationInfo) throw new Error('registration_not_verified')

  const { credential } = verification.registrationInfo
  await db.insert(passkeys).values({
    administrator_id,
    credential_id: credential.id,
    public_key: Buffer.from(credential.publicKey).toString('base64url'),
    counter: credential.counter,
    transports: response.response.transports,
  })
  return verification.verified
}
