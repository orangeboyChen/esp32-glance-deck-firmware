from __future__ import annotations

from collections.abc import Mapping
from typing import Any

from homeassistant.helpers.device_registry import DeviceInfo
from homeassistant.helpers.update_coordinator import CoordinatorEntity

from .coordinator import GlanceDeckCoordinator
from .const import DOMAIN


class GlanceDeckEntity(CoordinatorEntity[GlanceDeckCoordinator]):
    """Common device-backed entity."""

    _attr_has_entity_name = True

    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str) -> None:
        super().__init__(coordinator)
        self.device_id = device_id
        self._attr_unique_id = f"{device_id}_{self.__class__.__name__.lower()}"

    @property
    def device(self) -> Mapping[str, Any]:
        return self.coordinator.device(self.device_id) or {}

    @property
    def device_info(self) -> DeviceInfo:
        return DeviceInfo(
            identifiers={(DOMAIN, self.device_id)},
            name=str(self.device.get("name", self.device_id)),
            manufacturer="Glance Deck",
            model=str(self.device.get("board_model", "ESP32-S3-RLCD-4.2")),
            sw_version=self.device.get("firmware_version"),
        )

    @property
    def available(self) -> bool:
        return self.coordinator.last_update_success and self.device.get("status") == "online"

    @property
    def extra_state_attributes(self) -> dict[str, Any]:
        return {
            "glance_deck_device_id": self.device_id,
            "status": self.device.get("status"),
            "preview_url": f"/api/glance_deck/preview/{self.coordinator.config_entry.entry_id}/{self.device_id}",
        }
