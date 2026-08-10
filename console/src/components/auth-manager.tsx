'use client'

import { Alert, Block, Button, Flexbox, Input, Text } from '@lobehub/ui'
import { KeyRound, LogIn } from 'lucide-react'
import { useLocale, useTranslations } from 'next-intl'
import { useRouter } from '@/i18n/navigation'
import { useState } from 'react'

function decode_base64url(value: string) {
  const binary = atob(value.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - value.length % 4) % 4))
  return Uint8Array.from(binary, (character) => character.charCodeAt(0))
}

function encode_base64url(value: ArrayBuffer) {
  return btoa(String.fromCharCode(...new Uint8Array(value))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

export function LoginManager() {
  const translate = useTranslations('Auth')
  const locale = useLocale()
  const router = useRouter()
  const [email, set_email] = useState('')
  const [password, set_password] = useState('')
  const [busy, set_busy] = useState(false)
  const [error, set_error] = useState<string | null>(null)

  const finish = () => router.replace('/', { locale })
  const login = async (event: React.FormEvent) => {
    event.preventDefault()
    set_busy(true); set_error(null)
    try {
      const response = await fetch('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, password }) })
      if (!response.ok) throw new Error('invalidCredentials')
      finish()
    } catch (login_error) {
      set_error(login_error instanceof Error ? translate(login_error.message as 'invalidCredentials') : translate('loginFailed'))
    } finally { set_busy(false) }
  }

  const passkey_login = async () => {
    set_busy(true); set_error(null)
    try {
      if (!window.PublicKeyCredential) throw new Error('passkeyUnsupported')
      const options_response = await fetch('/api/auth/passkeys/login/options', { method: 'POST' })
      const options = await options_response.json() as { challenge: string; allowCredentials?: Array<{ id: string; type: 'public-key'; transports?: AuthenticatorTransport[] }> }
      if (!options_response.ok) throw new Error('loginFailed')
      const credential = await navigator.credentials.get({ publicKey: {
        ...options,
        challenge: decode_base64url(options.challenge),
        allowCredentials: options.allowCredentials?.map((item) => ({ ...item, id: decode_base64url(item.id) })),
      } }) as PublicKeyCredential | null
      if (!credential) throw new Error('loginFailed')
      const response = credential.response as AuthenticatorAssertionResponse
      const verify_response = await fetch('/api/auth/passkeys/login/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ id: credential.id, rawId: encode_base64url(credential.rawId), type: credential.type, response: { clientDataJSON: encode_base64url(response.clientDataJSON), authenticatorData: encode_base64url(response.authenticatorData), signature: encode_base64url(response.signature), userHandle: response.userHandle ? encode_base64url(response.userHandle) : undefined }, clientExtensionResults: credential.getClientExtensionResults() }) })
      if (!verify_response.ok) throw new Error('loginFailed')
      finish()
    } catch (login_error) {
      set_error(login_error instanceof Error ? translate(login_error.message as 'loginFailed' | 'passkeyUnsupported') : translate('loginFailed'))
    } finally { set_busy(false) }
  }

  return <main className="auth-shell"><Block className="auth-card" variant="outlined"><Flexbox gap={10}><Text className="eyebrow"><LogIn aria-hidden />Glance Deck</Text><h1>{translate('loginTitle')}</h1><Text type="secondary">{translate('loginDescription')}</Text></Flexbox>{error && <Alert showIcon type="error" message={error} /> }<form onSubmit={login}><Flexbox gap={12}><label htmlFor="login-email">{translate('email')}</label><Input id="login-email" required type="email" autoComplete="username" value={email} onChange={(event) => set_email(event.target.value)} /><label htmlFor="login-password">{translate('password')}</label><Input id="login-password" required type="password" autoComplete="current-password" value={password} onChange={(event) => set_password(event.target.value)} /><Button htmlType="submit" loading={busy} type="primary">{translate('login')}</Button></Flexbox></form><Button icon={KeyRound} disabled={busy} onClick={() => void passkey_login()}>{translate('passkeyLogin')}</Button><Button type="text" onClick={() => router.push('/setup')}>{translate('firstRunHint')}</Button></Block></main>
}

export function SetupManager() {
  const translate = useTranslations('Auth')
  const locale = useLocale()
  const router = useRouter()
  const [email, set_email] = useState('')
  const [password, set_password] = useState('')
  const [busy, set_busy] = useState(false)
  const [error, set_error] = useState<string | null>(null)
  const setup = async (event: React.FormEvent) => {
    event.preventDefault(); set_busy(true); set_error(null)
    try {
      const response = await fetch('/api/auth/setup', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, password }) })
      if (!response.ok) throw new Error('setupFailed')
      router.replace('/', { locale })
    } catch (setup_error) { set_error(setup_error instanceof Error ? translate(setup_error.message as 'setupFailed') : translate('setupFailed')) } finally { set_busy(false) }
  }
  return <main className="auth-shell"><Block className="auth-card" variant="outlined"><Text className="eyebrow">Glance Deck</Text><h1>{translate('setupTitle')}</h1><Text type="secondary">{translate('setupDescription')}</Text>{error && <Alert showIcon type="error" message={error} />}<form onSubmit={setup}><Flexbox gap={12}><label htmlFor="setup-email">{translate('email')}</label><Input id="setup-email" required type="email" autoComplete="username" value={email} onChange={(event) => set_email(event.target.value)} /><label htmlFor="setup-password">{translate('password')}</label><Input id="setup-password" required minLength={12} type="password" autoComplete="new-password" value={password} onChange={(event) => set_password(event.target.value)} /><Button htmlType="submit" loading={busy} type="primary">{translate('createAdministrator')}</Button></Flexbox></form></Block></main>
}
