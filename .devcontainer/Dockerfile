FROM espressif/idf:v5.3.1

ARG ESPUP_TOOLCHAIN_VERSION=1.85.0.0

USER root

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl pkg-config libudev-dev \
    && useradd --create-home --shell /bin/bash esp \
    && mkdir -p /workspaces/esp32-glance-deck \
    && chown -R esp:esp /workspaces \
    && rm -rf /var/lib/apt/lists/*

USER esp
ENV CARGO_HOME=/home/esp/.cargo
ENV PATH=/home/esp/.cargo/bin:${PATH}

RUN mkdir -p /home/esp/.cargo/bin

RUN case "$(uname -m)" in x86_64) echo x86_64-unknown-linux-gnu ;; aarch64) echo aarch64-unknown-linux-gnu ;; *) exit 1 ;; esac > /tmp/host-target

RUN curl --proto '=https' --tlsv1.2 --retry 5 --retry-all-errors -fsSL "https://static.rust-lang.org/rustup/dist/$(cat /tmp/host-target)/rustup-init" -o /tmp/rustup-init

RUN chmod +x /tmp/rustup-init

RUN for attempt in 1 2 3 4 5; do /tmp/rustup-init -y --profile minimal && break; test "$attempt" = 5 && exit 1; sleep "$attempt"; done

RUN curl --proto '=https' --tlsv1.2 --retry 5 --retry-all-errors -fsSL "https://github.com/esp-rs/espup/releases/latest/download/espup-$(cat /tmp/host-target)" -o /home/esp/.cargo/bin/espup

RUN chmod +x /home/esp/.cargo/bin/espup

RUN espup install --targets esp32s3 --toolchain-version ${ESPUP_TOOLCHAIN_VERSION}

ENV RUSTUP_TOOLCHAIN=esp

RUN bash -lc '. /home/esp/export-esp.sh && cargo install ldproxy --locked'

RUN cargo install cargo-espflash --version 4.3.0 --locked

RUN curl -fsSL https://bun.sh/install | bash

WORKDIR /workspaces/esp32-glance-deck-firmware
