FROM espressif/idf:v5.3.1

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
    && curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal \
    && case "$(uname -m)" in \
        x86_64) espup_target=x86_64-unknown-linux-gnu ;; \
        aarch64) espup_target=aarch64-unknown-linux-gnu ;; \
        *) echo "unsupported container architecture: $(uname -m)" >&2; exit 1 ;; \
      esac \
    && curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/esp-rs/espup/releases/latest/download/espup-${espup_target}" -o /home/esp/.cargo/bin/espup \
    && chmod +x /home/esp/.cargo/bin/espup \
    && espup install --targets esp32s3 \
    && . /home/esp/export-esp.sh \
    && cargo install cargo-espflash --locked \
    && curl -fsSL https://bun.sh/install | bash

WORKDIR /workspaces/esp32-glance-deck
