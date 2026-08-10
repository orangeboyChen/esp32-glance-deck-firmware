import { redirect } from 'next/navigation'

import { LoginManager } from '@/components/auth-manager'
import { administrator_exists } from '@/server/session'

export default async function login_page({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params
  if (!await administrator_exists()) redirect(`/${locale}/setup`)
  return <LoginManager />
}
