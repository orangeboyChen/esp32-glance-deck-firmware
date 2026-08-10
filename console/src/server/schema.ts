import { boolean, integer, jsonb, pgEnum, pgTable, text, timestamp, uniqueIndex, uuid, varchar } from 'drizzle-orm/pg-core'

export const device_status = pgEnum('device_status', ['enrolling', 'online', 'offline', 'error'])
export const command_status = pgEnum('command_status', ['queued', 'sent', 'confirmed', 'failed'])

export const administrators = pgTable('administrators', {
  id: uuid('id').defaultRandom().primaryKey(),
  email: varchar('email', { length: 320 }).notNull(),
  password_hash: text('password_hash').notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('administrators_email_unique').on(table.email)])

export const sessions = pgTable('sessions', {
  id: uuid('id').defaultRandom().primaryKey(),
  administrator_id: uuid('administrator_id').references(() => administrators.id, { onDelete: 'cascade' }).notNull(),
  token_hash: text('token_hash').notNull(),
  expires_at: timestamp('expires_at', { withTimezone: true }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const passkeys = pgTable('passkeys', {
  id: uuid('id').defaultRandom().primaryKey(),
  administrator_id: uuid('administrator_id').references(() => administrators.id, { onDelete: 'cascade' }).notNull(),
  credential_id: text('credential_id').notNull(),
  public_key: text('public_key').notNull(),
  counter: integer('counter').default(0).notNull(),
  transports: jsonb('transports').$type<string[]>(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('passkeys_credential_id_unique').on(table.credential_id)])

export const webauthn_challenges = pgTable('webauthn_challenges', {
  id: uuid('id').defaultRandom().primaryKey(),
  administrator_id: uuid('administrator_id').references(() => administrators.id, { onDelete: 'cascade' }),
  challenge: text('challenge').notNull(),
  purpose: varchar('purpose', { length: 32 }).notNull(),
  expires_at: timestamp('expires_at', { withTimezone: true }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const display_releases = pgTable('display_releases', {
  id: uuid('id').defaultRandom().primaryKey(),
  version: integer('version').notNull(),
  page_id: varchar('page_id', { length: 64 }).notNull(),
  document: jsonb('document').notNull(),
  preview_svg: text('preview_svg').notNull(),
  content_sha256: varchar('content_sha256', { length: 64 }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const devices = pgTable('devices', {
  id: varchar('id', { length: 64 }).primaryKey(),
  name: varchar('name', { length: 128 }).notNull(),
  board_model: varchar('board_model', { length: 64 }).default('ESP32-S3-RLCD-4.2').notNull(),
  status: device_status('status').default('enrolling').notNull(),
  firmware_version: varchar('firmware_version', { length: 64 }),
  wifi_rssi: integer('wifi_rssi'),
  active_page_id: varchar('active_page_id', { length: 64 }).default('system').notNull(),
  release_id: uuid('release_id').references(() => display_releases.id),
  last_seen_at: timestamp('last_seen_at', { withTimezone: true }),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const device_commands = pgTable('device_commands', {
  id: uuid('id').defaultRandom().primaryKey(),
  device_id: varchar('device_id', { length: 64 }).references(() => devices.id, { onDelete: 'cascade' }).notNull(),
  action: varchar('action', { length: 64 }).notNull(),
  payload: jsonb('payload').notNull(),
  status: command_status('status').default('queued').notNull(),
  error_message: text('error_message'),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
  confirmed_at: timestamp('confirmed_at', { withTimezone: true }),
})

export const api_tokens = pgTable('api_tokens', {
  id: uuid('id').defaultRandom().primaryKey(),
  label: varchar('label', { length: 128 }).notNull(),
  token_hash: text('token_hash').notNull(),
  scopes: jsonb('scopes').$type<string[]>().notNull(),
  revoked_at: timestamp('revoked_at', { withTimezone: true }),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const audit_events = pgTable('audit_events', {
  id: uuid('id').defaultRandom().primaryKey(),
  actor: varchar('actor', { length: 128 }).notNull(),
  action: varchar('action', { length: 128 }).notNull(),
  target: varchar('target', { length: 256 }).notNull(),
  metadata: jsonb('metadata').notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})
