type DevicePreviewProps = {
  title: string
  previewSvg: string | null
  isSelected?: boolean
}

export function DevicePreview({ title, previewSvg, isSelected = false }: DevicePreviewProps) {
  if (previewSvg) {
    return <img className="device-preview image-preview" data-selected={isSelected || undefined} src={`data:image/svg+xml;base64,${Buffer.from(previewSvg).toString('base64')}`} alt={`${title} display preview`} />
  }
  return (
    <div className="device-preview" data-selected={isSelected || undefined} aria-label={`${title} display preview`}>
      <div className="preview-header">
        <span>{title.toUpperCase()}</span>
        <span>1 / 4</span>
      </div>
      <div className="preview-content">
        <p className="preview-label">Connect a source</p>
        <strong>—</strong>
        <p>Pair a device, then publish its first display release.</p>
      </div>
      <div className="preview-footer">GLANCE DECK</div>
    </div>
  )
}
