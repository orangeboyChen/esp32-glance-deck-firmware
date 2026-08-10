import { afterEach, describe, expect, mock, test } from 'bun:test'

const published = mock(async () => undefined)
const select_results: unknown[][] = []
const select = mock(() => ({
  from: () => ({
    where: () => ({
      limit: async () => select_results.shift() ?? [],
      then: (resolve: (value: unknown[]) => unknown) => Promise.resolve(select_results.shift() ?? []).then(resolve),
    }),
    then: (resolve: (value: unknown[]) => unknown) => Promise.resolve(select_results.shift() ?? []).then(resolve),
  }),
}))
const transaction = mock(async (callback: (transaction: typeof database) => Promise<unknown>) => callback(database))
const database = {
  select,
  transaction,
  insert: () => ({ values: () => ({ returning: async () => [{ id: 'release-1', version: 4, page_id: 'usage', image_format: 'mono1-msb', image_width: 400, image_height: 300, content_sha256: 'a'.repeat(64) }] }) }),
  update: () => ({ set: () => ({ where: async () => undefined }) }),
}

mock.module('./db', () => ({ db: database }))
mock.module('./mqtt', () => ({ publish_device_release: published }))

const { publish_source_changes } = await import('./source-publisher')

describe('source-bound display publication', () => {
  afterEach(() => {
    select_results.length = 0
    select.mockClear()
    transaction.mockClear()
    published.mockClear()
    delete process.env.DEVICE_ASSET_URL
  })

  test('creates a new bitmap release and publishes it to every assigned device', async () => {
    process.env.DEVICE_ASSET_URL = 'https://console.example'
    select_results.push(
      [{ source_id: 'source-1', page_id: 'usage', device_ids: ['desk-a', 'desk-b'], document_template: { title: '{{plan_name}}', lines: [{ label: 'Today', value: '{{used}}%' }] } }],
      [{ id: 'desk-a', release_id: 'old-a' }, { id: 'desk-b', release_id: 'old-b' }],
      [],
      [{ next_version: 4 }],
    )
    expect(await publish_source_changes('source-1', { plan_name: 'Pro', used: 72 })).toBe(2)
    expect(transaction).toHaveBeenCalledTimes(1)
    expect(published).toHaveBeenCalledTimes(2)
    expect(published.mock.calls[0]?.[0]).toBe('desk-a')
    expect(published.mock.calls[0]?.[1]).toMatchObject({ id: 'release-1', version: 4, active_page_id: 'usage' })
  })

  test('does not republish an unchanged release already assigned to all devices', async () => {
    select_results.push(
      [{ source_id: 'source-1', page_id: 'usage', device_ids: ['desk-a'], document_template: { title: 'Fixed' } }],
      [{ id: 'desk-a', release_id: 'release-existing' }],
      [{ id: 'release-existing' }],
    )
    expect(await publish_source_changes('source-1', {})).toBe(0)
    expect(transaction).not.toHaveBeenCalled()
    expect(published).not.toHaveBeenCalled()
  })

  test('rejects a binding that references missing devices', async () => {
    select_results.push(
      [{ source_id: 'source-1', page_id: 'usage', device_ids: ['desk-a', 'desk-missing'], document_template: { title: 'Fixed' } }],
      [{ id: 'desk-a', release_id: null }],
    )
    await expect(publish_source_changes('source-1', {})).rejects.toThrow('display_binding_device_not_found')
  })
})
