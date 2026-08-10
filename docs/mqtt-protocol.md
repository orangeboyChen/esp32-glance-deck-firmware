# MQTT protocol

Each device has an immutable `device_id`, for example `glance-deck-office`.
All topics use `glance_deck/<device_id>/`.

| Topic suffix | Direction | Retained | Purpose |
| --- | --- | --- | --- |
| `display` | HA to device | Yes | Current display document |
| `command` | HA to device | No | Immediate action |
| `state` | Device to control plane | Yes | Connectivity and UI state |
| `availability` | Device to control plane | Yes | `online` or `offline` |
| `ota` | Control plane to device | No | Signed remote OTA job |
| `ota/state` | Device to control plane | Yes | OTA progress and result |

## Display document

The device must reject unknown document versions and payloads larger than its
configured limit. A later revision can add widgets without changing commands.

```json
{
  "version": 1,
  "title": "AI subscriptions",
  "updated_at": "2026-08-09T18:00:00+08:00",
  "pages": [
    {
      "id": "usage",
      "title": "Usage",
      "widgets": [
        {"type": "gauge", "label": "Today", "value": 72, "unit": "%"},
        {"type": "text", "label": "Weekly", "value": "41% used"}
      ]
    }
  ]
}
```

## Commands

```json
{"action":"show_page","page_id":"usage"}
```

Supported initial actions: `show_page`, `next_page`, `previous_page`,
`set_rotation`, and `refresh`.

## Device state

```json
{"version":1,"page_id":"usage","wifi_rssi":-56,"display_updated_at":"2026-08-09T18:00:00+08:00"}
```
