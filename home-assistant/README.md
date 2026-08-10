# Home Assistant integration

The integration is an HTTP API client for the Glance Deck control plane. It
does not connect to a device address or MQTT broker.

Run its test suite with `uv` and Python 3.13:

```sh
uv sync
uv run pytest
```

The test command enforces at least 90% line coverage for the integration.
