from __future__ import annotations

from homeassistant.components.button import ButtonEntity
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
            async_add_entities(GlanceDeckButton(coordinator, device_id, key, action) for device_id in new_ids for key, action in (("next_page", "next_page"), ("refresh_display", "refresh_release"), ("check_update", None)))
            added.update(new_ids)
    add_new_entities()
    entry.async_on_unload(coordinator.async_add_listener(add_new_entities))


class GlanceDeckButton(GlanceDeckEntity, ButtonEntity):
    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str, key: str, action: str | None) -> None:
        super().__init__(coordinator, device_id)
        self.action = action
        self._attr_unique_id = f"{device_id}_{key}"
        self._attr_name = key.replace("_", " ").title()

    async def async_press(self) -> None:
        if self.action is not None:
            await self.coordinator.api.async_command(self.device_id, self.action)
        await self.coordinator.async_request_refresh()
