import { describe, expect, test } from 'bun:test'

import { validate_mqtt_url } from './mqtt'

describe('MQTT transport validation', () => {
  test('requires TLS outside an explicit trusted-internal exception', () => {
    expect(validate_mqtt_url('mqtts://broker.example').protocol).toBe('mqtts:')
    expect(() => validate_mqtt_url('mqtt://broker.example')).toThrow('mqtt_tls_required')
    expect(validate_mqtt_url('mqtt://broker.example', true).protocol).toBe('mqtt:')
  })
})
