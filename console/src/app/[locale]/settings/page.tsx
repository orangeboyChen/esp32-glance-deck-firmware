import { SettingsManager } from '@/components/settings-manager'
import { require_page_administrator } from '@/server/session'

export default async function settings_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  await require_page_administrator(locale)
  return <SettingsManager />
}
