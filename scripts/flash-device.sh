#!/usr/bin/env bash
set -euo pipefail

firmware_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${ESPFLASH_PORT:-/dev/cu.usbmodem21401}"

cd "$firmware_directory"

driver_fingerprint="$(shasum components/rlcd/CMakeLists.txt components/rlcd/rlcd.c components/rlcd/include/rlcd.h | shasum | cut -d ' ' -f 1)"
target_directory="${CARGO_TARGET_DIR:-/private/tmp/glance-deck-firmware-build-${driver_fingerprint}}"

docker run --rm \
  -v "$firmware_directory":/workspaces/esp32-glance-deck-firmware \
  -v "$target_directory":/tmp/build \
  -w /workspaces/esp32-glance-deck-firmware \
  -e CARGO_TARGET_DIR=/tmp/build \
  glance-deck-firmware-dev:latest \
  bash -lc '
    . /opt/esp/idf/export.sh
    . /home/esp/export-esp.sh
    export CARGO_TARGET_DIR=/tmp/build
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
