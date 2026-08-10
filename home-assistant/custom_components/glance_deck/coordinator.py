from __future__ import annotations

from datetime import timedelta
import logging
from typing import Any

from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.update_coordinator import DataUpdateCoordinator, UpdateFailed

from .api import GlanceDeckApiClient, GlanceDeckApiError
from .const import COORDINATOR_UPDATE_SECONDS, DOMAIN


class GlanceDeckCoordinator(DataUpdateCoordinator[dict[str, dict[str, Any]]]):
    """Fetch device state from the control plane."""

    def __init__(self, hass: HomeAssistant, entry: ConfigEntry, api: GlanceDeckApiClient) -> None:
        super().__init__(
            hass,
            logger=logging.getLogger(__name__),
            name=f"{DOMAIN}_{entry.entry_id}",
            update_interval=timedelta(seconds=COORDINATOR_UPDATE_SECONDS),
        )
        self.api = api

    async def _async_update_data(self) -> dict[str, dict[str, Any]]:
        try:
            devices = await self.api.async_get_devices()
            result: dict[str, dict[str, Any]] = {}
            for device in devices:
                device_id = device.get("id")
                if not isinstance(device_id, str):
                    continue
                display = await self.api.async_get_display(device_id)
                result[device_id] = {**device, "display": display}
            return result
        except GlanceDeckApiError as error:
            raise UpdateFailed(str(error)) from error

    def device(self, device_id: str) -> dict[str, Any] | None:
        return self.data.get(device_id) if self.data else None
