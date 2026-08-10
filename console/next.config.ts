import type { NextConfig } from 'next'
import create_next_intl_plugin from 'next-intl/plugin'

const next_config: NextConfig = {
  output: 'standalone',
  serverExternalPackages: ['@resvg/resvg-js'],
}

export default create_next_intl_plugin('./src/i18n/request.ts')(next_config)
