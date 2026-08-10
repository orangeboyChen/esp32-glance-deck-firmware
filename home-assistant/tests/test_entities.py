from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import AsyncMock

import pytest

from custom_components.glance_deck.binary_sensor import GlanceDeckStatusBinarySensor
from custom_components.glance_deck.button import GlanceDeckButton
from custom_components.glance_deck.entity import GlanceDeckEntity
from custom_components.glance_deck.select import GlanceDeckPageSelect
from custom_components.glance_deck.sensor import GlanceDeckSensor
from custom_components.glance_deck.update import GlanceDeckUpdate


class FakeCoordinator:
    def __init__(self) -> None:
        self.data = {
            "deck-a": {
                "id": "deck-a", "name": "Desk", "status": "online", "firmware_version": "1.0.0",
                "wifi_rssi": -57, "active_page_id": "usage", "last_seen_at": "2026-08-10T12:00:00Z",
                "available_firmware_version": "1.1.0", "ota_status": "downloading",
                "display": {"stale": True, "release_version": 4, "pages": [{"id": "usage"}, {"id": "alerts"}], "source": {"values": {"used": 72, "total": 100, "resets_at": "2026-08-11T00:00:00Z"}}},
            }
        }
        self.last_update_success = True
        self.config_entry = SimpleNamespace(entry_id="entry-a")
        self.api = SimpleNamespace(async_command=AsyncMock(), async_start_ota=AsyncMock())
        self.async_request_refresh = AsyncMock()

    def async_add_listener(self, _listener):
        return lambda: None

    def device(self, device_id: str):
        return self.data.get(device_id)


def test_entity_metadata_and_status_properties() -> None:
    coordinator = FakeCoordinator()
    entity = GlanceDeckEntity(coordinator, "deck-a")
    assert entity.available is True
    assert entity.device_info["name"] == "Desk"
    assert entity.extra_state_attributes["preview_url"].endswith("/entry-a/deck-a")
    assert GlanceDeckStatusBinarySensor(coordinator, "deck-a", "online").is_on is True
    assert GlanceDeckStatusBinarySensor(coordinator, "deck-a", "stale").is_on is True
    assert GlanceDeckStatusBinarySensor(coordinator, "deck-a", "ota").is_on is True
    coordinator.data["deck-a"]["status"] = "offline"
    assert entity.available is False


def test_sensor_values_and_page_selection() -> None:
    coordinator = FakeCoordinator()
    assert GlanceDeckSensor(coordinator, "deck-a", "wifi_rssi", None).native_value == -57
    assert GlanceDeckSensor(coordinator, "deck-a", "last_seen_at", None).native_value.year == 2026
    assert GlanceDeckSensor(coordinator, "deck-a", "release_version", None).native_value == 4
    assert GlanceDeckSensor(coordinator, "deck-a", "usage_percentage", None).native_value == 72.0
    assert GlanceDeckSensor(coordinator, "deck-a", "reset_time", None).native_value.year == 2026
    page_select = GlanceDeckPageSelect(coordinator, "deck-a")
    assert page_select.current_option == "usage"
    assert page_select.options == ["usage", "alerts"]


@pytest.mark.asyncio
async def test_entities_send_commands_and_start_updates() -> None:
    coordinator = FakeCoordinator()
    await GlanceDeckButton(coordinator, "deck-a", "next_page", "next_page").async_press()
    coordinator.api.async_command.assert_awaited_once_with("deck-a", "next_page")
    page_select = GlanceDeckPageSelect(coordinator, "deck-a")
    await page_select.async_select_option("alerts")
    coordinator.api.async_command.assert_awaited_with("deck-a", "show_page", {"page_id": "alerts"})
    update = GlanceDeckUpdate(coordinator, "deck-a")
    assert update.installed_version == "1.0.0"
    assert update.latest_version == "1.1.0"
    assert update.update_available is True
    await update.async_install(None, False)
    coordinator.api.async_start_ota.assert_awaited_once_with("deck-a")
