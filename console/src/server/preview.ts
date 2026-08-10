export const fallback_preview_svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="300" height="400" viewBox="0 0 300 400">
  <rect width="300" height="400" fill="#f2f4ed"/>
  <rect x="8" y="8" width="284" height="384" fill="none" stroke="#26322a" stroke-width="4"/>
  <text x="28" y="48" font-family="Arial" font-size="12" font-weight="700" fill="#26322a">GLANCE DECK</text>
  <text x="28" y="95" font-family="Arial" font-size="12" fill="#627168">WAITING FOR PAIRING</text>
  <text x="28" y="177" font-family="Georgia" font-size="42" fill="#26322a">—</text>
  <text x="28" y="224" font-family="Arial" font-size="13" fill="#627168">Pair a device, then publish</text>
  <text x="28" y="244" font-family="Arial" font-size="13" fill="#627168">its first display release.</text>
  <text x="28" y="365" font-family="Arial" font-size="10" font-weight="700" fill="#26322a">300 × 400</text>
</svg>`.trim()

export type Display_document = {
  title: string
  subtitle?: string
  lines?: Array<{ label: string; value: string }>
}

function escape_xml(value: string) {
  return value.replace(/[<>&"']/g, (character) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;', "'": '&apos;' })[character] ?? character)
}

export function render_display_preview(document: Display_document) {
  const title = escape_xml(document.title)
  const subtitle = document.subtitle ? escape_xml(document.subtitle) : ''
  const lines = (document.lines ?? []).slice(0, 7).map((line, index) => {
    const y = 135 + index * 34
    return `<text x="28" y="${y}" font-family="Arial" font-size="13" fill="#627168">${escape_xml(line.label)}</text><text x="272" y="${y}" text-anchor="end" font-family="Arial" font-size="16" font-weight="700" fill="#26322a">${escape_xml(line.value)}</text>`
  }).join('')
  return `<svg xmlns="http://www.w3.org/2000/svg" width="300" height="400" viewBox="0 0 300 400"><rect width="300" height="400" fill="#f2f4ed"/><rect x="8" y="8" width="284" height="384" fill="none" stroke="#26322a" stroke-width="4"/><text x="28" y="48" font-family="Arial" font-size="12" font-weight="700" fill="#26322a">GLANCE DECK</text><text x="28" y="88" font-family="Georgia" font-size="27" fill="#26322a">${title}</text><text x="28" y="111" font-family="Arial" font-size="12" fill="#627168">${subtitle}</text>${lines}<line x1="28" x2="272" y1="354" y2="354" stroke="#9ba89f"/><text x="28" y="374" font-family="Arial" font-size="10" fill="#627168">IMMUTABLE DISPLAY RELEASE</text></svg>`
}
