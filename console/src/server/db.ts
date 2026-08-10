import { drizzle } from 'drizzle-orm/postgres-js'
import postgres from 'postgres'

const database_url = process.env.DATABASE_URL

export const database_available = Boolean(database_url)

const client = database_url ? postgres(database_url, { max: 4 }) : undefined

export const db = client ? drizzle(client) : undefined
