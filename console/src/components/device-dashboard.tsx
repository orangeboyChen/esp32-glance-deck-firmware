'use client'

import { Alert, Block, Button, Checkbox, Empty, Flexbox, Segmented, Tag, Text, Tooltip, toast } from '@lobehub/ui'
import { ArrowDown, ArrowUp, Bell, CircleAlert, ChevronRight, Cpu, Database, Monitor, PanelsTopLeft, Plus, Radio, RefreshCw, Settings, Wifi } from 'lucide-react'
import { useAtom, useSetAtom } from 'jotai'
import { useLocale, useTranslations } from 'next-intl'
import { useEffect, useState } from 'react'

import { DevicePreview } from './device-preview'
import { EnrollmentDialog } from './enrollment-dialog'
import { usePathname, useRouter } from '@/i18n/navigation'
import {
  begin_device_command_atom,
  command_feedback_atom,
  resolve_device_command_atom,
  selected_device_id_atom,
  selected_preview_id_atom,
} from './dashboard-state'
import type { DeviceSummary } from '@/server/devices'

type DeviceDashboardProps = {
  devices: DeviceSummary[]
  summary: { active_alerts: number; source_updates_today: number }
}

type DevicePageConfiguration = {
  active_page_id: string
  desired_page_id: string
  enabled_page_ids: string[]
  available_pages: Array<{ page_id: string }>
}

function status_color(status: DeviceSummary['status']) {
  if (status === 'online') return 'green'
  if (status === 'error') return 'red'
  if (status === 'offline') return 'default'
  return 'gold'
}

