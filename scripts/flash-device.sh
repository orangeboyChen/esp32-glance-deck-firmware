#!/usr/bin/env bash
set -euo pipefail

firmware_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_directory="${CARGO_TARGET_DIR:-/private/tmp/glance-deck-firmware-build}"
port="${ESPFLASH_PORT:-/dev/cu.usbmodem21401}"

cd "$firmware_directory"

docker run --rm \
  -v "$firmware_directory":/workspaces/esp32-glance-deck-firmware \
  -v "$target_directory":/tmp/build \
  -w /workspaces/esp32-glance-deck-firmware \
  -e CARGO_TARGET_DIR=/tmp/build \
  glance-deck-firmware-dev:latest \
  bash -lc '
    . /opt/esp/idf/export.sh
    . /home/esp/export-esp.sh
    PATH="$(printf "%s" "$PATH" | tr ":" "\n" | grep -v "/home/esp/.rustup/toolchains/esp/xtensa-esp-elf" | paste -sd: -)"
    export PATH="/opt/esp/tools/xtensa-esp-elf/esp-13.2.0_20240530/xtensa-esp-elf/bin:/home/esp/.cargo/bin:$PATH"
    cargo build --release --features esp
  '

espflash flash \
  -p "$port" \
  -B 460800 \
  -c esp32s3 \
  -s 16mb \
  -f 80mhz \
  -m dio \
  -T partitions.csv \
  --target-app-partition factory \
  --bootloader "$target_directory/xtensa-esp32s3-espidf/release/bootloader.bin" \
  "$target_directory/xtensa-esp32s3-espidf/release/glance-deck-firmware"
