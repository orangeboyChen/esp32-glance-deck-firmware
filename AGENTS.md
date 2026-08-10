# ESP32 Glance Deck contributor guide

## Product boundary

ESP32 Glance Deck is a Wi-Fi connected reflective display for status, alerts,
and personal data. The Next.js control plane owns data collection, device
management, display documents, and firmware releases. Home Assistant consumes
the control plane API for entities and automation. The device renders display
documents and reports its own health; it never stores third-party service
credentials.

## Repository layout

- `firmware/`: ESP-IDF application for the device.
- `console/`: Next.js and LobeUI full-stack control plane.
- `home-assistant/`: MQTT discovery, dashboard, and automation examples.
- `docs/`: protocol and hardware documentation.

## Git workflow

- Use English Conventional Commit messages for every commit, for example
  `feat(console): add device preview API` or
  `chore(devcontainer): add ESP-Rust toolchain`.
- Commit and push each verified, coherent implementation milestone promptly.
- Do not include the local-only `plan.md` in commits unless the user explicitly
  asks for it.
- Always use elevated permission for `gh` commands.

## Firmware conventions

- Target ESP-IDF 5.3 or newer and C17.
- Keep hardware-specific code in `components/display`; application code must
  not depend on the Waveshare driver's internal APIs.
- Use MQTT for device state, display documents, commands, and OTA jobs.
  Devices initiate all network connections; do not add an HTTP server for HA
  control.
- Store only Wi-Fi and MQTT connection configuration in NVS. Do not persist
  Home Assistant tokens, third-party API keys, cookies, or personal message
  history.
- Treat MQTT payloads as untrusted input. Validate document versions, limits,
  and JSON field types before changing the UI.
- OTA must validate an HTTPS source, signed firmware manifest, image hash, and
  ESP-IDF app signature before applying an update. Preserve a known-good OTA
  partition for automatic rollback.
- Use `snake_case` for C identifiers and `UPPER_SNAKE_CASE` for macros.

## Validation

After changing firmware, run `idf.py build` from `firmware/` in an ESP-IDF
shell. Do not claim a hardware flash test unless it was actually performed.
