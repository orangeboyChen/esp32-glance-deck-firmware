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

The repository contains the v1 control-plane, enrollment, display-release,
Home Assistant, and OTA protocol implementations. The firmware runs as a Rust
application in ESP-IDF and keeps the last verified frame available while the
network is offline. Battery support is defined around an external protected
power-path carrier; the firmware reports `unavailable` until that carrier's
actual gauge and charger pin map are selected.

## Local development

The console is managed with Bun:

```sh
bun install --cwd console
bun run --cwd console test:coverage
bun run --cwd console build
```

The Home Assistant integration is managed with uv:

```sh
uv sync --directory home-assistant
uv run --directory home-assistant pytest
```

For firmware work, open the repository in the provided dev container and run
`idf.py build` from `firmware/`. Host-only Rust tests can be run without ESP-IDF:

```sh
CARGO_TARGET_DIR=/private/tmp/glance-deck-host-target cargo test --manifest-path firmware/Cargo.toml --lib --no-default-features
```

Replace every development secret before exposing the console through a reverse
proxy.

When running the Compose stack with real hardware, set `DEVICE_ASSET_URL` to
the externally reachable HTTPS origin (for example the Traefik hostname). The
default Compose stack intentionally does not expose a device-safe asset URL:
an ESP32 cannot use the host's `localhost`, and firmware rejects plain HTTP.

## License

The license will be selected before the first functional release.
