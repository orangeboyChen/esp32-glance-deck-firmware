# MQTT protocol

Each device has an immutable `device_id`, for example `glance-deck-office`.
All topics use `glance_deck/<device_id>/`.

| Topic suffix | Direction | Retained | Purpose |
| --- | --- | --- | --- |
| `release` | Control plane to device | Yes | Current immutable display bitmap |
| `command` | HA to device | No | Immediate action |
| `state` | Device to control plane | Yes | Connectivity and UI state |
| `availability` | Device to control plane | Yes | `online` or `offline` |
| `ota` | Control plane to device | No | Signed remote OTA job |
| `ota/state` | Device to control plane | Yes | OTA progress and result |

## Display release

The control plane is the only renderer. It rasterizes all text, including CJK,
with its bundled font and sends an immutable 1-bit MSB-first image. The ESP32
must not parse SVG or depend on a font catalog. It must reject unknown document
versions, image formats, dimensions, hashes, or byte counts.

```json
{
  "release_id": "b39d5ac2-8ff6-4b7c-95fd-31d243e58e11",
  "document_version": 1,
  "active_page_id": "usage",
  "pages": [{
    "page_id": "usage",
    "image_format": "mono1-msb",
    "image_width": 400,
    "image_height": 300,
    "image_url": "https://console.example/api/v1/releases/.../image?signature=...",
    "image_sha256": "...",
    "image_bytes": 15000
  }]
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
