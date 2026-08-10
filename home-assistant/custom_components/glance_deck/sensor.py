from __future__ import annotations

from datetime import datetime
from typing import Any

from homeassistant.components.sensor import SensorDeviceClass, SensorEntity
from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.entity_platform import AddEntitiesCallback

from .coordinator import GlanceDeckCoordinator
from .entity import GlanceDeckEntity


SENSORS: tuple[tuple[str, str | None], ...] = (
    ("current_page", None),
    ("firmware_version", None),
    ("wifi_rssi", SensorDeviceClass.SIGNAL_STRENGTH),
    ("last_seen_at", SensorDeviceClass.TIMESTAMP),
    ("release_version", None),
    ("usage_percentage", None),
    ("reset_time", SensorDeviceClass.TIMESTAMP),
    ("battery_percent", SensorDeviceClass.BATTERY),
    ("battery_mv", SensorDeviceClass.VOLTAGE),
    ("power_updated_at", SensorDeviceClass.TIMESTAMP),
)


async def async_setup_entry(hass: HomeAssistant, entry: ConfigEntry, async_add_entities: AddEntitiesCallback) -> None:
    coordinator: GlanceDeckCoordinator = entry.runtime_data
    added: set[str] = set()
    def add_new_entities() -> None:
        new_ids = set(coordinator.data) - added
        if new_ids:
            async_add_entities(GlanceDeckSensor(coordinator, device_id, key, device_class) for device_id in new_ids for key, device_class in SENSORS)
            added.update(new_ids)
    add_new_entities()
    entry.async_on_unload(coordinator.async_add_listener(add_new_entities))


class GlanceDeckSensor(GlanceDeckEntity, SensorEntity):
    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str, key: str, device_class: SensorDeviceClass | None) -> None:
        super().__init__(coordinator, device_id)
        self.key = key
        self._attr_unique_id = f"{device_id}_{key}"
        self._attr_name = key.replace("_", " ").title()
        self._attr_device_class = device_class
        if key == "wifi_rssi":
            self._attr_native_unit_of_measurement = "dBm"
        if key == "battery_percent":
            self._attr_native_unit_of_measurement = "%"
        if key == "battery_mv":
            self._attr_native_unit_of_measurement = "mV"

    @property
    def native_value(self) -> Any:
        if self.key == "release_version":
            return self.device.get("display", {}).get("release_version") or self.device.get("release_version")
        source = self.device.get("display", {}).get("source", {})
        values = source.get("values", {}) if isinstance(source, dict) else {}
        if self.key == "usage_percentage":
            used, total = values.get("used"), values.get("total")
            if isinstance(used, (int, float)) and isinstance(total, (int, float)) and total > 0:
                return round(used / total * 100, 1)
            return None
        if self.key == "reset_time":
            value = values.get("resets_at")
            return datetime.fromisoformat(value.replace("Z", "+00:00")) if isinstance(value, str) else None
        value = self.device.get(self.key)
        if self.key in ("last_seen_at", "power_updated_at") and isinstance(value, str):
            return datetime.fromisoformat(value.replace("Z", "+00:00"))
        return value

    @property
    def extra_state_attributes(self) -> dict[str, Any]:
        attributes = dict(super().extra_state_attributes)
        if self.key in ("battery_percent", "battery_mv", "power_updated_at"):
            attributes.update({
                "power_source": self.device.get("power_source", "unavailable"),
                "charging": self.device.get("charging"),
            })
        return attributes
