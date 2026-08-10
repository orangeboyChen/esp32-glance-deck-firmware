'use client'

import { ThemeProvider, ToastHost } from '@lobehub/ui'
import { Provider as JotaiProvider } from 'jotai'
import type { ReactNode } from 'react'

export function Providers({ children }: { children: ReactNode }) {
  return (
    <JotaiProvider>
      <ThemeProvider themeMode="auto">
        {children}<ToastHost />
      </ThemeProvider>
    </JotaiProvider>
  )
}
