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

RUN curl --proto '=https' --tlsv1.2 -fsSL https://get.esp-rs.org | sh -s -- -y \
    && . /home/esp/export-esp.sh \
    && cargo install cargo-espflash --locked \
    && curl -fsSL https://bun.sh/install | bash

WORKDIR /workspaces/esp32-glance-deck
