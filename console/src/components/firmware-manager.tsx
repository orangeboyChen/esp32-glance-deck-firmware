'use client'

import { Alert, Block, Button, Checkbox, Empty, Flexbox, Input, Modal, Segmented, Tag, Text, toast } from '@lobehub/ui'
import { ArrowLeft, Cpu, Download, RefreshCw, ShieldCheck } from 'lucide-react'
import { useLocale, useTranslations } from 'next-intl'
import { useEffect, useState } from 'react'

import { usePathname, useRouter } from '@/i18n/navigation'

type FirmwareRelease = { id: string; version: string; board_model: string; channel: 'stable' | 'beta' | 'test'; verified_at: string; manifest_url: string }
type Device = { id: string; name: string; board_model: string; firmware_version: string | null; status: string; ota_status: string | null; ota_job_id: string | null; power_source: string | null; battery_percent: number | null }

export function FirmwareManager() {
  const translate = useTranslations('Firmware')
  const locale = useLocale()
  const pathname = usePathname()
  const router = useRouter()
  const [releases, set_releases] = useState<FirmwareRelease[]>([])
  const [devices, set_devices] = useState<Device[]>([])
  const [loading, set_loading] = useState(true)
  const [error, set_error] = useState<string | null>(null)
  const [selection, set_selection] = useState<{ release: FirmwareRelease; device: Device } | null>(null)
  const [installing, set_installing] = useState(false)
  const [rollout_release_id, set_rollout_release_id] = useState('')
  const [rollout_percentage, set_rollout_percentage] = useState('100')
  const [rollout_device_ids, set_rollout_device_ids] = useState<string[]>([])
  const [rollout_busy, set_rollout_busy] = useState(false)

  const change_locale = (next_locale: 'en' | 'zh-CN' | 'ja') => router.replace(pathname, { locale: next_locale })
  const load = async () => {
    set_loading(true)
    set_error(null)
    try {
      const [release_response, device_response] = await Promise.all([fetch('/api/v1/firmware/releases', { cache: 'no-store' }), fetch('/api/v1/devices', { cache: 'no-store' })])
      if (!release_response.ok || !device_response.ok) throw new Error('load_failed')
      set_releases((await release_response.json() as { releases: FirmwareRelease[] }).releases)
      set_devices((await device_response.json() as { devices: Device[] }).devices)
    } catch { set_error(translate('loadFailed')) } finally { set_loading(false) }
  }
  useEffect(() => { void load() }, [])

  const start_rollout = async () => {
    if (!rollout_release_id || !rollout_device_ids.length) return
    set_rollout_busy(true)
    try {
      const response = await fetch('/api/v1/ota/rollouts', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ firmware_release_id: rollout_release_id, device_ids: rollout_device_ids, percentage: Number(rollout_percentage) }) })
      const payload = await response.json() as { error?: string; selected_count?: number }
      if (!response.ok) throw new Error(payload.error || 'rolloutFailed')
      toast.success(translate('rolloutQueued', { count: payload.selected_count ?? 0 }))
      set_rollout_device_ids([])
      await load()
    } catch (rollout_error) {
      const code = rollout_error instanceof Error ? rollout_error.message : 'rolloutFailed'
      toast.error(translate.has(code) ? translate(code) : translate('rolloutFailed'))
    } finally { set_rollout_busy(false) }
  }

  const install = async () => {
    if (!selection) return
    set_installing(true)
    try {
      const response = await fetch(`/api/v1/devices/${selection.device.id}/ota`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ firmware_release_id: selection.release.id }) })
      const payload = await response.json() as { error?: string }
      if (!response.ok) throw new Error(payload.error || 'ota_failed')
      toast.success(translate('queued', { device: selection.device.name, version: selection.release.version }))
      set_selection(null)
      await load()
    } catch (install_error) {
      const code = install_error instanceof Error ? install_error.message : 'ota_failed'
      toast.error(translate.has(code) ? translate(code) : translate('otaFailed'))
    } finally { set_installing(false) }
  }

  const update_job = async (device: Device, action: 'cancel' | 'rollback') => {
    if (!device.ota_job_id) return
    try {
      const response = await fetch(`/api/v1/ota/jobs/${device.ota_job_id}`, { method: 'PATCH', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ action }) })
      const payload = await response.json() as { error?: string }
      if (!response.ok) throw new Error(payload.error || 'otaJobActionFailed')
      toast.success(translate(action === 'cancel' ? 'cancelled' : 'rollbackQueued'))
      await load()
    } catch (job_error) {
      const code = job_error instanceof Error ? job_error.message : 'otaJobActionFailed'
      toast.error(translate.has(code) ? translate(code) : translate('otaJobActionFailed'))
    }
  }

  return <main className="sources-shell">
    <header className="dashboard-header"><Flexbox className="dashboard-introduction" gap={10}><Button icon={ArrowLeft} onClick={() => router.push('/')} size="large">{translate('back')}</Button><Text className="eyebrow"><Cpu aria-hidden />{translate('eyebrow')}</Text><h1>{translate('title')}</h1><Text className="header-subtitle">{translate('subtitle')}</Text></Flexbox><Segmented aria-label={translate('language')} options={[{ label: 'EN', value: 'en' }, { label: '中文', value: 'zh-CN' }, { label: '日本語', value: 'ja' }]} value={locale} onChange={(value) => change_locale(value as 'en' | 'zh-CN' | 'ja')} /></header>
    <section className="sources-section" aria-labelledby="releases-heading"><Flexbox horizontal align="center" justify="space-between" wrap="wrap" gap={12}><h2 id="releases-heading">{translate('verifiedReleases')}</h2><Button icon={RefreshCw} onClick={() => void load()}>{translate('refresh')}</Button></Flexbox>{error && <Alert message={error} showIcon type="error" />}{loading ? <Text>{translate('loading')}</Text> : releases.length === 0 ? <Empty className="empty-state" emoji="◌" title={translate('noReleases')} description={translate('noReleasesDescription')} /> : <Flexbox gap={16}>{releases.map((release) => <Block className="release-card" key={release.id} variant="outlined"><Flexbox horizontal align="center" justify="space-between" wrap="wrap" gap={8}><Flexbox gap={3}><h3>{release.version}</h3><Text type="secondary">{release.board_model}</Text></Flexbox><Tag color={release.channel === 'stable' ? 'green' : 'gold'} icon={<ShieldCheck aria-hidden size={14} />}>{release.channel}</Tag></Flexbox><Text type="secondary">{translate('verified', { date: new Date(release.verified_at).toLocaleString(locale) })}</Text><a href={release.manifest_url} rel="noreferrer" target="_blank">{translate('viewManifest')}</a><div className="compatible-devices"><Text strong>{translate('compatibleDevices')}</Text>{devices.filter((device) => device.board_model === release.board_model).length === 0 ? <Text type="secondary">{translate('noCompatibleDevices')}</Text> : devices.filter((device) => device.board_model === release.board_model).map((device) => { const status = device.ota_status || 'none'; const cancellable = ['awaiting_confirmation', 'queued'].includes(status); const rollbackable = ['healthy', 'failed'].includes(status); return <Flexbox className="firmware-device-row" horizontal align="center" justify="space-between" gap={12} key={device.id} wrap="wrap"><Flexbox gap={2}><Text>{device.name}</Text><Text type="secondary">{translate('deviceVersion', { version: device.firmware_version || translate('unknown') })} · {translate('otaStatus', { status: status === 'none' ? translate('none') : status })}</Text></Flexbox><Flexbox horizontal gap={8} wrap="wrap"><Button disabled={['queued', 'sent', 'downloading', 'verifying', 'rebooting'].includes(status)} icon={Download} onClick={() => set_selection({ device, release })} size="large">{translate('startUpdate')}</Button>{cancellable && <Button onClick={() => void update_job(device, 'cancel')} size="large">{translate('cancelUpdate')}</Button>}{rollbackable && <Button onClick={() => void update_job(device, 'rollback')} size="large">{translate('rollback')}</Button>}</Flexbox></Flexbox> })}</div></Block>)}</Flexbox>}</section>
    <section className="sources-section" aria-labelledby="rollout-heading"><h2 id="rollout-heading">{translate('rolloutTitle')}</h2><Text type="secondary">{translate('rolloutDescription')}</Text><Flexbox className="source-form" gap={8}><label htmlFor="rollout-release">{translate('rolloutRelease')}</label><select id="rollout-release" value={rollout_release_id} onChange={(event) => set_rollout_release_id(event.target.value)}><option value="">{translate('chooseRelease')}</option>{releases.map((release) => <option key={release.id} value={release.id}>{release.version} · {release.channel}</option>)}</select><label htmlFor="rollout-percentage">{translate('rolloutPercentage')}</label><Input id="rollout-percentage" type="number" min={1} max={100} value={rollout_percentage} onChange={(event) => set_rollout_percentage(event.target.value)} /><fieldset className="alert-targets"><legend>{translate('rolloutDevices')}</legend>{devices.map((device) => <Checkbox checked={rollout_device_ids.includes(device.id)} key={device.id} onChange={(checked) => set_rollout_device_ids(checked ? [...rollout_device_ids, device.id] : rollout_device_ids.filter((id) => id !== device.id))}>{device.name}</Checkbox>)}</fieldset><Button disabled={!rollout_release_id || !rollout_device_ids.length} loading={rollout_busy} onClick={() => void start_rollout()} type="primary">{translate('startRollout')}</Button></Flexbox></section>
    <Modal open={Boolean(selection)} title={translate('confirmTitle')} okText={translate('startUpdate')} okButtonProps={{ loading: installing }} cancelText={translate('cancel')} onCancel={() => !installing && set_selection(null)} onOk={() => void install()}><Flexbox gap={12}><Text>{selection && translate('confirmDescription', { device: selection.device.name, version: selection.release.version })}</Text><Alert showIcon type="warning" message={translate('confirmWarning')} /></Flexbox></Modal>
  </main>
}
