'use client'

import { Alert, Block, Button, Flexbox, Input, Modal, Segmented, Tag, Text, toast } from '@lobehub/ui'
import { ArrowLeft, KeyRound, Plus, ShieldCheck, Trash2 } from 'lucide-react'
import { atom, useAtom } from 'jotai'
import { useLocale, useTranslations } from 'next-intl'
import { useEffect, useState } from 'react'

import { usePathname, useRouter } from '@/i18n/navigation'

type ApiToken = { id: string; label: string; scopes: string[]; created_at: string }
type Passkey = { id: string; created_at: string; transports: string[] | null }
type NewToken = { token: string; record: ApiToken }

const token_label_atom = atom('Home Assistant')
const token_scopes_atom = atom<string[]>(['devices:read', 'devices:command', 'alerts:read'])

function to_base64url(value: ArrayBuffer) {
  const bytes = new Uint8Array(value)
  let binary = ''
  bytes.forEach((byte) => { binary += String.fromCharCode(byte) })
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function from_base64url(value: string) {
  const binary = atob(value.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - value.length % 4) % 4))
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0))
  return bytes.buffer
}

function serialise_credential(credential: Credential) {
  const public_key = credential as PublicKeyCredential
  const response = public_key.response as AuthenticatorAttestationResponse
  return {
    id: public_key.id,
    rawId: to_base64url(public_key.rawId),
    response: {
      clientDataJSON: to_base64url(response.clientDataJSON),
      attestationObject: to_base64url(response.attestationObject),
      transports: response.getTransports?.(),
    },
    type: public_key.type,
    clientExtensionResults: public_key.getClientExtensionResults(),
  }
}

