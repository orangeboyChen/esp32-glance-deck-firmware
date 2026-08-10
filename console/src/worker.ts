import { dispatch_queued_commands } from './server/commands'
import { start_device_state_consumer } from './server/mqtt'

const worker_name = 'glance-deck-worker'

async function tick() {
  try {
    const count = await dispatch_queued_commands()
    if (count > 0) console.log(`${worker_name}: dispatched ${count} device command(s)`)
  } catch (error) {
    console.error(`${worker_name}: command dispatch failed`, error)
  }
}

console.log(`${worker_name}: ready to process command, source, and OTA jobs`)
start_device_state_consumer()
await tick()
setInterval(tick, 1_000)
