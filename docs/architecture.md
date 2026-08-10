# Architecture

ESP32 Glance Deck presents information on a Waveshare ESP32-S3-RLCD-4.2. The
Next.js + LobeUI control plane is the system of record for source collection,
display documents, device management, and OTA releases. Home Assistant is an
API client of that control plane.

```text
Data sources --> Control plane --> MQTT broker <-- Glance Deck
                    |                  |
                    |                  +-- device state and OTA progress
                    v
            Home Assistant API integration
```

## Ownership

| Concern | Owner |
| --- | --- |
| Subscription/API data collection and calculations | Control plane |
| Display documents, device configuration, OTA releases | Control plane |
| Alerts, automation, and history | Control plane and/or Home Assistant |
| HA entities and automations sourced from control-plane API | Home Assistant |
| Wi-Fi/MQTT reconnect and cached current display | Device |
| Reflective screen rendering and local button navigation | Device |
| Third-party API credentials | Encrypted control-plane storage only |

## Firmware layers

1. `main`: boot sequencing and application lifecycle.
2. `provisioning_esp`, `esp_config`, and `esp_mqtt`: Wi-Fi provisioning, NVS,
   reconnect, and MQTT transport.
3. `mqtt`, `release_sync`, and `page_controller`: bounded protocol parsing,
   demand-based page downloads, cache commits, and state reporting.
4. `components/display`: RLCD/LVGL adapter and renderer.
5. `runtime`, `buttons`, and `local_screen`: local interaction and maintenance
   state.

The display adapter is intentionally isolated because the factory sample's
driver and pin setup will be brought in after the board is available for a
hardware smoke test.

## Usage-source imports

`POST /api/v1/sources/cc-switch/preview` accepts a CC Switch UsageScript
export only as a review aid. It validates the request URL and method, redacts
credential-bearing fields, and extracts returned field names from extractor
text without evaluating JavaScript. The administrator must provide the final
safe JSONPath mapping and encrypted credential values through the normal source
API; the worker never executes imported JavaScript.
