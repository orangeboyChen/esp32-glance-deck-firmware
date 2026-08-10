import { and, asc, eq } from 'drizzle-orm'

import { db } from './db'
import { publish_device_command } from './mqtt'
import { device_commands } from './schema'

export async function dispatch_queued_commands() {
  if (!db) return 0
  let dispatched = 0
  for (let index = 0; index < 20; index += 1) {
    const processed = await db.transaction(async (transaction) => {
      const [command] = await transaction.select().from(device_commands)
        .where(eq(device_commands.status, 'queued'))
        .orderBy(asc(device_commands.created_at))
        .limit(1)
        .for('update', { skipLocked: true })
      if (!command) return false

      try {
        await publish_device_command(command.device_id, command)
        await transaction.update(device_commands).set({ status: 'sent' })
          .where(and(eq(device_commands.id, command.id), eq(device_commands.status, 'queued')))
      } catch (error) {
        await transaction.update(device_commands).set({
          status: 'failed',
          error_message: error instanceof Error ? error.message : 'mqtt_publish_failed',
        }).where(eq(device_commands.id, command.id))
      }
      return true
    })
    if (!processed) break
    dispatched += 1
  }
  return dispatched
}
