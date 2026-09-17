# ESP32 Glance Deck Firmware

Rust firmware for the Waveshare ESP32-S3-RLCD-4.2 Glance Deck device. The
device connects to the control plane over MQTT, renders verified display
documents, reports health, and applies signed OTA releases.

## Development

Use the provided ESP-IDF dev container (`espressif/idf:v5.3.1`), then build the
device image with Cargo rather than `idf.py`, because this is a Rust crate that
links ESP-IDF through `esp-idf-sys`:

    . ${IDF_PATH}/export.sh
    cargo build --release --features esp --target xtensa-esp32s3-espidf

Host-only unit tests, which need no ESP-IDF and cover the protocol, cache, and
input-handling logic:

    cargo test --lib --no-default-features

## OTA signing key

Every OTA manifest must carry an Ed25519 signature the device verifies before
writing an image. The public key is baked in at compile time, so it has to be
present when the device image is built:

    FIRMWARE_MANIFEST_PUBLIC_KEY_HEX=<64 hex characters> cargo build --release \
      --features esp --target xtensa-esp32s3-espidf

If the variable is unset the build warns and produces an image that rejects
every OTA job with `firmware_public_key_missing`; if it is set to something
that is not 64 hex characters the build fails. Generate a key pair with
`openssl genpkey -algorithm ed25519` or with the signing tool in the control
plane, and keep the private key off the device.

The device protocol is versioned in docs/mqtt-protocol.md.
