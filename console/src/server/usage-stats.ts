type UsageValue = string | number | null

export type UsageSnapshot = {
  fetched_at: Date
  values: Record<string, UsageValue>
}

export type DerivedUsage = {
  today_used: number | null
  today_percent: number | null
  week_used: number | null
  week_percent: number | null
}

function start_of_day(now: Date) {
  const start = new Date(now)
  start.setHours(0, 0, 0, 0)
  return start
}

function start_of_week(now: Date) {
  const start = start_of_day(now)
  const day = start.getDay()
  start.setDate(start.getDate() - (day === 0 ? 6 : day - 1))
  return start
}

function numeric(value: UsageValue) {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function delta_since(snapshots: UsageSnapshot[], boundary: Date, current_used: number) {
  const baseline = snapshots.find((snapshot) => snapshot.fetched_at >= boundary)
  const baseline_used = baseline ? numeric(baseline.values.used) : null
  if (baseline_used === null || current_used < baseline_used) return null
  return current_used - baseline_used
}

export function derive_usage_metrics(current: Record<string, UsageValue>, snapshots: UsageSnapshot[], now = new Date()): DerivedUsage {
  const current_used = numeric(current.used)
  const total = numeric(current.total)
  if (current_used === null) {
    return { today_used: null, today_percent: null, week_used: null, week_percent: null }
  }

  const today_used = delta_since(snapshots, start_of_day(now), current_used)
  const week_used = delta_since(snapshots, start_of_week(now), current_used)
  const percent = (value: number | null) => value === null || total === null || total <= 0 ? null : Math.min(100, Math.max(0, (value / total) * 100))
  return { today_used, today_percent: percent(today_used), week_used, week_percent: percent(week_used) }
}
