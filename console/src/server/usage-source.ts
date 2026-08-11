import { isIP } from 'node:net'
import { lookup } from 'node:dns/promises'
import { request as https_request } from 'node:https'

import { and, desc, eq, gte } from 'drizzle-orm'

import { db } from './db'
import { decrypt_secret } from './secrets'
import { publish_source_changes } from './source-publisher'
import { evaluate_alert_rules } from './alerts'
import { source_snapshots, usage_sources } from './schema'
import { derive_usage_metrics } from './usage-stats'

const MAX_RESPONSE_BYTES = 256 * 1024
const fields = ['plan_name', 'used', 'remaining', 'total', 'unit', 'resets_at', 'status'] as const
type MappedValue = string | number | null

function json_path(value: unknown, selector: string): MappedValue {
  const parts = /^\$((?:\.[A-Za-z_][A-Za-z0-9_]*)|(?:\[\d+\]))*$/.exec(selector)
  if (!parts) throw new Error('mapper_jsonpath_invalid')
  let current: unknown = value
  for (const match of selector.matchAll(/\.([A-Za-z_][A-Za-z0-9_]*)|\[(\d+)\]/g)) {
    if (match[1]) current = current && typeof current === 'object' && !Array.isArray(current) ? (current as Record<string, unknown>)[match[1]] : undefined
    else current = Array.isArray(current) ? current[Number(match[2])] : undefined
  }
  if (current === null || typeof current === 'string' || typeof current === 'number') return current
  return null
}

function interpolate(template: string, secrets: Record<string, string>) {
  return template.replace(/\{\{([A-Za-z][A-Za-z0-9_]*)\}\}/g, (_match, key: string) => {
    if (!(key in secrets)) throw new Error(`secret_template_missing:${key}`)
    return secrets[key]
  })
}

function redact_response(value: string, secrets: Record<string, string>) {
  return Object.values(secrets).filter(Boolean).reduce((result, secret) => result.replaceAll(secret, '[REDACTED]'), value)
}

function is_private_address(address: string) {
  if (isIP(address) === 4) {
    const [first, second] = address.split('.').map(Number)
    return first === 10 || first === 127 || first === 0 || first === 169 && second === 254 || first === 172 && second >= 16 && second <= 31 || first === 192 && second === 168
  }
  return address === '::1' || address.startsWith('fc') || address.startsWith('fd') || address.startsWith('fe80:')
}

type SafeSourceUrl = {
  url: URL
  address?: string
  family?: 4 | 6
}

async function safe_url(base_url: string, request_path: string): Promise<SafeSourceUrl> {
  const url = new URL(request_path, base_url)
  const local_dev = process.env.NODE_ENV !== 'production' && (url.hostname === 'localhost' || url.hostname === '127.0.0.1')
  if (url.protocol !== 'https:' && !local_dev) throw new Error('source_https_required')
  const allowed_hosts = (process.env.SOURCE_ALLOWED_HOSTS ?? '').split(',').map((host) => host.trim().toLowerCase()).filter(Boolean)
  if (!local_dev && allowed_hosts.length > 0 && !allowed_hosts.includes(url.hostname.toLowerCase())) {
    throw new Error('source_host_not_allowlisted')
  }
  if (!local_dev) {
    const resolved = await lookup(url.hostname, { all: true })
    if (resolved.length === 0 || resolved.some(({ address }) => is_private_address(address))) throw new Error('source_private_address_blocked')
    const target = resolved[0]
    if (!target) throw new Error('source_address_unavailable')
    const family = isIP(target.address)
    if (family !== 4 && family !== 6) throw new Error('source_address_invalid')
    return { url, address: target.address, family }
  }
  return { url }
}

