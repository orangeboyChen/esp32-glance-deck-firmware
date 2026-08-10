# Home Assistant integration

`custom_components/glance_deck` is a Home Assistant Custom Integration. It
calls the Glance Deck control-plane REST API using a scoped HA API token. It
never connects to device IP addresses or MQTT; commands are sent to the
control plane, which owns MQTT and confirms device delivery.

## Install and configure

1. Copy `custom_components/glance_deck` into Home Assistant's
   `config/custom_components/` directory and restart Home Assistant.
2. In **Settings → Devices & services → Add integration**, select **Glance
   Deck** and enter the HTTPS control-plane URL plus a token with
   `devices:read`, `devices:command`, and (for OTA) `ota:install` scopes.
3. The integration discovers each backend device and creates availability,
   stale-display, OTA, current-page, release, RSSI, firmware, page select,
   command buttons, and update entities.

The coordinator polls the backend every 30 seconds. Device and preview state
are therefore a backend view; HA never receives broker credentials.

## Services

- `glance_deck.show_page`: `device_id`, `page_id`
- `glance_deck.refresh_device`: `device_id`
- `glance_deck.start_ota`: `device_id`

All services return only after the control plane accepts the command. Device
confirmation appears on the next coordinator update; a queued command is not
treated as a completed physical action.

## Same-canvas Lovelace preview

The backend's `/preview` endpoint is proxied through an authenticated HA view,
so the API token never appears in browser markup. Copy
`lovelace/glance-deck-preview-card.js` into `config/www/`, add it as a module
resource, then configure the card using a Glance Deck `current_page` sensor:

```yaml
type: custom:glance-deck-preview-card
entity: sensor.glance_deck_office_current_page
```

The card shows the exact immutable 300 × 400 preview assigned by the control
plane—the same asset delivered to the physical device. It does not recreate
the display layout in Lovelace.
