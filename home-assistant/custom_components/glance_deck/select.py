from __future__ import annotations

from homeassistant.components.select import SelectEntity
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
            async_add_entities(GlanceDeckPageSelect(coordinator, device_id) for device_id in new_ids)
            added.update(new_ids)
    add_new_entities()
    entry.async_on_unload(coordinator.async_add_listener(add_new_entities))


class GlanceDeckPageSelect(GlanceDeckEntity, SelectEntity):
    _attr_name = "Active page"

    def __init__(self, coordinator: GlanceDeckCoordinator, device_id: str) -> None:
        super().__init__(coordinator, device_id)
        self._attr_unique_id = f"{device_id}_active_page"

    @property
    def current_option(self) -> str | None:
        value = self.device.get("active_page_id")
        return value if isinstance(value, str) else None

    @property
    def options(self) -> list[str]:
        configuration = self.device.get("page_configuration", {})
        enabled_pages = configuration.get("enabled_page_ids", []) if isinstance(configuration, dict) else []
        options = [page_id for page_id in enabled_pages if isinstance(page_id, str)]
        return options or ([self.current_option] if self.current_option else [])

    async def async_select_option(self, option: str) -> None:
        await self.coordinator.api.async_command(self.device_id, "show_page", {"page_id": option})
        await self.coordinator.async_request_refresh()
