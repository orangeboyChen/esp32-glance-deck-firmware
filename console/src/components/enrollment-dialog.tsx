'use client'

import { Button, Flexbox, Input, Modal, Text, toast } from '@lobehub/ui'
import { useTranslations } from 'next-intl'
import { FormEvent, useState } from 'react'

import { useRouter } from '@/i18n/navigation'

type EnrollmentDialogProps = {
  open: boolean
  onClose: () => void
}

export function EnrollmentDialog({ open, onClose }: EnrollmentDialogProps) {
  const translate = useTranslations('Dashboard')
  const router = useRouter()
  const [name, set_name] = useState('')
  const [pairing_code, set_pairing_code] = useState('')
  const [submitting, set_submitting] = useState(false)
  const [error, set_error] = useState<string | null>(null)

  const close = () => {
    if (submitting) return
    set_error(null)
    set_name('')
    set_pairing_code('')
    onClose()
  }

  const finish = () => {
    set_error(null)
    set_name('')
    set_pairing_code('')
    onClose()
  }

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    set_error(null)
    set_submitting(true)
    try {
      const response = await fetch('/api/v1/devices/enroll', {
        body: JSON.stringify({ name: name.trim(), pairing_code, board_model: 'ESP32-S3-RLCD-4.2' }),
        headers: { 'content-type': 'application/json' },
        method: 'POST',
      })
      const payload = await response.json() as { error?: string; device_id?: string }
      if (!response.ok) throw new Error(payload.error || 'enrollment_failed')
      toast.success(translate('deviceAdded', { id: payload.device_id || '' }))
      finish()
      router.refresh()
    } catch (submission_error) {
      const reason = submission_error instanceof Error ? submission_error.message : 'enrollment_failed'
      set_error(translate.has(reason) ? translate(reason) : translate('enrollmentFailed'))
    } finally {
      set_submitting(false)
    }
  }

  return (
    <Modal
      destroyOnHidden
      footer={
        <Flexbox horizontal justify="flex-end" gap={8}>
          <Button disabled={submitting} onClick={close} size="large">{translate('cancel')}</Button>
          <Button form="enrollment-form" htmlType="submit" loading={submitting} size="large" type="primary">{translate('pairDevice')}</Button>
        </Flexbox>
      }
      open={open}
      title={translate('addDeviceTitle')}
      width={480}
      onCancel={close}
    >
      <form className="enrollment-form" id="enrollment-form" onSubmit={submit}>
        <Text type="secondary">{translate('addDeviceDescription')}</Text>
        <label htmlFor="device-name">{translate('deviceName')}</label>
        <Input id="device-name" maxLength={128} placeholder={translate('deviceNamePlaceholder')} required value={name} onChange={(event) => set_name(event.target.value)} />
        <label htmlFor="pairing-code">{translate('pairingCode')}</label>
        <Input id="pairing-code" inputMode="numeric" maxLength={6} pattern="[0-9]{6}" placeholder={translate('pairingCodePlaceholder')} required value={pairing_code} onChange={(event) => set_pairing_code(event.target.value.replace(/[^0-9]/g, '').slice(0, 6))} />
        <Text type="secondary">{translate('pairingCodeHelp')}</Text>
        {error && <Text className="enrollment-error" type="danger">{error}</Text>}
      </form>
    </Modal>
  )
}
