import { SourcesManager } from '@/components/sources-manager'
import { require_page_administrator } from '@/server/session'

export default async function sources_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  await require_page_administrator(locale)
  return <SourcesManager />
}
