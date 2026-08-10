'use client'

import { Alert, Block, Button, Empty, Flexbox, Segmented, Tag, Text, toast } from '@lobehub/ui'
import { useAtom, useSetAtom } from 'jotai'

import { DevicePreview } from './device-preview'
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

export function DeviceDashboard({ devices }: DeviceDashboardProps) {
  const [selected_device_id, set_selected_device_id] = useAtom(selected_device_id_atom)
  const [selected_preview_id, set_selected_preview_id] = useAtom(selected_preview_id_atom)
  const [command_feedback, set_command_feedback] = useAtom(command_feedback_atom)
  const begin_command = useSetAtom(begin_device_command_atom)
  const resolve_command = useSetAtom(resolve_device_command_atom)

  const select_device = (device: DeviceSummary) => {
    set_selected_device_id(device.id)
    set_selected_preview_id(device.id)
    set_command_feedback({
      device_id: device.id,
      message: `${device.name} is selected. Its displayed document is shown below.`,
      phase: 'idle',
    })
  }

  const refresh_preview = async (device: DeviceSummary) => {
    begin_command(device.id)
    try {
      const response = await fetch(`/api/v1/devices/${device.id}/preview`, { cache: 'no-store' })
      if (!response.ok) throw new Error('The preview service did not accept this request.')
      resolve_command({ device_id: device.id, message: 'Preview is current with the control plane.', phase: 'accepted' })
      toast.success('Preview refreshed')
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unable to refresh the preview.'
      resolve_command({ device_id: device.id, message, phase: 'error' })
      toast.error('Preview refresh failed')
    }
  }

  return (
    <Flexbox className="shell" gap={32}>
      <Flexbox className="header" horizontal justify="space-between" gap={24}>
        <Flexbox gap={8}>
          <Text className="eyebrow">GLANCE DECK / CONTROL PLANE</Text>
          <h1>Your devices, at a glance.</h1>
          <Text className="header-subtitle">A shared, device-accurate preview for displays, alerts, and Home Assistant automation.</Text>
        </Flexbox>
        <Button size="large" type="primary">Add device</Button>
      </Flexbox>

      <Flexbox className="summary" horizontal gap={20} aria-label="System summary">
        <Block className="summary-item" variant="borderless"><strong>{devices.length}</strong><Text>registered devices</Text></Block>
        <Block className="summary-item" variant="borderless"><strong>0</strong><Text>active alerts</Text></Block>
        <Block className="summary-item" variant="borderless"><strong>—</strong><Text>source updates today</Text></Block>
      </Flexbox>

      <Flexbox gap={16} aria-label="Devices">
        <Flexbox horizontal align="center" justify="space-between" gap={16}>
          <Flexbox gap={2}><h2 className="section-title">Devices</h2><Text type="secondary">Select a device to inspect the exact display document it last confirmed.</Text></Flexbox>
          <Segmented options={[{ label: 'All devices', value: 'all' }, { label: 'Needs attention', value: 'attention' }]} value="all" />
        </Flexbox>
        {devices.length === 0 && (
          <Empty className="empty-state" emoji="🖥️" title="No paired devices" description="Put a device into enrollment mode, then choose Add device." action={<Button type="primary">Add device</Button>} />
        )}
        {devices.map((device) => {
          const is_selected = selected_device_id === device.id
          const status_label = device.status === 'online' ? 'Online' : device.status === 'enrolling' ? 'Needs pairing' : device.status
          return (
            <Block className="device-card" key={device.id} data-selected={is_selected || undefined} variant="outlined" shadow>
              <Button className="preview-select" onClick={() => select_device(device)} type="text" aria-label={`Select ${device.name} preview`}>
                <DevicePreview title={device.active_page_id} previewSvg={device.preview_svg} isSelected={selected_preview_id === device.id} />
              </Button>
              <Flexbox className="device-meta" gap={12}>
                <Flexbox horizontal align="center" gap={8}>
                  <Tag color={device.status === 'online' ? 'green' : 'gold'}>{status_label}</Tag>
                  <Text type="secondary">{device.active_page_id}</Text>
                </Flexbox>
                <h3>{device.name}</h3>
                <Text type="secondary">{device.id} · {device.firmware_version ?? 'Firmware pending'}</Text>
                <Flexbox horizontal gap={8} wrap="wrap">
                  <Button onClick={() => select_device(device)} size="large" type="primary">Open device</Button>
                  <Button loading={command_feedback?.device_id === device.id && command_feedback.phase === 'submitting'} onClick={() => refresh_preview(device)} size="large">Refresh preview</Button>
                </Flexbox>
              </Flexbox>
            </Block>
          )
        })}
      </Flexbox>
      {command_feedback && <Alert className="live-feedback" closable onClose={() => set_command_feedback(null)} message={command_feedback.message} type={command_feedback.phase === 'error' ? 'error' : command_feedback.phase === 'accepted' ? 'success' : 'info'} showIcon />}
    </Flexbox>
  )
}
