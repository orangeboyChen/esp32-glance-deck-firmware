'use client'

import { Alert, Block, Button, Empty, Flexbox, Modal, Segmented, Tag, Text, toast } from '@lobehub/ui'
import { ArrowLeft, Cpu, Download, RefreshCw, ShieldCheck } from 'lucide-react'
import { useLocale, useTranslations } from 'next-intl'
import { useEffect, useState } from 'react'

import { usePathname, useRouter } from '@/i18n/navigation'

type FirmwareRelease = { id: string; version: string; board_model: string; channel: 'stable' | 'beta' | 'test'; verified_at: string; manifest_url: string }
type Device = { id: string; name: string; board_model: string; firmware_version: string | null; status: string; ota_status: string | null; power_source: string | null; battery_percent: number | null }

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

  return <main className="sources-shell">
    <header className="dashboard-header"><Flexbox className="dashboard-introduction" gap={10}><Button icon={ArrowLeft} onClick={() => router.push('/')} size="large">{translate('back')}</Button><Text className="eyebrow"><Cpu aria-hidden />{translate('eyebrow')}</Text><h1>{translate('title')}</h1><Text className="header-subtitle">{translate('subtitle')}</Text></Flexbox><Segmented aria-label={translate('language')} options={[{ label: 'EN', value: 'en' }, { label: '中文', value: 'zh-CN' }, { label: '日本語', value: 'ja' }]} value={locale} onChange={(value) => change_locale(value as 'en' | 'zh-CN' | 'ja')} /></header>
    <section className="sources-section" aria-labelledby="releases-heading"><Flexbox horizontal align="center" justify="space-between" wrap="wrap" gap={12}><h2 id="releases-heading">{translate('verifiedReleases')}</h2><Button icon={RefreshCw} onClick={() => void load()}>{translate('refresh')}</Button></Flexbox>{error && <Alert message={error} showIcon type="error" />}{loading ? <Text>{translate('loading')}</Text> : releases.length === 0 ? <Empty className="empty-state" emoji="◌" title={translate('noReleases')} description={translate('noReleasesDescription')} /> : <Flexbox gap={16}>{releases.map((release) => <Block className="release-card" key={release.id} variant="outlined"><Flexbox horizontal align="center" justify="space-between" wrap="wrap" gap={8}><Flexbox gap={3}><h3>{release.version}</h3><Text type="secondary">{release.board_model}</Text></Flexbox><Tag color={release.channel === 'stable' ? 'green' : 'gold'} icon={<ShieldCheck aria-hidden size={14} />}>{release.channel}</Tag></Flexbox><Text type="secondary">{translate('verified', { date: new Date(release.verified_at).toLocaleString(locale) })}</Text><a href={release.manifest_url} rel="noreferrer" target="_blank">{translate('viewManifest')}</a><div className="compatible-devices"><Text strong>{translate('compatibleDevices')}</Text>{devices.filter((device) => device.board_model === release.board_model).length === 0 ? <Text type="secondary">{translate('noCompatibleDevices')}</Text> : devices.filter((device) => device.board_model === release.board_model).map((device) => <Flexbox className="firmware-device-row" horizontal align="center" justify="space-between" gap={12} key={device.id} wrap="wrap"><Flexbox gap={2}><Text>{device.name}</Text><Text type="secondary">{translate('deviceVersion', { version: device.firmware_version || translate('unknown') })} · {translate('otaStatus', { status: device.ota_status || translate('none') })}</Text></Flexbox><Button disabled={device.ota_status === 'queued' || device.ota_status === 'sent' || device.ota_status === 'downloading' || device.ota_status === 'verifying' || device.ota_status === 'rebooting'} icon={Download} onClick={() => set_selection({ device, release })} size="large">{translate('startUpdate')}</Button></Flexbox>)}</div></Block>)}</Flexbox>}</section>
    <Modal open={Boolean(selection)} title={translate('confirmTitle')} okText={translate('startUpdate')} okButtonProps={{ loading: installing }} cancelText={translate('cancel')} onCancel={() => !installing && set_selection(null)} onOk={() => void install()}><Flexbox gap={12}><Text>{selection && translate('confirmDescription', { device: selection.device.name, version: selection.release.version })}</Text><Alert showIcon type="warning" message={translate('confirmWarning')} /></Flexbox></Modal>
  </main>
}
