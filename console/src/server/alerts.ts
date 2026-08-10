import { and, eq } from 'drizzle-orm'

import { db } from './db'
import { alert_rules, device_commands } from './schema'

export type Alert_value = string | number | null
export type Alert_operator = 'gt' | 'gte' | 'lt' | 'lte' | 'eq' | 'neq' | 'contains'

function numeric(value: Alert_value) {
  if (typeof value === 'number') return value
  if (typeof value === 'string' && value.trim() !== '') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }
  return null
}

export function matches_alert(value: Alert_value, operator: Alert_operator, threshold: string) {
  if (operator === 'contains') return typeof value === 'string' && value.toLowerCase().includes(threshold.toLowerCase())
  if (operator === 'eq' || operator === 'neq') {
    const left_number = numeric(value)
    const right_number = numeric(threshold)
    const equal = left_number !== null && right_number !== null ? left_number === right_number : String(value ?? '') === threshold
    return operator === 'eq' ? equal : !equal
  }
  const left = numeric(value)
  const right = numeric(threshold)
  if (left === null || right === null) return false
  if (operator === 'gt') return left > right
  if (operator === 'gte') return left >= right
  if (operator === 'lt') return left < right
  return left <= right
}

export async function evaluate_alert_rules(source_id: string, values: Record<string, Alert_value>) {
  if (!db) throw new Error('database_unavailable')
  const rules = await db.select().from(alert_rules).where(and(eq(alert_rules.source_id, source_id), eq(alert_rules.enabled, true)))
  const evaluated_at = new Date()
  let triggered = 0
  for (const rule of rules) {
    const value = values[rule.field] ?? null
    const active = matches_alert(value, rule.operator, rule.threshold)
    const became_active = active && !rule.active
    const became_resolved = !active && rule.active
    await db.update(alert_rules).set({ active, last_value: value, last_evaluated_at: evaluated_at, ...(became_active ? { last_triggered_at: evaluated_at } : {}) }).where(eq(alert_rules.id, rule.id))
    if (became_active) {
      triggered += 1
      if (!rule.test_only) {
        const page_id = rule.page_ids[0]
        if (page_id) {
          await db.insert(device_commands).values(rule.device_ids.map((device_id) => ({ device_id, action: 'show_page', payload: { page_id, alert_rule_id: rule.id, message: rule.message, severity: rule.severity } })))
        }
      }
    }
    if (became_resolved && !rule.test_only) {
      const page_id = rule.page_ids[0]
      if (page_id) {
        await db.insert(device_commands).values(rule.device_ids.map((device_id) => ({ device_id, action: 'refresh_release', payload: { alert_rule_id: rule.id, reason: 'alert_resolved' } })))
      }
    }
  }
  return { evaluated: rules.length, triggered }
}
