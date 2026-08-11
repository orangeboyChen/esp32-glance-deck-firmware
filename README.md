# ESP32 Glance Deck Firmware

Rust firmware for the Waveshare ESP32-S3-RLCD-4.2 Glance Deck device. The
device connects to the control plane over MQTT, renders verified display
documents, reports health, and applies signed OTA releases.

## Development

Use the provided ESP-IDF dev container, then run:

    idf.py build

Host-only unit tests:

    CARGO_TARGET_DIR=/private/tmp/glance-deck-host-target cargo test --lib --no-default-features

The device protocol is versioned in docs/mqtt-protocol.md.
