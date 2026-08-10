import { DeviceDashboard } from '@/components/device-dashboard'
import { list_devices } from '@/server/devices'

export default async function overview_page() {
  const devices = await list_devices()
  return <DeviceDashboard devices={devices} />
}
