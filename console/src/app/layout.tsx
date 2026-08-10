import type { Metadata } from 'next'
import './styles.css'
import { Providers } from './providers'

export const metadata: Metadata = {
  title: 'Glance Deck',
  description: 'ESP32 reflective display control plane',
}

export default function root_layout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body><Providers>{children}</Providers></body>
    </html>
  )
}
