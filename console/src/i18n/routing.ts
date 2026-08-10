import { defineRouting } from 'next-intl/routing'

export const routing = defineRouting({
  locales: ['en', 'zh-CN', 'ja'],
  defaultLocale: 'en',
  localePrefix: 'as-needed',
})
