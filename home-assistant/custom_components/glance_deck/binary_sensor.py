from __future__ import annotations

from homeassistant.components.binary_sensor import BinarySensorDeviceClass, BinarySensorEntity
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
            async_add_entities(GlanceDeckStatusBinarySensor(coordinator, device_id, kind) for device_id in new_ids for kind in ("online", "stale", "ota"))
            added.update(new_ids)
    add_new_entities()
    entry.async_on_unload(coordinator.async_add_listener(add_new_entities))


class GlanceDeckStatusBinarySensor(GlanceDeckEntity, BinarySensorEntity):
    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str, kind: str) -> None:
        super().__init__(coordinator, device_id)
        self.kind = kind
        self._attr_unique_id = f"{device_id}_{kind}"
        self._attr_name = {"online": "Online", "stale": "Display stale", "ota": "OTA in progress"}[kind]
        self._attr_device_class = BinarySensorDeviceClass.CONNECTIVITY if kind == "online" else None

    @property
    def is_on(self) -> bool:
        if self.kind == "online":
            return self.device.get("status") == "online"
        if self.kind == "stale":
            return bool(self.device.get("display", {}).get("stale", False))
        return self.device.get("ota_status") in {"queued", "downloading", "verifying", "rebooting"}
