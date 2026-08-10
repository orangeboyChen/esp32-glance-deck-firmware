import { describe, expect, test } from 'bun:test'

import { preview_cc_switch_import } from './cc-switch-import'

describe('CC Switch import preview', () => {
  test('redacts request credentials without executing the extractor', () => {
    const preview = preview_cc_switch_import({
      extractor: 'return { used: payload.used, remaining: payload.left }',
      interval: 900,
      request: {
        body: { api_key: 'body-secret', filter: 'today' },
        headers: { Authorization: 'Bearer header-secret', 'X-Trace': 'trace' },
        method: 'POST',
        url: 'https://usage.example.test/v1/quota?account={{ACCOUNT_ID}}',
      },
      variables: { API_KEY: 'variable-secret' },
    })

    expect(preview).toMatchObject({
      body: { api_key: '[REDACTED]', filter: 'today' },
      extractor_present: true,
      extractor_target_names: ['remaining', 'used'],
      headers: { Authorization: '[REDACTED]', 'X-Trace': 'trace' },
      mapping_required: true,
      method: 'POST',
      refresh_interval_seconds: 900,
      request_path: '/v1/quota?account={{ACCOUNT_ID}}',
      secret_variable_names: ['ACCOUNT_ID', 'API_KEY'],
    })
    expect(JSON.stringify(preview)).not.toContain('header-secret')
    expect(JSON.stringify(preview)).not.toContain('body-secret')
    expect(JSON.stringify(preview)).not.toContain('variable-secret')
  })

  test('rejects malformed or unsupported requests', () => {
    expect(() => preview_cc_switch_import({ request: { url: 'file:///etc/passwd' } })).toThrow('cc_switch_request_url_invalid')
    expect(() => preview_cc_switch_import({ request: { url: 'https://example.test', method: 'DELETE' } })).toThrow('cc_switch_request_method_invalid')
  })
})
