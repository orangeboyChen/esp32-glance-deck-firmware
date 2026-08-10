# Rust firmware

The device firmware uses Rust on top of ESP-IDF through `esp-idf-sys`,
`esp-idf-hal`, and `esp-idf-svc`. This preserves Espressif Wi-Fi, NVS, MQTT,
TLS, partition, and OTA support while the application protocol is implemented
in Rust.

The Waveshare RLCD driver will be included as an ESP-IDF C component and
called through a narrow Rust adapter. This avoids rewriting an unverified
display driver before hardware bring-up.

## Commands

Run these commands inside the Dev Container. The checked-in Cargo target
configuration rebuilds the ESP-IDF `std` library from the `espup`-provided
Rust source, which is required for the Xtensa target:

```sh
cd firmware
cargo build --features esp
cargo run --features esp
```

`cargo run` uses `espflash flash --monitor`; connect the board to the host and
attach the USB device to the Dev Container first.
