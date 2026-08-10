from __future__ import annotations

from homeassistant.components.update import UpdateEntity, UpdateEntityFeature
from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback

from .coordinator import GlanceDeckCoordinator
from .entity import GlanceDeckEntity


async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry, async_add_entities: AddEntitiesCallback) -> None:
    coordinator: GlanceDeckCoordinator = entry.runtime_data
    added: set[str] = set()
    def add_new_entities() -> None:
        new_ids = set(coordinator.data) - added
        if new_ids:
            async_add_entities(GlanceDeckUpdate(coordinator, device_id) for device_id in new_ids)
            added.update(new_ids)
    add_new_entities()
    entry.async_on_unload(coordinator.async_add_listener(add_new_entities))


class GlanceDeckUpdate(GlanceDeckEntity, UpdateEntity):
    _attr_supported_features = UpdateEntityFeature.INSTALL
    _attr_name = "Firmware"

    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str) -> None:
        super().__init__(coordinator, device_id)
        self._attr_unique_id = f"{device_id}_firmware_update"

    @property
    def installed_version(self) -> str | None:
        return self.device.get("firmware_version")

    @property
    def latest_version(self) -> str | None:
        return self.device.get("available_firmware_version")

    @property
    def update_available(self) -> bool:
        return bool(self.latest_version and self.latest_version != self.installed_version)

    async def async_install(self, version: str | None, backup: bool) -> None:
        await self.coordinator.api.async_start_ota(self.device_id)
        await self.coordinator.async_request_refresh()
