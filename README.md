# ESP32 Glance Deck

An open-source reflective ESP32 display for Home Assistant status, alerts, and
personal data at a glance.

Built for the Waveshare ESP32-S3-RLCD-4.2, Glance Deck provides a Next.js +
LobeUI control plane for data sources, display documents, device management,
and OTA releases. Home Assistant consumes the control plane API for entities
and automations. The device connects to MQTT, displays a normalized document,
and reports its own state.

## Project layout

- [`firmware/`](firmware/): ESP-IDF firmware.
- [`home-assistant/`](home-assistant/): Home Assistant integration guidance.
- [`docs/architecture.md`](docs/architecture.md): responsibilities and layers.
- [`docs/mqtt-protocol.md`](docs/mqtt-protocol.md): MQTT contract.

## Development status

The repository currently contains the ESP-IDF scaffold and the v1 MQTT
contract. Wi-Fi provisioning, RLCD driver integration, UI rendering, MQTT
transport, and Home Assistant discovery are the next implementation stages.

## License

The license will be selected before the first functional release.
