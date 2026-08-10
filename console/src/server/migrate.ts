import { readdir, readFile } from 'node:fs/promises'

import postgres from 'postgres'

const database_url = process.env.DATABASE_URL
if (!database_url) throw new Error('DATABASE_URL is required')

const sql = postgres(database_url, { max: 1 })
const migrations_directory = new URL('../../drizzle/', import.meta.url)
const migration_files = (await readdir(migrations_directory)).filter((file) => file.endsWith('.sql')).sort()

await sql.unsafe('CREATE TABLE IF NOT EXISTS "schema_migrations" ("name" text PRIMARY KEY, "applied_at" timestamp with time zone NOT NULL DEFAULT now())')
for (const migration_file of migration_files) {
  const [applied] = await sql<{ name: string }[]>`SELECT name FROM schema_migrations WHERE name = ${migration_file}`
  if (!applied) {
    const migration = await readFile(new URL(migration_file, migrations_directory), 'utf8')
    await sql.begin(async (transaction) => {
      await transaction.unsafe(migration)
      await transaction`INSERT INTO schema_migrations (name) VALUES (${migration_file})`
    })
  }
}
await sql.end()
console.log(`database migration complete (${migration_files.length} files)`)
