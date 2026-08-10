import { DisplayManager } from '@/components/display-manager'
import { require_page_administrator } from '@/server/session'

export default async function displays_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  await require_page_administrator(locale)
  return <DisplayManager />
}
