import { connect, type MqttClient } from 'mqtt'

let mqtt_client: MqttClient | undefined

function get_client() {
  if (mqtt_client) return mqtt_client
  const url = process.env.MQTT_URL
  if (!url) throw new Error('mqtt_url_missing')
  mqtt_client = connect(url, { reconnectPeriod: 5_000 })
  return mqtt_client
}

export async function publish_device_command(device_id: string, command: { id: string; action: string; payload: unknown }) {
  const client = get_client()
  const topic = `glance_deck/${device_id}/command`
  const message = JSON.stringify({ command_id: command.id, action: command.action, payload: command.payload })

  await new Promise<void>((resolve, reject) => {
    client.publish(topic, message, { qos: 1 }, (error) => error ? reject(error) : resolve())
  })
}
