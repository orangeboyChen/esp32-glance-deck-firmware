# Home Assistant integration

The integration is an HTTP API client for the Glance Deck control plane. It
does not connect to a device address or MQTT broker.

Run its test suite with Python 3.13:

```sh
python -m pip install -r requirements-test.txt
python -m pytest
```

The test command enforces at least 90% line coverage for the integration.
