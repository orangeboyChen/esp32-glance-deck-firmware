import { DeviceDashboard } from '@/components/device-dashboard'
import { dashboard_summary, list_devices } from '@/server/devices'
import { require_page_administrator } from '@/server/session'

export default async function overview_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  await require_page_administrator(locale)
  const [devices, summary] = await Promise.all([list_devices(), dashboard_summary()])
  return <DeviceDashboard devices={devices} summary={summary} />
}
