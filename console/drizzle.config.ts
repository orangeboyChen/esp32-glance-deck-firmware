import type { Config } from 'drizzle-kit'

export default {
  schema: './src/server/schema.ts',
  out: './drizzle',
  dialect: 'postgresql',
  dbCredentials: {
    url: process.env.DATABASE_URL ?? 'postgresql://glance_deck:glance_deck@localhost:5432/glance_deck',
  },
} satisfies Config
