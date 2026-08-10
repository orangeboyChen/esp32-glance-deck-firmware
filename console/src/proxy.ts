import create_middleware from 'next-intl/middleware'

import { routing } from './i18n/routing'

export default create_middleware(routing)

export const config = {
  matcher: '/((?!api|_next|.*\\..*).*)',
}
