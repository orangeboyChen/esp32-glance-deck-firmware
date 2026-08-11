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

RUN mkdir -p /home/esp/.cargo/bin \
    && case "$(uname -m)" in \
        x86_64) host_target=x86_64-unknown-linux-gnu ;; \
        aarch64) host_target=aarch64-unknown-linux-gnu ;; \
        *) echo "unsupported container architecture: $(uname -m)" >&2; exit 1 ;; \
      esac \
    && curl --proto '=https' --tlsv1.2 --retry 5 --retry-all-errors -fsSL "https://static.rust-lang.org/rustup/dist/${host_target}/rustup-init" -o /tmp/rustup-init \
    && chmod +x /tmp/rustup-init \
    && for attempt in 1 2 3 4 5; do /tmp/rustup-init -y --profile minimal && break; test "$attempt" = 5 && exit 1; sleep "$attempt"; done \
    && curl --proto '=https' --tlsv1.2 --retry 5 --retry-all-errors -fsSL "https://github.com/esp-rs/espup/releases/latest/download/espup-${host_target}" -o /home/esp/.cargo/bin/espup \
    && chmod +x /home/esp/.cargo/bin/espup \
    && espup install --targets esp32s3 --toolchain-version ${ESPUP_TOOLCHAIN_VERSION} \
    && . /home/esp/export-esp.sh \
    && cargo install ldproxy --locked \
    && cargo install cargo-espflash --locked \
    && curl -fsSL https://bun.sh/install | bash

WORKDIR /workspaces/esp32-glance-deck
