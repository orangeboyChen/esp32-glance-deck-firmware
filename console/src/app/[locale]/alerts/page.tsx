import { AlertsManager } from '@/components/alerts-manager'
import { require_page_administrator } from '@/server/session'

export default async function alerts_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  await require_page_administrator(locale)
  return <AlertsManager />
}
