import { useTranslations } from 'next-intl'

type DevicePreviewProps = {
  title: string
  previewSvg: string | null
  isSelected?: boolean
}

export function DevicePreview({ title, previewSvg, isSelected = false }: DevicePreviewProps) {
  const translate = useTranslations('Dashboard')
  const display_preview = translate('displayPreview', { title })
  if (previewSvg) {
    return (
      <img
        alt={display_preview}
        className="device-preview image-preview"
        data-selected={isSelected || undefined}
        src={`data:image/svg+xml;base64,${Buffer.from(previewSvg).toString('base64')}`}
      />
    )
  }

  return (
    <div className="device-preview" data-selected={isSelected || undefined} role="img" aria-label={display_preview}>
      <div className="preview-header">
        <span>{translate('unpublished')}</span>
        <span>1 / 4</span>
      </div>
      <div className="preview-content">
        <p className="preview-label">{translate('connectSource')}</p>
        <strong>—</strong>
        <p>{translate('publishFirstRelease')}</p>
      </div>
      <div className="preview-footer">GLANCE DECK</div>
    </div>
  )
}
