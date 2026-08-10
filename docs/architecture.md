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
| Third-party API credentials | Home Assistant only |

## Firmware layers

1. `main`: boot sequencing and application lifecycle.
2. `components/connectivity` (planned): Wi-Fi provisioning, NVS, MQTT.
3. `components/protocol` (planned): bounded JSON parsing and MQTT topics.
4. `components/display`: RLCD/LVGL adapter and renderer.
5. `components/ui` (planned): pages, widgets, and interaction state.

The display adapter is intentionally isolated because the factory sample's
driver and pin setup will be brought in after the board is available for a
hardware smoke test.
