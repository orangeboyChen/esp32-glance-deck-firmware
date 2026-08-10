'use client'

import { Alert, Block, Button, Checkbox, Empty, Flexbox, Input, Segmented, Tag, Text, toast } from '@lobehub/ui'
import { ArrowLeft, Bell, RefreshCw, Save, Trash2 } from 'lucide-react'
import { useLocale, useTranslations } from 'next-intl'
import { FormEvent, useEffect, useState } from 'react'

import { usePathname, useRouter } from '@/i18n/navigation'

type Source = { id: string; name: string }
type Device = { id: string; name: string; active_page_id: string }
type AlertRule = { id: string; name: string; source_id: string; source_name?: string; field: string; operator: Operator; threshold: string; device_ids: string[]; page_ids: string[]; severity: string; message: string; test_only: boolean; enabled: boolean; active: boolean; created_at: string }
type Operator = 'gt' | 'gte' | 'lt' | 'lte' | 'eq' | 'neq' | 'contains'

const fields = ['plan_name', 'used', 'remaining', 'total', 'unit', 'resets_at', 'status'] as const

export function AlertsManager() {
  const translate = useTranslations('Alerts')
  const locale = useLocale()
  const pathname = usePathname()
  const router = useRouter()
  const [sources, set_sources] = useState<Source[]>([])
  const [devices, set_devices] = useState<Device[]>([])
  const [alerts, set_alerts] = useState<AlertRule[]>([])
  const [loading, set_loading] = useState(true)
  const [saving, set_saving] = useState(false)
  const [error, set_error] = useState<string | null>(null)
  const [name, set_name] = useState('')
  const [source_id, set_source_id] = useState('')
  const [field, set_field] = useState<(typeof fields)[number]>('used')
  const [operator, set_operator] = useState<Operator>('gte')
  const [threshold, set_threshold] = useState('80')
  const [device_ids, set_device_ids] = useState<string[]>([])
  const [page_ids, set_page_ids] = useState('alerts')
  const [severity, set_severity] = useState('warning')
  const [message, set_message] = useState('')
  const [test_only, set_test_only] = useState(false)

  const change_locale = (next_locale: 'en' | 'zh-CN' | 'ja') => router.replace(pathname, { locale: next_locale })
  const load = async () => {
    set_loading(true)
    try {
      const [alerts_response, sources_response, devices_response] = await Promise.all([
        fetch('/api/v1/alerts', { cache: 'no-store' }),
        fetch('/api/v1/sources', { cache: 'no-store' }),
        fetch('/api/v1/devices', { cache: 'no-store' }),
      ])
      if (!alerts_response.ok || !sources_response.ok || !devices_response.ok) throw new Error('load_failed')
      set_alerts((await alerts_response.json() as { rules: AlertRule[] }).rules)
      set_sources((await sources_response.json() as { sources: Source[] }).sources)
      set_devices((await devices_response.json() as { devices: Device[] }).devices)
    } catch { set_error(translate('loadFailed')) } finally { set_loading(false) }
  }

  useEffect(() => { void load() }, [])

  const toggle_device = (id: string, checked: boolean) => set_device_ids((current) => checked ? [...current, id] : current.filter((item) => item !== id))
  const save = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!device_ids.length) { set_error(translate('targetRequired')); return }
    set_error(null)
    set_saving(true)
    try {
      const response = await fetch('/api/v1/alerts', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name: name.trim(), source_id, field, operator, threshold: threshold.trim(), device_ids, page_ids: page_ids.split(',').map((item) => item.trim()).filter(Boolean), severity, message: message.trim() || name.trim(), test_only, enabled: true }) })
      const payload = await response.json() as { error?: string }
      if (!response.ok) throw new Error(payload.error || 'save_failed')
      toast.success(translate('savedToast'))
      set_name(''); set_message(''); set_device_ids([]); set_test_only(false)
      await load()
    } catch { set_error(translate('saveFailed')) } finally { set_saving(false) }
  }

  const remove = async (id: string) => {
    const response = await fetch(`/api/v1/alerts/${id}`, { method: 'DELETE' })
    if (!response.ok) { toast.error(translate('deleteFailed')); return }
    toast.success(translate('deleted')); set_alerts((current) => current.filter((item) => item.id !== id))
  }

  return <main className="sources-shell alerts-shell">
    <header className="dashboard-header">
      <Flexbox className="dashboard-introduction" gap={10}>
        <Button icon={ArrowLeft} onClick={() => router.push('/')} size="large">{translate('back')}</Button>
        <Text className="eyebrow"><Bell aria-hidden />{translate('eyebrow')}</Text>
        <h1>{translate('title')}</h1><Text className="header-subtitle">{translate('subtitle')}</Text>
      </Flexbox>
      <Segmented aria-label={translate('language')} options={[{ label: 'EN', value: 'en' }, { label: '中文', value: 'zh-CN' }, { label: '日本語', value: 'ja' }]} value={locale} onChange={(value) => change_locale(value as 'en' | 'zh-CN' | 'ja')} />
    </header>

    <section className="sources-section" aria-labelledby="alerts-heading">
      <Flexbox horizontal align="center" justify="space-between" wrap="wrap" gap={12}><h2 id="alerts-heading">{translate('saved')}</h2><Button icon={RefreshCw} onClick={() => void load()}>{translate('refresh')}</Button></Flexbox>
      {loading ? <Text>{translate('loading')}</Text> : alerts.length === 0 ? <Empty className="empty-state" emoji="🔔" title={translate('none')} description={translate('noneDescription')} /> : <Flexbox gap={10}>{alerts.map((item) => <Block className="alert-rule-row" key={item.id} variant="outlined"><Flexbox gap={6}><Flexbox horizontal align="center" gap={8} wrap="wrap"><h3>{item.name}</h3>{item.active && <Tag color="red">{translate('active')}</Tag>}{item.test_only && <Tag color="gold">{translate('testOnly')}</Tag>}<Tag color={item.enabled ? 'green' : 'default'}>{item.enabled ? translate('enabled') : translate('disabled')}</Tag></Flexbox><Text type="secondary">{item.source_name ?? item.source_id} · {translate(`field_${item.field}`)} · {translate(`operator_${item.operator}`)} {item.threshold}</Text><Text type="secondary">{translate('targets', { count: item.device_ids.length, page: item.page_ids.join(', ') })}</Text><Text type="secondary">{item.message}</Text></Flexbox><Button aria-label={translate('delete')} icon={Trash2} onClick={() => void remove(item.id)} /></Block>)}</Flexbox>}
    </section>

    <section className="sources-section" aria-labelledby="new-alert-heading"><h2 id="new-alert-heading">{translate('new')}</h2><Text type="secondary">{translate('newDescription')}</Text>
      <form className="source-form" onSubmit={save}>
        <label htmlFor="alert-name">{translate('name')}</label><Input id="alert-name" required value={name} onChange={(event) => set_name(event.target.value)} />
        <label htmlFor="alert-source">{translate('source')}</label><select id="alert-source" required value={source_id} onChange={(event) => set_source_id(event.target.value)}><option value="">{translate('chooseSource')}</option>{sources.map((source) => <option key={source.id} value={source.id}>{source.name}</option>)}</select>
        <label htmlFor="alert-field">{translate('field')}</label><select id="alert-field" value={field} onChange={(event) => set_field(event.target.value as (typeof fields)[number])}>{fields.map((item) => <option key={item} value={item}>{translate(`field_${item}`)}</option>)}</select>
        <label>{translate('condition')}</label><Segmented options={(['gt', 'gte', 'lt', 'lte', 'eq', 'neq', 'contains'] as Operator[]).map((item) => ({ label: translate(`operator_${item}`), value: item }))} value={operator} onChange={(value) => set_operator(value as Operator)} />
        <label htmlFor="alert-threshold">{translate('threshold')}</label><Input id="alert-threshold" required value={threshold} onChange={(event) => set_threshold(event.target.value)} />
        <fieldset className="alert-targets"><legend>{translate('devices')}</legend>{devices.length === 0 ? <Text type="secondary">{translate('noDevices')}</Text> : devices.map((device) => <Checkbox checked={device_ids.includes(device.id)} key={device.id} onChange={(checked) => toggle_device(device.id, checked)}>{device.name}</Checkbox>)}</fieldset>
        <label htmlFor="alert-severity">{translate('severity')}</label><Segmented options={['info', 'warning', 'critical'].map((item) => ({ label: translate(`severity_${item}`), value: item }))} value={severity} onChange={(value) => set_severity(String(value))} />
        <label htmlFor="alert-message">{translate('message')}</label><Input id="alert-message" value={message} onChange={(event) => set_message(event.target.value)} />
        <label htmlFor="alert-page">{translate('page')}</label><Input id="alert-page" required value={page_ids} onChange={(event) => set_page_ids(event.target.value)} /><Text type="secondary">{translate('pageHelp')}</Text>
        <Checkbox checked={test_only} onChange={set_test_only}>{translate('testOnly')}</Checkbox>
        {test_only && <Alert showIcon type="warning" message={translate('testWarning')} />}
        {error && <Text role="alert" type="danger">{error}</Text>}<Button htmlType="submit" icon={Save} loading={saving} size="large" type="primary">{translate('create')}</Button>
      </form>
    </section>
  </main>
}
