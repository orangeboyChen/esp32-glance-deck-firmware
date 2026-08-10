'use client'

import { Alert, Block, Button, Empty, Flexbox, Segmented, Tag, Text, Tooltip, toast } from '@lobehub/ui'
import { CircleAlert, ChevronRight, Monitor, Plus, Radio, RefreshCw, Wifi } from 'lucide-react'
import { useAtom, useSetAtom } from 'jotai'
import { useLocale, useTranslations } from 'next-intl'

import { DevicePreview } from './device-preview'
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
}

function status_color(status: DeviceSummary['status']) {
  if (status === 'online') return 'green'
  if (status === 'error') return 'red'
  if (status === 'offline') return 'default'
  return 'gold'
}

export function DeviceDashboard({ devices }: DeviceDashboardProps) {
  const [selected_device_id, set_selected_device_id] = useAtom(selected_device_id_atom)
  const [selected_preview_id, set_selected_preview_id] = useAtom(selected_preview_id_atom)
  const [command_feedback, set_command_feedback] = useAtom(command_feedback_atom)
  const locale = useLocale()
  const pathname = usePathname()
  const router = useRouter()
  const translate = useTranslations('Dashboard')
  const begin_command = useSetAtom(begin_device_command_atom)
  const resolve_command = useSetAtom(resolve_device_command_atom)
  const change_locale = (next_locale: 'en' | 'zh-CN' | 'ja') => router.replace(pathname, { locale: next_locale })

  const select_device = (device: DeviceSummary) => {
    set_selected_device_id(device.id)
    set_selected_preview_id(device.id)
    set_command_feedback({
      device_id: device.id,
      message: translate('previewSelected', { name: device.name }),
      phase: 'idle',
    })
  }

  const refresh_preview = async (device: DeviceSummary) => {
    begin_command(device.id)
    try {
      const response = await fetch(`/api/v1/devices/${device.id}/preview`, { cache: 'no-store' })
      if (!response.ok) throw new Error(translate('previewRejected'))
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
          <Button aria-label={translate('addDevice')} icon={Plus} size="large" type="primary">
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
          <strong>0</strong>
          <Text type="secondary">{translate('activeAlerts')}</Text>
        </Block>
        <Block className="summary-item" variant="outlined">
          <RefreshCw aria-hidden className="summary-icon" />
          <strong>—</strong>
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
            value="all"
          />
        </Flexbox>

        {devices.length === 0 ? (
          <Empty
            className="empty-state"
            emoji="🖥️"
            title={translate('noDevices')}
            description={translate('noDevicesDescription')}
            action={<Button icon={Plus} type="primary">{translate('addDevice')}</Button>}
          />
        ) : (
          <Flexbox className="device-list" gap={16}>
            {devices.map((device) => {
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
                    <DevicePreview title={device.active_page_id} previewSvg={device.preview_svg} isSelected={selected_preview_id === device.id} />
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
    </main>
  )
}