export function DeviceDashboard({ devices, summary }: DeviceDashboardProps) {
  const [selected_device_id, set_selected_device_id] = useAtom(selected_device_id_atom)
  const [selected_preview_id, set_selected_preview_id] = useAtom(selected_preview_id_atom)
  const [command_feedback, set_command_feedback] = useAtom(command_feedback_atom)
  const locale = useLocale()
  const pathname = usePathname()
  const router = useRouter()
  const translate = useTranslations('Dashboard')
  const begin_command = useSetAtom(begin_device_command_atom)
  const resolve_command = useSetAtom(resolve_device_command_atom)
  const [page_configuration, set_page_configuration] = useState<DevicePageConfiguration | null>(null)
  const [page_loading, set_page_loading] = useState(false)
  const [page_saving, set_page_saving] = useState(false)
  const [enrollment_open, set_enrollment_open] = useState(false)
  const [device_filter, set_device_filter] = useState<'all' | 'attention'>('all')
  const [preview_svg_by_device, set_preview_svg_by_device] = useState<Record<string, string>>({})
  const change_locale = (next_locale: 'en' | 'zh-CN' | 'ja') => router.replace(pathname, { locale: next_locale })
  const visible_devices = device_filter === 'all'
    ? devices
    : devices.filter((device) => device.status !== 'online' || device.ota_status !== null || device.active_page_id === 'alerts')

  const select_device = (device: DeviceSummary) => {
    set_selected_device_id(device.id)
    set_selected_preview_id(device.id)
    set_command_feedback({
      device_id: device.id,
      message: translate('previewSelected', { name: device.name }),
      phase: 'idle',
    })
  }

  useEffect(() => {
    if (!selected_device_id) {
      set_page_configuration(null)
      return
    }
    let cancelled = false
    set_page_loading(true)
    void fetch(`/api/v1/devices/${selected_device_id}/pages`, { cache: 'no-store' })
      .then(async (response) => response.ok ? response.json() as Promise<DevicePageConfiguration> : null)
      .then((configuration) => { if (!cancelled) set_page_configuration(configuration) })
      .catch(() => { if (!cancelled) set_page_configuration(null) })
      .finally(() => { if (!cancelled) set_page_loading(false) })
    return () => { cancelled = true }
  }, [selected_device_id])

  const toggle_page = (page_id: string, checked: boolean) => {
    if (!page_configuration) return
    const enabled_page_ids = checked
      ? [...page_configuration.enabled_page_ids, page_id]
      : page_configuration.enabled_page_ids.filter((item) => item !== page_id)
    if (!enabled_page_ids.length || enabled_page_ids.length > 10) return
    set_page_configuration({ ...page_configuration, enabled_page_ids, desired_page_id: enabled_page_ids.includes(page_configuration.desired_page_id) ? page_configuration.desired_page_id : enabled_page_ids[0] })
  }

  const move_page = (page_id: string, direction: -1 | 1) => {
    if (!page_configuration) return
    const index = page_configuration.enabled_page_ids.indexOf(page_id)
    const next_index = index + direction
    if (index < 0 || next_index < 0 || next_index >= page_configuration.enabled_page_ids.length) return
    const enabled_page_ids = [...page_configuration.enabled_page_ids]
    ;[enabled_page_ids[index], enabled_page_ids[next_index]] = [enabled_page_ids[next_index], enabled_page_ids[index]]
    set_page_configuration({ ...page_configuration, enabled_page_ids })
  }

  const save_pages = async () => {
    if (!selected_device_id || !page_configuration) return
    set_page_saving(true)
    try {
      const response = await fetch(`/api/v1/devices/${selected_device_id}/pages`, { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ enabled_page_ids: page_configuration.enabled_page_ids, desired_page_id: page_configuration.desired_page_id }) })
      if (!response.ok) throw new Error('page_configuration_rejected')
      set_page_configuration(await response.json() as DevicePageConfiguration)
      toast.success(translate('pagesSaved'))
    } catch {
      toast.error(translate('pagesSaveFailed'))
    } finally {
      set_page_saving(false)
    }
  }

  const refresh_preview = async (device: DeviceSummary) => {
    begin_command(device.id)
    try {
      const response = await fetch(`/api/v1/devices/${device.id}/preview`, { cache: 'no-store' })
      if (!response.ok) throw new Error(translate('previewRejected'))
      const preview_svg = await response.text()
      set_preview_svg_by_device((previews) => ({ ...previews, [device.id]: preview_svg }))
      resolve_command({ device_id: device.id, message: translate('previewCurrent'), phase: 'accepted' })
      toast.success(translate('previewRefreshed'))
    } catch (error) {
      const message = error instanceof Error ? error.message : translate('previewRefreshFailed')
      resolve_command({ device_id: device.id, message, phase: 'error' })
      toast.error(translate('previewRefreshFailed'))
    }
  }

  return (
    <main className="dashboard-shell">
      <header className="dashboard-header">
        <Flexbox className="dashboard-introduction" gap={10}>
          <Text className="eyebrow"><Radio aria-hidden />{translate('controlPlane')}</Text>
          <h1>{translate('title')}</h1>
          <Text className="header-subtitle">{translate('subtitle')}</Text>
        </Flexbox>
        <Flexbox className="header-actions" horizontal gap={12} wrap="wrap">
          <Segmented aria-label={translate('language')} options={[{ label: 'EN', value: 'en' }, { label: '中文', value: 'zh-CN' }, { label: '日本語', value: 'ja' }]} onChange={(value) => change_locale(value as 'en' | 'zh-CN' | 'ja')} value={locale} />
          <Button icon={Database} onClick={() => router.push('/sources')} size="large">{translate('sources')}</Button>
          <Button icon={Cpu} onClick={() => router.push('/firmware')} size="large">{translate('firmware')}</Button>
          <Button icon={PanelsTopLeft} onClick={() => router.push('/displays')} size="large">{translate('displays')}</Button>
          <Button icon={Bell} onClick={() => router.push('/alerts')} size="large">{translate('alerts')}</Button>
          <Button icon={Settings} onClick={() => router.push('/settings')} size="large">{translate('settings')}</Button>
          <Button aria-label={translate('addDevice')} icon={Plus} onClick={() => set_enrollment_open(true)} size="large" type="primary">
            {translate('addDevice')}
          </Button>
        </Flexbox>
      </header>

      <section aria-label={translate('systemSummary')} className="summary-grid">
        <Block className="summary-item" variant="outlined">
          <Monitor aria-hidden className="summary-icon" />
          <strong>{devices.length}</strong>
          <Text type="secondary">{translate('registeredDevices')}</Text>
        </Block>
        <Block className="summary-item" variant="outlined">
          <CircleAlert aria-hidden className="summary-icon" />
          <strong>{summary.active_alerts}</strong>
          <Text type="secondary">{translate('activeAlerts')}</Text>
        </Block>
        <Block className="summary-item" variant="outlined">
          <RefreshCw aria-hidden className="summary-icon" />
          <strong>{summary.source_updates_today}</strong>
          <Text type="secondary">{translate('sourceUpdatesToday')}</Text>
        </Block>
      </section>

      <section aria-labelledby="devices-heading" className="devices-section">
        <Flexbox className="section-header" horizontal align="center" justify="space-between" gap={16} wrap="wrap">
          <Flexbox gap={4}>
            <h2 id="devices-heading">{translate('devices')}</h2>
            <Text type="secondary">{translate('devicesDescription')}</Text>
          </Flexbox>
          <Segmented
            aria-label={translate('devices')}
            options={[
              { label: translate('allDevices'), value: 'all' },
              { label: translate('needsAttention'), value: 'attention' },
            ]}
            onChange={(value) => set_device_filter(value as 'all' | 'attention')}
            value={device_filter}
          />
        </Flexbox>

        {visible_devices.length === 0 ? (
          <Empty
            className="empty-state"
            emoji="🖥️"
            title={translate('noDevices')}
            description={translate('noDevicesDescription')}
            action={<Button icon={Plus} onClick={() => set_enrollment_open(true)} type="primary">{translate('addDevice')}</Button>}
          />
        ) : (
          <Flexbox className="device-list" gap={16}>
            {visible_devices.map((device) => {
              const is_selected = selected_device_id === device.id
              const status_label = device.status === 'online'
                ? translate('online')
                : device.status === 'enrolling'
                  ? translate('needsPairing')
                  : device.status
              const is_refreshing = command_feedback?.device_id === device.id && command_feedback.phase === 'submitting'

              return (
                <Block
                  aria-label={`${device.name}, ${status_label}`}
                  className="device-card"
                  data-selected={is_selected || undefined}
                  key={device.id}
                  shadow={is_selected}
                  variant="outlined"
                >
                  <button className="preview-select" onClick={() => select_device(device)} type="button">
                    <DevicePreview title={device.active_page_id} previewSvg={preview_svg_by_device[device.id] ?? device.preview_svg} isSelected={selected_preview_id === device.id} />
                  </button>

                  <Flexbox className="device-meta" gap={16}>
                    <Flexbox horizontal align="center" justify="space-between" gap={12} wrap="wrap">
                      <Flexbox horizontal align="center" gap={8}>
                        <Tag color={status_color(device.status)} icon={device.status === 'online' ? <Wifi aria-hidden size={14} /> : <CircleAlert aria-hidden size={14} />}>{status_label}</Tag>
                        <Text className="page-id" type="secondary">{device.active_page_id}</Text>
                      </Flexbox>
                      <Text className="device-id" type="secondary">{device.id}</Text>
                    </Flexbox>
                    <Flexbox gap={4}>
                      <h3>{device.name}</h3>
                      <Text type="secondary">{device.firmware_version ?? translate('firmwarePending')}</Text>
                    </Flexbox>
                    <Flexbox className="device-actions" horizontal gap={10} wrap="wrap">
                      <Button icon={ChevronRight} iconPosition="end" onClick={() => select_device(device)} size="large" type="primary">
                        {translate('openDevice')}
                      </Button>
                      <Tooltip title={translate('refreshPreview')}>
                        <Button
                          aria-label={translate('refreshPreview')}
                          icon={RefreshCw}
                          loading={is_refreshing}
                          onClick={() => refresh_preview(device)}
                          size="large"
                        />
                      </Tooltip>
                    </Flexbox>
                  </Flexbox>
                </Block>
              )
            })}
          </Flexbox>
        )}
      </section>

      {selected_device_id && (
        <section aria-labelledby="page-control-heading" className="page-control-section">
          <Flexbox gap={4}>
            <h2 id="page-control-heading">{translate('pageControl')}</h2>
            <Text type="secondary">{translate('pageControlDescription')}</Text>
          </Flexbox>
          {page_loading ? <Text>{translate('pagesLoading')}</Text> : page_configuration ? (
            <Block className="page-control-card" variant="outlined">
              <Flexbox className="page-state" horizontal gap={12} wrap="wrap">
                <Tag>{translate('confirmedPage', { page: page_configuration.active_page_id })}</Tag>
                <Tag color={page_configuration.active_page_id === page_configuration.desired_page_id ? 'green' : 'gold'}>{translate('targetPage', { page: page_configuration.desired_page_id })}</Tag>
                <Text type="secondary">{translate('pageCount', { count: page_configuration.enabled_page_ids.length })}</Text>
              </Flexbox>
              <Flexbox className="page-options" gap={8}>
                {page_configuration.available_pages.map((page) => {
                  const enabled = page_configuration.enabled_page_ids.includes(page.page_id)
                  const index = page_configuration.enabled_page_ids.indexOf(page.page_id)
                  return <Flexbox className="page-option" horizontal align="center" justify="space-between" key={page.page_id} gap={12}>
                    <Checkbox checked={enabled} disabled={!enabled && page_configuration.enabled_page_ids.length >= 10} onChange={(checked) => toggle_page(page.page_id, checked)}>{page.page_id}</Checkbox>
                    <Flexbox horizontal gap={6}>
                      <Button aria-label={translate('movePageUp', { page: page.page_id })} disabled={!enabled || index === 0} icon={ArrowUp} onClick={() => move_page(page.page_id, -1)} size="large" />
                      <Button aria-label={translate('movePageDown', { page: page.page_id })} disabled={!enabled || index === page_configuration.enabled_page_ids.length - 1} icon={ArrowDown} onClick={() => move_page(page.page_id, 1)} size="large" />
                      <Button disabled={!enabled} onClick={() => set_page_configuration({ ...page_configuration, desired_page_id: page.page_id })} size="large" type={page_configuration.desired_page_id === page.page_id ? 'primary' : 'default'}>{translate('showPage')}</Button>
                    </Flexbox>
                  </Flexbox>
                })}
              </Flexbox>
              <Button disabled={page_saving} loading={page_saving} onClick={save_pages} size="large" type="primary">{translate('savePages')}</Button>
            </Block>
          ) : <Text type="secondary">{translate('pagesUnavailable')}</Text>}
        </section>
      )}

      {command_feedback && (
        <Alert
          aria-live="polite"
          className="live-feedback"
          closable
          closeText={translate('closeNotice')}
          message={command_feedback.message || translate('refreshPreview')}
          onClose={() => set_command_feedback(null)}
          showIcon
          type={command_feedback.phase === 'error' ? 'error' : command_feedback.phase === 'accepted' ? 'success' : 'info'}
        />
      )}
      <EnrollmentDialog open={enrollment_open} onClose={() => set_enrollment_open(false)} />
    </main>
  )
}
