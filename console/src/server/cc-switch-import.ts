const MAX_EXPORT_BYTES = 64 * 1024
const sensitive_key = /(?:authorization|cookie|token|secret|password|api[_-]?key|credential|bearer)/i
const variable_name = /\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}|\$\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}/g

type JsonRecord = Record<string, unknown>

export type CcSwitchImportPreview = {
  body: unknown
  extractor_present: boolean
  extractor_target_names: string[]
  headers: Record<string, string>
  mapping_required: true
  method: 'GET' | 'POST'
  refresh_interval_seconds: number | null
  request_path: string
  secret_variable_names: string[]
  url: string
}

function record(value: unknown): JsonRecord | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as JsonRecord : null
}

function string(value: unknown) {
  return typeof value === 'string' ? value : undefined
}

function redact_value(value: unknown, key?: string): unknown {
  if (key && sensitive_key.test(key)) return '[REDACTED]'
  if (Array.isArray(value)) return value.map((item) => redact_value(item))
  const object = record(value)
  if (object) return Object.fromEntries(Object.entries(object).map(([entry_key, entry_value]) => [entry_key, redact_value(entry_value, entry_key)]))
  if (typeof value === 'string' && /\b(?:bearer|basic)\s+\S+/i.test(value)) return '[REDACTED]'
  return value
}

function variables_in(value: unknown) {
  const names = new Set<string>()
  const collect = (item: unknown) => {
    if (typeof item === 'string') {
      for (const match of item.matchAll(variable_name)) names.add(match[1] || match[2])
    } else if (Array.isArray(item)) item.forEach(collect)
    else {
      const object = record(item)
      if (object) Object.values(object).forEach(collect)
    }
  }
  collect(value)
  return [...names].sort()
}

function extractor_targets(extractor: string | undefined) {
  if (!extractor) return []
  const returned = /\breturn\s*(?:\(\s*)?\{([\s\S]{0,8192}?)\}(?:\s*\))?\s*;?/m.exec(extractor)?.[1]
  if (!returned) return []
  const names = new Set<string>()
  for (const match of returned.matchAll(/(?:^|,)\s*(?:([A-Za-z_$][\w$]*)|['"]([^'"]+)['"])\s*:/g)) names.add(match[1] || match[2])
  return [...names].sort()
}

function request_from(value: JsonRecord) {
  return record(value.request) ?? record(value.config) ?? value
}

export function preview_cc_switch_import(exported: unknown): CcSwitchImportPreview {
  const serialized = JSON.stringify(exported)
  if (!serialized || serialized.length > MAX_EXPORT_BYTES) throw new Error('cc_switch_export_too_large')
  const root = record(exported)
  if (!root) throw new Error('cc_switch_export_invalid')
  const request = request_from(root)
  const url = string(request.url) ?? string(request.request_url) ?? string(root.url)
  if (!url) throw new Error('cc_switch_request_url_missing')
  let parsed: URL
  try { parsed = new URL(url) } catch { throw new Error('cc_switch_request_url_invalid') }
  if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('cc_switch_request_url_invalid')

  const method_value = (string(request.method) ?? 'GET').toUpperCase()
  if (method_value !== 'GET' && method_value !== 'POST') throw new Error('cc_switch_request_method_invalid')
  const headers_value = record(request.headers) ?? {}
  const headers = Object.fromEntries(Object.entries(headers_value).map(([key, value]) => [key, String(redact_value(value, key))]))
  const body = request.body ?? request.data ?? request.body_template ?? null
  const extractor = string(root.extractor) ?? string(request.extractor)
  const refresh = root.refresh_interval_seconds ?? root.refreshIntervalSeconds ?? root.interval ?? root.refresh_interval
  const refresh_interval_seconds = typeof refresh === 'number' && Number.isInteger(refresh) && refresh >= 60 && refresh <= 86_400 ? refresh : null
  const variables = record(root.variables) ?? record(root.env) ?? {}
  const secret_variable_names = [...new Set([...variables_in({ url, headers_value, body }), ...Object.keys(variables).filter((key) => sensitive_key.test(key))])].sort()

  return {
    body: redact_value(body),
    extractor_present: Boolean(extractor),
    extractor_target_names: extractor_targets(extractor),
    headers,
    mapping_required: true,
    method: method_value,
    refresh_interval_seconds,
    request_path: `${parsed.pathname}${parsed.search}` || '/',
    secret_variable_names,
    url: parsed.toString(),
  }
}
