import { DeviceDashboard } from '@/components/device-dashboard'
import { dashboard_summary, list_devices } from '@/server/devices'

export default async function overview_page() {
  const [devices, summary] = await Promise.all([list_devices(), dashboard_summary()])
  return <DeviceDashboard devices={devices} summary={summary} />
}