export function SettingsManager() {
  const translate = useTranslations('Settings')
  const locale = useLocale()
  const pathname = usePathname()
  const router = useRouter()
  const [label, set_label] = useAtom(token_label_atom)
  const [scopes, set_scopes] = useAtom(token_scopes_atom)
  const [tokens, set_tokens] = useState<ApiToken[]>([])
  const [passkeys, set_passkeys] = useState<Passkey[]>([])
  const [loading, set_loading] = useState(true)
  const [saving, set_saving] = useState(false)
  const [new_token, set_new_token] = useState<NewToken | null>(null)
  const [remove_passkey, set_remove_passkey] = useState<Passkey | null>(null)
  const [passkey_busy, set_passkey_busy] = useState(false)
  const [error, set_error] = useState<string | null>(null)
  const change_locale = (next_locale: 'en' | 'zh-CN' | 'ja') => router.replace(pathname, { locale: next_locale })
  const load = async () => {
    set_loading(true); set_error(null)
    try {
      const [token_response, passkey_response] = await Promise.all([fetch('/api/v1/tokens', { cache: 'no-store' }), fetch('/api/auth/passkeys', { cache: 'no-store' })])
      if (!token_response.ok || !passkey_response.ok) throw new Error('loadFailed')
      set_tokens((await token_response.json() as { tokens: ApiToken[] }).tokens)
      set_passkeys((await passkey_response.json() as { passkeys: Passkey[] }).passkeys)
    } catch { set_error(translate('loadFailed')) } finally { set_loading(false) }
  }
  useEffect(() => { void load() }, [])

  const create_token = async () => {
    if (!label.trim() || scopes.length === 0) return
    set_saving(true)
    try {
      const response = await fetch('/api/v1/tokens', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ label: label.trim(), scopes }) })
      const payload = await response.json() as NewToken | { error?: string }
      if (!response.ok || !('token' in payload)) throw new Error('tokenCreateFailed')
      set_new_token(payload); set_label('Home Assistant'); await load(); toast.success(translate('tokenCreated'))
    } catch { toast.error(translate('tokenCreateFailed')) } finally { set_saving(false) }
  }
  const revoke_token = async (token: ApiToken) => {
    try {
      const response = await fetch(`/api/v1/tokens/${token.id}`, { method: 'DELETE' })
      if (!response.ok) throw new Error()
      set_tokens((current) => current.filter((item) => item.id !== token.id)); toast.success(translate('tokenRevoked'))
    } catch { toast.error(translate('tokenRevokeFailed')) }
  }
  const register_passkey = async () => {
    set_passkey_busy(true)
    try {
      if (!window.PublicKeyCredential) throw new Error('passkeyUnsupported')
      const options_response = await fetch('/api/auth/passkeys/register/options', { method: 'POST' })
      if (!options_response.ok) throw new Error('passkeyRegisterFailed')
      const options = await options_response.json() as PublicKeyCredentialCreationOptions & { challenge: string; user: PublicKeyCredentialUserEntity; excludeCredentials?: PublicKeyCredentialDescriptor[] }
      const credential = await navigator.credentials.create({ publicKey: { ...options, challenge: from_base64url(options.challenge), user: { ...options.user, id: from_base64url(options.user.id as unknown as string) }, excludeCredentials: options.excludeCredentials?.map((item) => ({ ...item, id: from_base64url(item.id as unknown as string) })) } })
      if (!credential) throw new Error('passkeyRegisterFailed')
      const verify_response = await fetch('/api/auth/passkeys/register/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(serialise_credential(credential)) })
      if (!verify_response.ok) throw new Error('passkeyRegisterFailed')
      toast.success(translate('passkeyAdded')); await load()
    } catch (registration_error) {
      const code = registration_error instanceof Error ? registration_error.message : 'passkeyRegisterFailed'
      toast.error(translate.has(code) ? translate(code) : translate('passkeyRegisterFailed'))
    } finally { set_passkey_busy(false) }
  }
  const delete_passkey = async () => {
    if (!remove_passkey) return
    set_passkey_busy(true)
    try {
      const response = await fetch(`/api/auth/passkeys/${remove_passkey.id}`, { method: 'DELETE' })
      if (!response.ok) throw new Error()
      set_passkeys((current) => current.filter((item) => item.id !== remove_passkey.id)); set_remove_passkey(null); toast.success(translate('passkeyRemoved'))
    } catch { toast.error(translate('passkeyRemoveFailed')) } finally { set_passkey_busy(false) }
  }
  const available_scopes = ['devices:read', 'devices:command', 'alerts:read', 'ota:install']
  return <main className="sources-shell settings-shell">
    <header className="dashboard-header"><Flexbox className="dashboard-introduction" gap={10}><Button icon={ArrowLeft} onClick={() => router.push('/')} size="large">{translate('back')}</Button><Text className="eyebrow"><ShieldCheck aria-hidden />{translate('eyebrow')}</Text><h1>{translate('title')}</h1><Text className="header-subtitle">{translate('subtitle')}</Text></Flexbox><Segmented aria-label={translate('language')} options={[{ label: 'EN', value: 'en' }, { label: '中文', value: 'zh-CN' }, { label: '日本語', value: 'ja' }]} value={locale} onChange={(value) => change_locale(value as 'en' | 'zh-CN' | 'ja')} /></header>
    {error && <Alert className="settings-alert" message={error} showIcon type="error" />}
    {loading ? <Text>{translate('loading')}</Text> : (
      <Flexbox className="settings-sections" gap={28}>
        <section aria-labelledby="ha-token-heading">
          <Flexbox gap={4}><h2 id="ha-token-heading">{translate('tokensTitle')}</h2><Text type="secondary">{translate('tokensDescription')}</Text></Flexbox>
          <Block className="settings-card" variant="outlined">
            <Flexbox className="settings-token-form" gap={10}>
              <label htmlFor="token-label">{translate('tokenLabel')}</label>
              <Input id="token-label" maxLength={128} value={label} onChange={(event) => set_label(event.target.value)} />
              <Text type="secondary">{translate('scopeHint')}</Text>
              <Flexbox className="scope-list" gap={8}>{available_scopes.map((scope) => <label className="scope-option" key={scope}><input checked={scopes.includes(scope)} type="checkbox" onChange={(event) => set_scopes((current) => event.target.checked ? [...current, scope] : current.filter((item) => item !== scope))} />{translate(`scope_${scope.replace(':', '_')}`)}</label>)}</Flexbox>
              <Button disabled={!label.trim() || scopes.length === 0} loading={saving} icon={Plus} onClick={() => void create_token()} size="large" type="primary">{translate('createToken')}</Button>
            </Flexbox>
          </Block>
          <Flexbox className="token-list" gap={10}>{tokens.length === 0 ? <Text type="secondary">{translate('noTokens')}</Text> : tokens.map((token) => <Block className="token-row" key={token.id} variant="outlined"><Flexbox gap={5}><Flexbox horizontal align="center" justify="space-between" gap={12}><Text strong>{token.label}</Text><Button aria-label={translate('revokeToken')} icon={Trash2} onClick={() => void revoke_token(token)} size="large" type="text" /></Flexbox><Text type="secondary">{token.scopes.map((scope) => translate(`scope_${scope.replace(':', '_')}`)).join(' · ')}</Text><Text type="secondary">{translate('created', { date: new Date(token.created_at).toLocaleString(locale) })}</Text></Flexbox></Block>)}</Flexbox>
        </section>
        <section aria-labelledby="passkey-heading">
          <Flexbox gap={4}><h2 id="passkey-heading">{translate('passkeysTitle')}</h2><Text type="secondary">{translate('passkeysDescription')}</Text></Flexbox>
          <Block className="settings-card" variant="outlined">
            <Flexbox horizontal align="center" justify="space-between" gap={16} wrap="wrap"><Flexbox horizontal align="center" gap={10}><KeyRound aria-hidden /><Text>{translate('passkeyCount', { count: passkeys.length })}</Text></Flexbox><Button disabled={passkey_busy} loading={passkey_busy} icon={Plus} onClick={() => void register_passkey()} size="large" type="primary">{translate('addPasskey')}</Button></Flexbox>
            <Text type="secondary">{translate('passkeyHint')}</Text>
            {passkeys.length > 0 && <Flexbox className="passkey-list" gap={8}>{passkeys.map((passkey) => <Flexbox className="passkey-row" horizontal align="center" justify="space-between" gap={12} key={passkey.id}><Flexbox gap={3}><Text>{translate('passkeyName')}</Text><Text type="secondary">{translate('created', { date: new Date(passkey.created_at).toLocaleString(locale) })}</Text></Flexbox><Button aria-label={translate('removePasskey')} disabled={passkey_busy} icon={Trash2} onClick={() => set_remove_passkey(passkey)} size="large" type="text" /></Flexbox>)}</Flexbox>}
          </Block>
        </section>
      </Flexbox>
    )}
    <Modal cancelText={translate('cancel')} okButtonProps={{ danger: true, loading: passkey_busy }} okText={translate('removePasskey')} onCancel={() => !passkey_busy && set_remove_passkey(null)} onOk={() => void delete_passkey()} open={Boolean(remove_passkey)} title={translate('removePasskeyTitle')}><Text>{translate('removePasskeyDescription')}</Text></Modal>
    <Modal cancelText={translate('close')} okText={translate('done')} onCancel={() => set_new_token(null)} onOk={() => set_new_token(null)} open={Boolean(new_token)} title={translate('tokenCreatedTitle')}><Flexbox gap={12}><Alert showIcon type="warning" message={translate('tokenOnlyShownOnce')} /><Input readOnly value={new_token?.token ?? ''} /></Flexbox></Modal>
  </main>
}
