# MQTT protocol

Each device has an immutable `device_id`, for example `glance-deck-office`.
All topics use `glance_deck/<device_id>/`.

| Topic suffix | Direction | Retained | Purpose |
| --- | --- | --- | --- |
| `release` | Control plane to device | Yes | Current immutable display page directory |
| `command` | Control plane to device | No | Immediate action requested by console or HA |
| `state` | Device to control plane | Yes | Connectivity and UI state |
| `availability` | Device to control plane | Yes | `online` or `offline` |
| `ota` | Control plane to device | No | Signed remote OTA job |
| `ota/state` | Device to control plane | Yes | OTA progress and result |
| `ota/check` | Device to control plane | No | Local maintenance update check request |
| `ota/check/state` | Control plane to device | No | Signed candidate metadata or check result |

## Display release

The control plane is the only renderer. It rasterizes all text, including CJK,
with its bundled font and publishes immutable 1-bit MSB-first page metadata.
The ESP32 must not parse SVG or depend on a font catalog. It must reject unknown
document versions, image formats, dimensions, hashes, or byte counts.

The retained payload is a page directory, not a bulk-transfer request. On a
new release the device downloads only `active_page_id`; on an explicit page
change it flushes a verified cached page immediately or downloads only that
target page. If offline, it cycles only cached pages and keeps the current
verified frame when the requested target is unavailable.

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
    "image_url": "https://console.example/api/v1/releases/.../pages/usage/image?signature=...",
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
`set_rotation`, `refresh_release`, and `enter_maintenance`.

## Device state

```json
{"version":1,"page_id":"usage","wifi_rssi":-56,"display_updated_at":"2026-08-09T18:00:00+08:00","power":{"source":"usb_and_battery","charging":true,"battery_percent":82,"battery_mv":3975}}
```

`power.source` is one of `usb`, `battery`, `usb_and_battery`, or
`unavailable`. Battery percentage and millivolts are optional: the device
omits them when its installed power hardware cannot measure them. The control
plane persists the last power report and timestamp; it must not derive a
percentage from Wi-Fi state or assume that USB implies charging.

## Local OTA check

The maintenance page publishes `{"version":1}` to `ota/check`. The control
plane compares the latest verified stable release for the device board and
returns `ota/check/state`. An `available` response contains only HTTPS manifest
metadata and the image hash; the device shows the candidate and does not write
an OTA partition until the user confirms locally. The same signature, hash,
power, rollback, and health checks used by remote OTA then apply.
