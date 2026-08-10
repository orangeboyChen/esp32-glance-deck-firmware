# Dev Container firmware workflow

Open this repository in VS Code and run **Dev Containers: Reopen in
Container**. The image provides ESP-IDF 5.3.1, the Espressif Rust toolchain,
and `espflash`.

For USB flashing on Linux, attach the board when opening the container and add
the appropriate serial device (typically `/dev/ttyUSB0` or `/dev/ttyACM0`) to
the Dev Container run arguments. Docker Desktop on macOS does not expose host
USB serial devices to Linux containers reliably; build in the container, then
run `cargo run` or `espflash` from an ESP-Rust environment on the host to
flash/monitor.

The first build downloads Rust and ESP-IDF crates into the container cache.
Run `cargo build` before connecting hardware, then `cargo run` to flash and
open the serial monitor.
