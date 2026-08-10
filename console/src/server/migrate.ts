import { readFile } from 'node:fs/promises'

import postgres from 'postgres'

const database_url = process.env.DATABASE_URL
if (!database_url) throw new Error('DATABASE_URL is required')

const sql = postgres(database_url, { max: 1 })
const migration = await readFile(new URL('../../drizzle/0000_initial.sql', import.meta.url), 'utf8')

await sql.unsafe(migration)
await sql.end()
console.log('database migration complete')
