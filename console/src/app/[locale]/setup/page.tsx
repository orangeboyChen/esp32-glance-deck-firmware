import { redirect } from 'next/navigation'

import { SetupManager } from '@/components/auth-manager'
import { administrator_exists } from '@/server/session'

export default async function setup_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  if (await administrator_exists()) redirect(`/${locale}/login`)
  return <SetupManager />
}
