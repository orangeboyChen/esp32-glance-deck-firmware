import { and, asc, eq } from 'drizzle-orm'

import { db } from './db'
import { publish_device_command } from './mqtt'
import { device_commands } from './schema'

export async function dispatch_queued_commands() {
  if (!db) return 0
  const queued = await db.select().from(device_commands)
    .where(eq(device_commands.status, 'queued'))
    .orderBy(asc(device_commands.created_at))
    .limit(20)

  for (const command of queued) {
    try {
      await publish_device_command(command.device_id, command)
      await db.update(device_commands)
        .set({ status: 'sent' })
        .where(and(eq(device_commands.id, command.id), eq(device_commands.status, 'queued')))
    } catch (error) {
      await db.update(device_commands)
        .set({ status: 'failed', error_message: error instanceof Error ? error.message : 'mqtt_publish_failed' })
        .where(eq(device_commands.id, command.id))
    }
  }
  return queued.length
}
