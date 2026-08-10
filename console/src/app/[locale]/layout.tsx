import type { Metadata } from 'next'
import { NextIntlClientProvider, hasLocale } from 'next-intl'
import { notFound } from 'next/navigation'

import '../styles.css'
import { Providers } from '../providers'
import { routing } from '@/i18n/routing'

export const metadata: Metadata = {
  title: 'Glance Deck',
  description: 'ESP32 reflective display control plane',
}

export default async function locale_layout({ children, params }: Readonly<{ children: React.ReactNode; params: Promise<{ locale: string }> }>) {
  const { locale } = await params
  if (!hasLocale(routing.locales, locale)) notFound()

  return (
    <html lang={locale}>
      <body>
        <NextIntlClientProvider>
          <Providers>{children}</Providers>
        </NextIntlClientProvider>
      </body>
    </html>
  )
}
