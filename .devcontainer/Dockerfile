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

RUN curl --proto '=https' --tlsv1.2 -fsSL https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-unknown-linux-gnu -o /home/esp/.cargo/bin/espup \
    && chmod +x /home/esp/.cargo/bin/espup \
    && espup install \
    && . /home/esp/export-esp.sh \
    && cargo install cargo-espflash --locked \
    && curl -fsSL https://bun.sh/install | bash

WORKDIR /workspaces/esp32-glance-deck
