# ESP32 Glance Deck

An open-source reflective ESP32 display for Home Assistant status, alerts, and
personal data at a glance.

Built for the Waveshare ESP32-S3-RLCD-4.2, Glance Deck provides a Next.js +
LobeUI control plane for data sources, display documents, device management,
and OTA releases. Home Assistant consumes the control plane API for entities
and automations. The device connects to MQTT, displays a normalized document,
and reports its own state.

## Project layout

- [`firmware/`](firmware/): Rust firmware built on ESP-IDF.
- [`console/`](console/): Bun-managed Next.js + LobeUI control plane.
- [`.devcontainer/`](.devcontainer/): reproducible ESP-IDF/Rust firmware environment.
- [`home-assistant/`](home-assistant/): Home Assistant integration guidance.
- [`docs/architecture.md`](docs/architecture.md): responsibilities and layers.
- [`docs/mqtt-protocol.md`](docs/mqtt-protocol.md): MQTT contract.
- [`docs/traefik-mqtt.md`](docs/traefik-mqtt.md): production MQTT-over-TLS/WSS edge setup.

## Development status

The repository contains a Rust-on-ESP-IDF firmware scaffold, Bun-managed
Next.js control-plane scaffold, Docker Compose development services, and the
v1 MQTT contract. Wi-Fi provisioning, RLCD driver integration, device MQTT,
Home Assistant discovery, and the authenticated control plane are next.

## License

The license will be selected before the first functional release.
