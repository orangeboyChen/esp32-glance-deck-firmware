import { boolean, customType, integer, jsonb, pgEnum, pgTable, primaryKey, text, timestamp, uniqueIndex, uuid, varchar } from 'drizzle-orm/pg-core'

const bytea = customType<{ data: Buffer; driverData: Buffer }>({
  dataType: () => 'bytea',
})

export const device_status = pgEnum('device_status', ['enrolling', 'online', 'offline', 'error'])
export const command_status = pgEnum('command_status', ['queued', 'sent', 'confirmed', 'failed'])
export const source_status = pgEnum('source_status', ['active', 'paused', 'error'])
export const ota_job_status = pgEnum('ota_job_status', ['awaiting_confirmation', 'queued', 'sent', 'downloading', 'verifying', 'rebooting', 'healthy', 'rolled_back', 'failed', 'cancelled'])
export const alert_operator = pgEnum('alert_operator', ['gt', 'gte', 'lt', 'lte', 'eq', 'neq', 'contains'])

export const administrators = pgTable('administrators', {
  id: uuid('id').defaultRandom().primaryKey(),
  email: varchar('email', { length: 320 }).notNull(),
  password_hash: text('password_hash').notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('administrators_email_unique').on(table.email)])

export const sessions = pgTable('sessions', {
  id: uuid('id').defaultRandom().primaryKey(),
  administrator_id: uuid('administrator_id').references(() => administrators.id, { onDelete: 'cascade' }).notNull(),
  token_selector: varchar('token_selector', { length: 32 }),
  token_hash: text('token_hash').notNull(),
  expires_at: timestamp('expires_at', { withTimezone: true }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('sessions_token_selector_unique').on(table.token_selector)])

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
  device_image: bytea('device_image').notNull(),
  image_format: varchar('image_format', { length: 32 }).default('mono1-msb').notNull(),
  image_width: integer('image_width').default(400).notNull(),
  image_height: integer('image_height').default(300).notNull(),
  content_sha256: varchar('content_sha256', { length: 64 }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const display_release_pages = pgTable('display_release_pages', {
  release_id: uuid('release_id').references(() => display_releases.id, { onDelete: 'cascade' }).notNull(),
  page_id: varchar('page_id', { length: 64 }).notNull(),
  position: integer('position').notNull(),
  document: jsonb('document').notNull(),
  preview_svg: text('preview_svg').notNull(),
  device_image: bytea('device_image').notNull(),
  image_format: varchar('image_format', { length: 32 }).default('mono1-msb').notNull(),
  image_width: integer('image_width').default(400).notNull(),
  image_height: integer('image_height').default(300).notNull(),
  content_sha256: varchar('content_sha256', { length: 64 }).notNull(),
}, (table) => [
  primaryKey({ columns: [table.release_id, table.page_id] }),
  uniqueIndex('display_release_pages_position_unique').on(table.release_id, table.position),
])

export const usage_sources = pgTable('usage_sources', {
  id: uuid('id').defaultRandom().primaryKey(),
  name: varchar('name', { length: 128 }).notNull(),
  base_url: text('base_url').notNull(),
  request_path: text('request_path').notNull(),
  method: varchar('method', { length: 8 }).default('GET').notNull(),
  headers: jsonb('headers').$type<Record<string, string>>().default({}).notNull(),
  body_template: text('body_template'),
  secret_ciphertext: text('secret_ciphertext').notNull(),
  mapper: jsonb('mapper').$type<Record<string, string>>().notNull(),
  refresh_interval_seconds: integer('refresh_interval_seconds').default(900).notNull(),
  status: source_status('source_status').default('active').notNull(),
  last_success_at: timestamp('last_success_at', { withTimezone: true }),
  last_error: text('last_error'),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const source_snapshots = pgTable('source_snapshots', {
  id: uuid('id').defaultRandom().primaryKey(),
  source_id: uuid('source_id').references(() => usage_sources.id, { onDelete: 'cascade' }).notNull(),
  values: jsonb('values').$type<Record<string, string | number | null>>().notNull(),
  response_preview: text('response_preview'),
  fetched_at: timestamp('fetched_at', { withTimezone: true }).defaultNow().notNull(),
})

export const alert_rules = pgTable('alert_rules', {
  id: uuid('id').defaultRandom().primaryKey(),
  name: varchar('name', { length: 128 }).notNull(),
  source_id: uuid('source_id').references(() => usage_sources.id, { onDelete: 'cascade' }).notNull(),
  field: varchar('field', { length: 32 }).notNull(),
  operator: alert_operator('operator').notNull(),
  threshold: text('threshold').notNull(),
  severity: varchar('severity', { length: 16 }).default('warning').notNull(),
  message: varchar('message', { length: 256 }).notNull(),
  device_ids: jsonb('device_ids').$type<string[]>().notNull(),
  page_ids: jsonb('page_ids').$type<string[]>().notNull(),
  enabled: boolean('enabled').default(true).notNull(),
  test_only: boolean('test_only').default(false).notNull(),
  active: boolean('active').default(false).notNull(),
  last_value: jsonb('last_value'),
  last_evaluated_at: timestamp('last_evaluated_at', { withTimezone: true }),
  last_triggered_at: timestamp('last_triggered_at', { withTimezone: true }),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const display_bindings = pgTable('display_bindings', {
  id: uuid('id').defaultRandom().primaryKey(),
  source_id: uuid('source_id').references(() => usage_sources.id, { onDelete: 'cascade' }).notNull(),
  page_id: varchar('page_id', { length: 64 }).notNull(),
  document_template: jsonb('document_template').$type<{
    title: string
    subtitle?: string
    lines?: Array<{ label: string; value: string }>
  }>().notNull(),
  device_ids: jsonb('device_ids').$type<string[]>().notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const devices = pgTable('devices', {
  id: varchar('id', { length: 64 }).primaryKey(),
  name: varchar('name', { length: 128 }).notNull(),
  board_model: varchar('board_model', { length: 64 }).default('ESP32-S3-RLCD-4.2').notNull(),
  status: device_status('status').default('enrolling').notNull(),
  firmware_version: varchar('firmware_version', { length: 64 }),
  last_good_firmware_release_id: uuid('last_good_firmware_release_id').references(() => firmware_releases.id),
  wifi_rssi: integer('wifi_rssi'),
  active_page_id: varchar('active_page_id', { length: 64 }).default('system').notNull(),
  desired_page_id: varchar('desired_page_id', { length: 64 }),
  enabled_page_ids: jsonb('enabled_page_ids').$type<string[]>(),
  power_source: varchar('power_source', { length: 16 }),
  charging: boolean('charging'),
  battery_percent: integer('battery_percent'),
  battery_mv: integer('battery_mv'),
  power_updated_at: timestamp('power_updated_at', { withTimezone: true }),
  release_id: uuid('release_id').references(() => display_releases.id),
  last_seen_at: timestamp('last_seen_at', { withTimezone: true }),
  enrollment_code_hash: varchar('enrollment_code_hash', { length: 64 }),
  enrollment_expires_at: timestamp('enrollment_expires_at', { withTimezone: true }),
  mqtt_username: varchar('mqtt_username', { length: 128 }),
  mqtt_password_ciphertext: text('mqtt_password_ciphertext'),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
})

export const device_enrollment_requests = pgTable('device_enrollment_requests', {
  id: uuid('id').defaultRandom().primaryKey(),
  pairing_code_hash: varchar('pairing_code_hash', { length: 64 }).notNull(),
  claim_secret_hash: varchar('claim_secret_hash', { length: 64 }).notNull(),
  board_model: varchar('board_model', { length: 64 }).notNull(),
  claimed_device_id: varchar('claimed_device_id', { length: 64 }).references(() => devices.id, { onDelete: 'set null' }),
  expires_at: timestamp('expires_at', { withTimezone: true }).notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('device_enrollment_requests_pairing_code_unique').on(table.pairing_code_hash)])

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

export const firmware_releases = pgTable('firmware_releases', {
  id: uuid('id').defaultRandom().primaryKey(),
  version: varchar('version', { length: 64 }).notNull(),
  board_model: varchar('board_model', { length: 64 }).notNull(),
  channel: varchar('channel', { length: 16 }).default('stable').notNull(),
  manifest_url: text('manifest_url').notNull(),
  image_url: text('image_url').notNull(),
  image_sha256: varchar('image_sha256', { length: 64 }).notNull(),
  manifest_signature: text('manifest_signature').notNull(),
  verified_at: timestamp('verified_at', { withTimezone: true }).defaultNow().notNull(),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
}, (table) => [uniqueIndex('firmware_releases_version_board_unique').on(table.version, table.board_model)])

export const ota_jobs = pgTable('ota_jobs', {
  id: uuid('id').defaultRandom().primaryKey(),
  device_id: varchar('device_id', { length: 64 }).references(() => devices.id, { onDelete: 'cascade' }).notNull(),
  firmware_release_id: uuid('firmware_release_id').references(() => firmware_releases.id).notNull(),
  status: ota_job_status('ota_job_status').default('queued').notNull(),
  nonce: varchar('nonce', { length: 128 }).notNull(),
  error_message: text('error_message'),
  created_at: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
  completed_at: timestamp('completed_at', { withTimezone: true }),
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