async function fetch_source(source_url: SafeSourceUrl, method: string, headers: Record<string, string>, body: string | undefined) {
  if (!source_url.address || !source_url.family) {
    const response = await fetch(source_url.url, { method, headers, body, redirect: 'error', signal: AbortSignal.timeout(10_000) })
    return { status: response.status, content_type: response.headers.get('content-type') ?? '', raw: await response.text() }
  }

  return new Promise<{ status: number; content_type: string; raw: string }>((resolve, reject) => {
    const request = https_request({
      protocol: source_url.url.protocol,
      hostname: source_url.url.hostname,
      port: source_url.url.port || undefined,
      path: `${source_url.url.pathname}${source_url.url.search}`,
      method,
      headers,
      servername: source_url.url.hostname,
      lookup: (_hostname, _options, callback) => callback(null, source_url.address!, source_url.family!),
    }, (response) => {
      const chunks: Buffer[] = []
      let size = 0
      response.on('data', (chunk: Buffer) => {
        size += chunk.length
        if (size > MAX_RESPONSE_BYTES) request.destroy(new Error('source_response_too_large'))
        else chunks.push(chunk)
      })
      response.on('end', () => resolve({
        status: response.statusCode ?? 0,
        content_type: String(response.headers['content-type'] ?? ''),
        raw: Buffer.concat(chunks).toString('utf8'),
      }))
    })
    request.setTimeout(10_000, () => request.destroy(new Error('source_request_timeout')))
    request.once('error', reject)
    request.end(body)
  })
}

export async function refresh_usage_source(source_id: string) {
  if (!db) throw new Error('database_unavailable')
  const [source] = await db.select().from(usage_sources).where(eq(usage_sources.id, source_id)).limit(1)
  if (!source) throw new Error('source_not_found')
  try {
    const secrets = decrypt_secret(source.secret_ciphertext)
    const source_url = await safe_url(source.base_url, source.request_path)
    const headers = Object.fromEntries(Object.entries(source.headers).map(([key, value]) => [key, interpolate(value, secrets)]))
    const body = source.body_template ? interpolate(source.body_template, secrets) : undefined
    const response = await fetch_source(source_url, source.method, headers, body)
    if (response.status < 200 || response.status >= 300) throw new Error(`source_http_${response.status}`)
    if (!response.content_type.includes('json')) throw new Error('source_content_type_invalid')
    const raw = response.raw
    const parsed: unknown = JSON.parse(raw)
    const values = Object.fromEntries(fields.map((field) => [field, source.mapper[field] ? json_path(parsed, source.mapper[field]) : null])) as Record<string, MappedValue>
    const previous = await db.select({ values: source_snapshots.values }).from(source_snapshots)
      .where(eq(source_snapshots.source_id, source_id)).orderBy(desc(source_snapshots.fetched_at)).limit(1)
    const history_start = new Date()
    history_start.setDate(history_start.getDate() - 7)
    const history = await db.select({ values: source_snapshots.values, fetched_at: source_snapshots.fetched_at })
      .from(source_snapshots)
      .where(and(eq(source_snapshots.source_id, source_id), gte(source_snapshots.fetched_at, history_start)))
      .orderBy(source_snapshots.fetched_at)
    const persisted_values = { ...values, ...derive_usage_metrics(values, history) }
    const changed = JSON.stringify(previous[0]?.values ?? null) !== JSON.stringify(persisted_values)
    await db.transaction(async (transaction) => {
      await transaction.insert(source_snapshots).values({ source_id, values: persisted_values, response_preview: redact_response(raw, secrets).slice(0, 2048) })
      await transaction.update(usage_sources).set({ status: 'active', last_success_at: new Date(), last_error: null }).where(eq(usage_sources.id, source_id))
    })
    if (changed) await publish_source_changes(source_id, persisted_values)
    await evaluate_alert_rules(source_id, persisted_values)
    return persisted_values
  } catch (error) {
    const message = error instanceof Error ? error.message : 'source_refresh_failed'
    await db.update(usage_sources).set({ status: 'error', last_error: message }).where(eq(usage_sources.id, source_id))
    throw error
  }
}
